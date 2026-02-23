use crate::errors::Nc2ParquetError;
use crate::filters::{FilterResult, NCFilter};
use polars::prelude::*;
use std::collections::{HashMap, HashSet};

/// Manages dimension indices and coordinate combinations during filtering operations.
#[derive(Debug, Clone)]
pub(crate) struct DimensionIndexManager {
    dimension_indices: HashMap<String, HashSet<usize>>,
    dimension_order: Vec<String>,
    explicit_combinations: Option<Vec<Vec<usize>>>,
}

impl DimensionIndexManager {
    pub(crate) fn new(var: &netcdf::Variable) -> Result<Self, Nc2ParquetError> {
        let mut dimension_indices = HashMap::new();
        let mut dimension_order = Vec::new();

        for dim in var.dimensions() {
            let dim_name = dim.name().to_string();
            let dim_size = dim.len();

            let indices: HashSet<usize> = (0..dim_size).collect();
            dimension_indices.insert(dim_name.clone(), indices);
            dimension_order.push(dim_name);
        }

        Ok(DimensionIndexManager {
            dimension_indices,
            dimension_order,
            explicit_combinations: None,
        })
    }

    pub(crate) fn apply_filter_result(
        &mut self,
        result: &FilterResult,
    ) -> Result<(), Nc2ParquetError> {
        match result {
            FilterResult::Single { dimension, indices } => {
                if let Some(current_indices) = self.dimension_indices.get_mut(dimension) {
                    let new_indices: HashSet<usize> = indices.iter().cloned().collect();
                    *current_indices = current_indices
                        .intersection(&new_indices)
                        .cloned()
                        .collect();
                } else {
                    return Err(Nc2ParquetError::Extraction(format!(
                        "Unknown dimension: {}",
                        dimension
                    )));
                }
            }

            FilterResult::Pairs {
                lat_dimension,
                lon_dimension,
                pairs,
            } => {
                self.apply_explicit_pairs(lat_dimension, lon_dimension, pairs)?;
            }

            FilterResult::Triplets {
                time_dimension,
                lat_dimension,
                lon_dimension,
                triplets,
            } => {
                self.apply_explicit_triplets(
                    time_dimension,
                    lat_dimension,
                    lon_dimension,
                    triplets,
                )?;
            }
        }
        Ok(())
    }

    fn apply_explicit_pairs(
        &mut self,
        lat_dim: &str,
        lon_dim: &str,
        pairs: &[(usize, usize)],
    ) -> Result<(), Nc2ParquetError> {
        let lat_pos = self
            .dimension_order
            .iter()
            .position(|d| d == lat_dim)
            .ok_or_else(|| Nc2ParquetError::DimensionNotFound(lat_dim.to_string()))?;
        let lon_pos = self
            .dimension_order
            .iter()
            .position(|d| d == lon_dim)
            .ok_or_else(|| Nc2ParquetError::DimensionNotFound(lon_dim.to_string()))?;

        let mut combinations = Vec::new();

        let other_dimensions: Vec<(usize, Vec<usize>)> = self
            .dimension_order
            .iter()
            .enumerate()
            .filter(|(pos, _)| *pos != lat_pos && *pos != lon_pos)
            .map(|(pos, dim_name)| {
                let indices: Vec<usize> =
                    self.dimension_indices[dim_name].iter().cloned().collect();
                (pos, indices)
            })
            .collect();

        self.generate_combinations_with_pairs(
            &other_dimensions,
            pairs,
            lat_pos,
            lon_pos,
            &mut Vec::new(),
            0,
            &mut combinations,
        );

        self.explicit_combinations = Some(combinations);
        Ok(())
    }

    fn apply_explicit_triplets(
        &mut self,
        time_dim: &str,
        lat_dim: &str,
        lon_dim: &str,
        triplets: &[(usize, usize, usize)],
    ) -> Result<(), Nc2ParquetError> {
        let time_pos = self
            .dimension_order
            .iter()
            .position(|d| d == time_dim)
            .ok_or_else(|| Nc2ParquetError::DimensionNotFound(time_dim.to_string()))?;
        let lat_pos = self
            .dimension_order
            .iter()
            .position(|d| d == lat_dim)
            .ok_or_else(|| Nc2ParquetError::DimensionNotFound(lat_dim.to_string()))?;
        let lon_pos = self
            .dimension_order
            .iter()
            .position(|d| d == lon_dim)
            .ok_or_else(|| Nc2ParquetError::DimensionNotFound(lon_dim.to_string()))?;

        let mut combinations = Vec::new();
        for &(time_idx, lat_idx, lon_idx) in triplets {
            let mut coord = vec![0; self.dimension_order.len()];
            coord[time_pos] = time_idx;
            coord[lat_pos] = lat_idx;
            coord[lon_pos] = lon_idx;
            combinations.push(coord);
        }

        self.explicit_combinations = Some(combinations);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)] // Reason: recursive combinator requires all context parameters; decomposing would obscure the algorithm
    fn generate_combinations_with_pairs(
        &self,
        other_dims: &[(usize, Vec<usize>)],
        pairs: &[(usize, usize)],
        lat_pos: usize,
        lon_pos: usize,
        current: &mut Vec<usize>,
        other_dim_idx: usize,
        results: &mut Vec<Vec<usize>>,
    ) {
        if other_dim_idx >= other_dims.len() {
            for &(lat_idx, lon_idx) in pairs {
                let mut coord = vec![0; self.dimension_order.len()];
                for (i, &val) in current.iter().enumerate() {
                    coord[other_dims[i].0] = val;
                }
                coord[lat_pos] = lat_idx;
                coord[lon_pos] = lon_idx;
                results.push(coord);
            }
            return;
        }

        let (_, ref indices) = other_dims[other_dim_idx];
        for &idx in indices {
            current.push(idx);
            self.generate_combinations_with_pairs(
                other_dims,
                pairs,
                lat_pos,
                lon_pos,
                current,
                other_dim_idx + 1,
                results,
            );
            current.pop();
        }
    }

    #[allow(dead_code)] // Used in #[cfg(test)] modules
    pub(crate) fn get_dimension_indices(&self, dim_name: &str) -> Option<&HashSet<usize>> {
        self.dimension_indices.get(dim_name)
    }

    pub(crate) fn get_dimension_order(&self) -> &Vec<String> {
        &self.dimension_order
    }

    pub(crate) fn get_all_coordinate_combinations(&self) -> Vec<Vec<usize>> {
        if let Some(ref explicit) = self.explicit_combinations {
            explicit.clone()
        } else {
            let mut result = Vec::new();
            self.generate_combinations(&mut Vec::new(), 0, &mut result);
            result
        }
    }

    fn generate_combinations(
        &self,
        current: &mut Vec<usize>,
        dim_index: usize,
        result: &mut Vec<Vec<usize>>,
    ) {
        if dim_index >= self.dimension_order.len() {
            result.push(current.clone());
            return;
        }

        let dim_name = &self.dimension_order[dim_index];
        if let Some(indices) = self.dimension_indices.get(dim_name) {
            let mut sorted_indices: Vec<usize> = indices.iter().cloned().collect();
            sorted_indices.sort();

            for &idx in &sorted_indices {
                current.push(idx);
                self.generate_combinations(current, dim_index + 1, result);
                current.pop();
            }
        }
    }
}

pub(crate) fn extract_data_to_dataframe(
    file: &netcdf::File,
    var: &netcdf::Variable,
    var_name: &str,
    filters: &Vec<Box<dyn NCFilter>>,
) -> Result<DataFrame, Nc2ParquetError> {
    let mut dim_manager = DimensionIndexManager::new(var)?;
    for filter in filters.iter() {
        let result = filter.apply(file)?;
        dim_manager.apply_filter_result(&result)?;
    }
    extract_data_with_dimension_manager(file, var, var_name, &dim_manager)
}

fn extract_data_with_dimension_manager(
    file: &netcdf::File,
    var: &netcdf::Variable,
    var_name: &str,
    dim_manager: &DimensionIndexManager,
) -> Result<DataFrame, Nc2ParquetError> {
    let dimension_order = dim_manager.get_dimension_order();
    let coordinate_vars: HashMap<String, Vec<f64>> =
        get_coordinate_variables(file, dimension_order)?;
    let combinations = dim_manager.get_all_coordinate_combinations();

    let mut data_columns: HashMap<String, Vec<f64>> = HashMap::new();
    let mut variable_values = Vec::new();

    for dim_name in dimension_order {
        data_columns.insert(dim_name.clone(), Vec::new());
    }

    for combination in &combinations {
        for (i, dim_name) in dimension_order.iter().enumerate() {
            let idx = combination[i];

            let coord_value = coordinate_vars
                .get(dim_name)
                .map(|coords| coords[idx])
                .unwrap_or(idx as f64);
            data_columns.get_mut(dim_name).unwrap().push(coord_value);
        }

        let indices: Vec<usize> = combination.clone();
        let value = extract_variable_value(var, &indices)?;
        variable_values.push(value);
    }

    let mut columns = Vec::new();

    for dim_name in dimension_order {
        let values = data_columns.remove(dim_name).unwrap();
        columns.push(Series::new(dim_name.as_str().into(), values).into());
    }

    columns.push(Series::new(var_name.into(), variable_values).into());

    let df = DataFrame::new(columns)?;
    Ok(df)
}

fn get_coordinate_variables(
    file: &netcdf::File,
    dimension_order: &[String],
) -> Result<HashMap<String, Vec<f64>>, Nc2ParquetError> {
    let mut coordinate_vars = HashMap::new();

    for dim_name in dimension_order {
        if let Some(coord_var) = file.variable(dim_name)
            && let Ok(coords_array) = coord_var.get::<f64, _>(..)
        {
            let coords_vec: Vec<f64> = coords_array.iter().cloned().collect();
            coordinate_vars.insert(dim_name.clone(), coords_vec);
        }
    }

    Ok(coordinate_vars)
}

fn extract_variable_value(
    var: &netcdf::Variable,
    indices: &[usize],
) -> Result<f32, Nc2ParquetError> {
    match indices.len() {
        1 => {
            let value_array = var.get::<f32, _>(indices[0])?;
            Ok(value_array[[]])
        }
        2 => {
            let value_array = var.get::<f32, _>((indices[0], indices[1]))?;
            Ok(value_array[[]])
        }
        3 => {
            let value_array = var.get::<f32, _>((indices[0], indices[1], indices[2]))?;
            Ok(value_array[[]])
        }
        4 => {
            let value_array =
                var.get::<f32, _>((indices[0], indices[1], indices[2], indices[3]))?;
            Ok(value_array[[]])
        }
        _ => Err(Nc2ParquetError::UnsupportedDimensionality(indices.len())),
    }
}

use crate::errors::Nc2ParquetError;
use crate::filters::{FilterResult, NCFilter};
use polars::prelude::*;
use std::collections::{HashMap, HashSet};

/// A flat, cache-friendly buffer for storing multi-dimensional index combinations.
///
/// Replaces `Vec<Vec<usize>>` by packing all combinations into a single contiguous
/// allocation. Each combination occupies exactly `stride` consecutive elements.
///
/// # Layout
///
/// For `n` combinations of `d` dimensions:
/// ```text
/// [combo0_dim0, combo0_dim1, ..., combo0_dimD-1,
///  combo1_dim0, combo1_dim1, ..., combo1_dimD-1,
///  ...]
/// ```
pub(crate) struct CombinationBuffer {
    data: Vec<usize>,
    stride: usize,
}

impl CombinationBuffer {
    /// Allocates a flat buffer with capacity for `num_combinations` entries, each
    /// of length `num_dimensions`.
    fn with_capacity(num_combinations: usize, num_dimensions: usize) -> Self {
        Self {
            data: Vec::with_capacity(num_combinations * num_dimensions),
            stride: num_dimensions,
        }
    }

    /// Appends one combination to the buffer.
    ///
    /// # Panics
    ///
    /// Panics (in debug builds) if `combo.len() != self.stride`.
    fn push_combination(&mut self, combo: &[usize]) {
        debug_assert_eq!(
            combo.len(),
            self.stride,
            "push_combination: combo length {} does not match stride {}",
            combo.len(),
            self.stride
        );
        self.data.extend_from_slice(combo);
    }

    /// Returns the number of stored combinations.
    pub(crate) fn len(&self) -> usize {
        if self.stride == 0 {
            0
        } else {
            self.data.len() / self.stride
        }
    }

    /// Returns an iterator over combinations as `&[usize]` slices.
    #[allow(dead_code)] // Provided as part of the buffer API; IntoIterator is used internally
    pub(crate) fn iter(&self) -> impl Iterator<Item = &[usize]> {
        self.data.chunks_exact(self.stride)
    }
}

/// Allows `for combo in &buffer` syntax in test code.
impl<'a> IntoIterator for &'a CombinationBuffer {
    type Item = &'a [usize];
    type IntoIter = std::slice::ChunksExact<'a, usize>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.chunks_exact(self.stride)
    }
}

/// Manages dimension indices and coordinate combinations during filtering operations.
#[derive(Debug, Clone)]
pub(crate) struct DimensionIndexManager {
    dimension_indices: HashMap<String, HashSet<usize>>,
    dimension_order: Vec<String>,
    explicit_combinations: Option<CombinationBuffer>,
}

impl std::fmt::Debug for CombinationBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CombinationBuffer")
            .field("len", &self.len())
            .field("stride", &self.stride)
            .finish()
    }
}

impl Clone for CombinationBuffer {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            stride: self.stride,
        }
    }
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

        let other_dimensions: Vec<(usize, Vec<usize>)> = self
            .dimension_order
            .iter()
            .enumerate()
            .filter(|(pos, _)| *pos != lat_pos && *pos != lon_pos)
            .map(|(pos, dim_name)| {
                let mut indices: Vec<usize> =
                    self.dimension_indices[dim_name].iter().cloned().collect();
                indices.sort_unstable();
                (pos, indices)
            })
            .collect();

        let num_dims = self.dimension_order.len();
        let other_total: usize = other_dimensions.iter().map(|(_, v)| v.len()).product();
        let total = other_total * pairs.len();
        let mut buffer = CombinationBuffer::with_capacity(total, num_dims);

        self.generate_combinations_with_pairs(
            &other_dimensions,
            pairs,
            lat_pos,
            lon_pos,
            &mut vec![0usize; num_dims],
            0,
            &mut buffer,
        );

        self.explicit_combinations = Some(buffer);
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

        let num_dims = self.dimension_order.len();
        let mut buffer = CombinationBuffer::with_capacity(triplets.len(), num_dims);
        let mut coord = vec![0usize; num_dims];
        for &(time_idx, lat_idx, lon_idx) in triplets {
            coord[time_pos] = time_idx;
            coord[lat_pos] = lat_idx;
            coord[lon_pos] = lon_idx;
            buffer.push_combination(&coord);
        }

        self.explicit_combinations = Some(buffer);
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
        results: &mut CombinationBuffer,
    ) {
        if other_dim_idx >= other_dims.len() {
            for &(lat_idx, lon_idx) in pairs {
                current[lat_pos] = lat_idx;
                current[lon_pos] = lon_idx;
                results.push_combination(current);
            }
            return;
        }

        let (dim_pos, ref indices) = other_dims[other_dim_idx];
        for &idx in indices {
            current[dim_pos] = idx;
            self.generate_combinations_with_pairs(
                other_dims,
                pairs,
                lat_pos,
                lon_pos,
                current,
                other_dim_idx + 1,
                results,
            );
        }
    }

    #[allow(dead_code)] // Used in #[cfg(test)] modules
    pub(crate) fn get_dimension_indices(&self, dim_name: &str) -> Option<&HashSet<usize>> {
        self.dimension_indices.get(dim_name)
    }

    pub(crate) fn get_dimension_order(&self) -> &Vec<String> {
        &self.dimension_order
    }

    pub(crate) fn get_all_coordinate_combinations(&self) -> CombinationBuffer {
        if let Some(ref explicit) = self.explicit_combinations {
            explicit.clone()
        } else {
            let sorted_dims: Vec<Vec<usize>> = self
                .dimension_order
                .iter()
                .map(|dim_name| {
                    let mut indices: Vec<usize> =
                        self.dimension_indices[dim_name].iter().cloned().collect();
                    indices.sort_unstable();
                    indices
                })
                .collect();

            let num_dims = sorted_dims.len();
            let total: usize = sorted_dims.iter().map(|d| d.len()).product();
            let mut buffer = CombinationBuffer::with_capacity(total, num_dims);
            let mut current = vec![0usize; num_dims];
            Self::generate_combinations_flat(&sorted_dims, &mut current, 0, &mut buffer);
            buffer
        }
    }

    /// Returns true when the extraction is a Cartesian product (no explicit Pairs/Triplets).
    pub(crate) fn is_cartesian_product(&self) -> bool {
        self.explicit_combinations.is_none()
    }

    /// Returns the dimension names paired with their sorted, deduplicated index sets.
    ///
    /// The returned vector preserves the original dimension order from the NetCDF variable.
    pub(crate) fn sorted_dimension_indices(&self) -> Vec<(String, Vec<usize>)> {
        self.dimension_order
            .iter()
            .map(|dim_name| {
                let mut indices: Vec<usize> = self
                    .dimension_indices
                    .get(dim_name)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                indices.sort();
                indices.dedup();
                (dim_name.clone(), indices)
            })
            .collect()
    }

    fn generate_combinations_flat(
        sorted_dims: &[Vec<usize>],
        current: &mut Vec<usize>,
        dim_index: usize,
        result: &mut CombinationBuffer,
    ) {
        if dim_index >= sorted_dims.len() {
            result.push_combination(current);
            return;
        }
        for &idx in &sorted_dims[dim_index] {
            current[dim_index] = idx;
            Self::generate_combinations_flat(sorted_dims, current, dim_index + 1, result);
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

/// Extracts multiple variables from the same NetCDF file into a single DataFrame.
///
/// Dimension columns are shared across all variables. Each variable produces one
/// additional value column. All variables must have identical dimensions (same
/// names and sizes as the first variable); a mismatch returns an error.
///
/// The `DimensionIndexManager` (filters applied once against the first variable)
/// is reused for every subsequent variable, so filters are applied exactly once
/// and the same row set is produced for all variables.
pub(crate) fn extract_multi_variable_dataframe(
    file: &netcdf::File,
    var_names: &[String],
    filters: &[Box<dyn NCFilter>],
) -> Result<DataFrame, Nc2ParquetError> {
    debug_assert!(!var_names.is_empty(), "var_names must be non-empty");

    let first_name = &var_names[0];

    // Build dim_manager from the first variable and apply filters once.
    // The variable borrow is scoped to end before the loop below.
    let dim_manager = {
        let first_var = file
            .variable(first_name)
            .ok_or_else(|| Nc2ParquetError::VariableNotFound(first_name.clone()))?;
        let mut dm = DimensionIndexManager::new(&first_var)?;
        for filter in filters.iter() {
            let result = filter.apply(file)?;
            dm.apply_filter_result(&result)?;
        }
        dm
    };

    let first_dims: Vec<(String, usize)> = {
        let first_var = file
            .variable(first_name)
            .ok_or_else(|| Nc2ParquetError::VariableNotFound(first_name.clone()))?;
        first_var
            .dimensions()
            .iter()
            .map(|d| (d.name().to_string(), d.len()))
            .collect()
    };

    let mut df = {
        let first_var = file
            .variable(first_name)
            .ok_or_else(|| Nc2ParquetError::VariableNotFound(first_name.clone()))?;
        extract_data_with_dimension_manager(file, &first_var, first_name, &dim_manager)?
    };

    for name in var_names.iter().skip(1) {
        let var_dims: Vec<(String, usize)> = {
            let var = file
                .variable(name)
                .ok_or_else(|| Nc2ParquetError::VariableNotFound(name.clone()))?;
            var.dimensions()
                .iter()
                .map(|d| (d.name().to_string(), d.len()))
                .collect()
        };

        if var_dims != first_dims {
            return Err(Nc2ParquetError::Extraction(format!(
                "Variable '{}' has dimensions {:?} but expected {:?} (matching first variable '{}')",
                name, var_dims, first_dims, first_name
            )));
        }

        let values: Vec<f32> = {
            let var = file
                .variable(name)
                .ok_or_else(|| Nc2ParquetError::VariableNotFound(name.clone()))?;
            extract_variable_values_with_dim_manager(&var, &dim_manager)?
        };

        df.with_column(Series::new(name.as_str().into(), values))?;
    }

    Ok(df)
}

/// Extracts variable values (without dimension columns) using a pre-built
/// `DimensionIndexManager`. Used by `extract_multi_variable_dataframe` to
/// obtain additional variable columns without rebuilding the dim_manager.
fn extract_variable_values_with_dim_manager(
    var: &netcdf::Variable,
    dim_manager: &DimensionIndexManager,
) -> Result<Vec<f32>, Nc2ParquetError> {
    if dim_manager.is_cartesian_product() {
        extract_variable_values_batch(var, dim_manager)
    } else {
        extract_variable_values_cellwise(var, dim_manager)
    }
}

/// Values-only extraction for the Cartesian-product (batch slab-read) path.
fn extract_variable_values_batch(
    var: &netcdf::Variable,
    dim_manager: &DimensionIndexManager,
) -> Result<Vec<f32>, Nc2ParquetError> {
    let dim_indices = dim_manager.sorted_dimension_indices();

    if dim_indices.iter().any(|(_, idxs)| idxs.is_empty()) {
        return Ok(Vec::new());
    }

    let ndims = dim_indices.len();
    let mut starts = Vec::with_capacity(ndims);
    let mut counts = Vec::with_capacity(ndims);
    let mut local_offsets: Vec<Vec<usize>> = Vec::with_capacity(ndims);

    for (_, idxs) in &dim_indices {
        let first = idxs[0];
        let last = *idxs.last().expect("non-empty checked above");
        starts.push(first);
        counts.push(last - first + 1);
        let offsets: Vec<usize> = idxs.iter().map(|&g| g - first).collect();
        local_offsets.push(offsets);
    }

    let extents = netcdf::Extents::try_from((starts.as_slice(), counts.as_slice()))
        .map_err(Nc2ParquetError::NetCdf)?;
    let slab: Vec<f32> = var.get_values::<f32, _>(extents)?;

    let mut strides = vec![1usize; ndims];
    for i in (0..ndims.saturating_sub(1)).rev() {
        strides[i] = strides[i + 1] * counts[i + 1];
    }

    let total_rows: usize = local_offsets.iter().map(|v| v.len()).product();
    let mut values: Vec<f32> = Vec::with_capacity(total_rows);

    let mut pos = vec![0usize; ndims];

    'outer: loop {
        let flat: usize = pos
            .iter()
            .enumerate()
            .map(|(d, &p)| local_offsets[d][p] * strides[d])
            .sum();
        values.push(slab[flat]);

        let mut carry = true;
        for d in (0..ndims).rev() {
            if carry {
                pos[d] += 1;
                if pos[d] < local_offsets[d].len() {
                    carry = false;
                } else {
                    pos[d] = 0;
                }
            }
        }
        if carry {
            break 'outer;
        }
    }

    Ok(values)
}

/// Values-only extraction for the cellwise (explicit Pairs/Triplets) path.
fn extract_variable_values_cellwise(
    var: &netcdf::Variable,
    dim_manager: &DimensionIndexManager,
) -> Result<Vec<f32>, Nc2ParquetError> {
    let combinations = dim_manager.get_all_coordinate_combinations();
    let mut values: Vec<f32> = Vec::with_capacity(combinations.len());

    for combination in &combinations {
        let value = extract_variable_value(var, combination)?;
        values.push(value);
    }

    Ok(values)
}

fn extract_data_with_dimension_manager(
    file: &netcdf::File,
    var: &netcdf::Variable,
    var_name: &str,
    dim_manager: &DimensionIndexManager,
) -> Result<DataFrame, Nc2ParquetError> {
    if dim_manager.is_cartesian_product() {
        extract_data_batch(file, var, var_name, dim_manager)
    } else {
        extract_data_cellwise(file, var, var_name, dim_manager)
    }
}

/// Extracts variable data cell-by-cell for explicit (Pairs/Triplets) filter combinations.
///
/// Each cell is read individually using index-based access. This path is used
/// when `dim_manager` holds explicit combinations that cannot be represented as
/// a Cartesian product (e.g. from `NC2DPointFilter` or `NC3DPointFilter`).
fn extract_data_cellwise(
    file: &netcdf::File,
    var: &netcdf::Variable,
    var_name: &str,
    dim_manager: &DimensionIndexManager,
) -> Result<DataFrame, Nc2ParquetError> {
    let dimension_order = dim_manager.get_dimension_order();

    // Drop coordinate_vars and combinations before building the DataFrame to
    // avoid holding both the raw buffers and the Polars columns simultaneously.
    let (data_columns, variable_values) = {
        let coordinate_vars: HashMap<String, Vec<f64>> =
            get_coordinate_variables(file, dimension_order)?;
        let combinations = dim_manager.get_all_coordinate_combinations();

        let total_rows = combinations.len();
        let mut data_columns: HashMap<String, Vec<f64>> = dimension_order
            .iter()
            .map(|name| (name.clone(), Vec::with_capacity(total_rows)))
            .collect();
        let mut variable_values: Vec<f32> = Vec::with_capacity(total_rows);

        for combination in &combinations {
            for (i, dim_name) in dimension_order.iter().enumerate() {
                let idx = combination[i];
                let coord_value = coordinate_vars
                    .get(dim_name)
                    .map(|coords| coords[idx])
                    .unwrap_or(idx as f64);
                data_columns.get_mut(dim_name).unwrap().push(coord_value);
            }

            let value = extract_variable_value(var, combination)?;
            variable_values.push(value);
        }

        (data_columns, variable_values)
    };

    build_dataframe(dimension_order, data_columns, var_name, variable_values)
}

/// Extracts variable data using a single slab read for Cartesian product extractions.
///
/// For each dimension the bounding box `[min_index, max_index]` is computed, and the
/// entire sub-array is fetched in one NetCDF call via `var.get`. The returned
/// `ndarray::ArrayD<f32>` is stored in row-major (C) order, so local offsets within
/// the bounding box map directly to the flattened array. Non-contiguous indices (gaps
/// inside the bounding box) are included in the read and filtered out during iteration,
/// preserving the same Cartesian product ordering as the cellwise path.
///
/// Empty extractions (any dimension has no selected indices) return an empty `DataFrame`
/// with the correct schema immediately, without performing any NetCDF I/O.
fn extract_data_batch(
    file: &netcdf::File,
    var: &netcdf::Variable,
    var_name: &str,
    dim_manager: &DimensionIndexManager,
) -> Result<DataFrame, Nc2ParquetError> {
    let dim_indices = dim_manager.sorted_dimension_indices();
    let dimension_order = dim_manager.get_dimension_order();

    if dim_indices.iter().any(|(_, idxs)| idxs.is_empty()) {
        let empty_data_columns: HashMap<String, Vec<f64>> = dimension_order
            .iter()
            .map(|name| (name.clone(), Vec::new()))
            .collect();
        return build_dataframe(dimension_order, empty_data_columns, var_name, Vec::new());
    }

    // Compute the bounding box (start index + count) for each dimension.
    // Non-contiguous selected indices are handled by reading the full bounding box
    // and filtering during iteration — this avoids multiple slab reads at the
    // cost of a small amount of wasted I/O for sparse selections.
    let ndims = dim_indices.len();
    let mut starts = Vec::with_capacity(ndims);
    let mut counts = Vec::with_capacity(ndims);
    let mut local_offsets: Vec<Vec<usize>> = Vec::with_capacity(ndims);

    for (_, idxs) in &dim_indices {
        let first = idxs[0];
        let last = *idxs.last().expect("non-empty checked above");
        starts.push(first);
        counts.push(last - first + 1);
        let offsets: Vec<usize> = idxs.iter().map(|&g| g - first).collect();
        local_offsets.push(offsets);
    }

    // Drop coordinate_vars and slab before building the DataFrame to avoid
    // holding both the raw buffers and the Polars columns simultaneously.
    let (data_columns, variable_values) = {
        let coordinate_vars: HashMap<String, Vec<f64>> =
            get_coordinate_variables(file, dimension_order)?;

        let extents = netcdf::Extents::try_from((starts.as_slice(), counts.as_slice()))
            .map_err(Nc2ParquetError::NetCdf)?;
        // get_values returns a Vec<f32> in C (row-major) order matching the stride indexing below.
        let slab: Vec<f32> = var.get_values::<f32, _>(extents)?;

        // stride[i] = product of counts[i+1..ndims] (row-major).
        let mut strides = vec![1usize; ndims];
        for i in (0..ndims.saturating_sub(1)).rev() {
            strides[i] = strides[i + 1] * counts[i + 1];
        }

        let total_rows: usize = local_offsets.iter().map(|v| v.len()).product();

        let mut data_columns: HashMap<String, Vec<f64>> = dimension_order
            .iter()
            .map(|name| (name.clone(), Vec::with_capacity(total_rows)))
            .collect();
        let mut variable_values: Vec<f32> = Vec::with_capacity(total_rows);

        // Iterate over the Cartesian product of the per-dimension local offsets.
        // The row-major flat index into `slab` is: sum(local_offsets[d][pos[d]] * strides[d]).
        let mut pos = vec![0usize; ndims];

        'outer: loop {
            let flat: usize = pos
                .iter()
                .enumerate()
                .map(|(d, &p)| local_offsets[d][p] * strides[d])
                .sum();

            let value = slab[flat];

            for (d, dim_name) in dimension_order.iter().enumerate() {
                let global_idx = starts[d] + local_offsets[d][pos[d]];
                let coord_value = coordinate_vars
                    .get(dim_name)
                    .map(|coords| coords[global_idx])
                    .unwrap_or(global_idx as f64);
                data_columns.get_mut(dim_name).unwrap().push(coord_value);
            }
            variable_values.push(value);

            let mut carry = true;
            for d in (0..ndims).rev() {
                if carry {
                    pos[d] += 1;
                    if pos[d] < local_offsets[d].len() {
                        carry = false;
                    } else {
                        pos[d] = 0;
                    }
                }
            }
            if carry {
                break 'outer;
            }
        }

        (data_columns, variable_values)
    };

    build_dataframe(dimension_order, data_columns, var_name, variable_values)
}

/// Assembles the final `DataFrame` from pre-built column data.
///
/// Columns are emitted in `dimension_order` followed by the variable column.
fn build_dataframe(
    dimension_order: &[String],
    mut data_columns: HashMap<String, Vec<f64>>,
    var_name: &str,
    variable_values: Vec<f32>,
) -> Result<DataFrame, Nc2ParquetError> {
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

/// Extracts multiple variables with potentially different dimensions into one DataFrame.
///
/// The variable with the most dimensions is chosen as the "master" — its full
/// Cartesian product forms the output skeleton (dimension columns). All other
/// variables are broadcast (each value is repeated as needed) to match that
/// skeleton, so every row has a value for every variable.
///
/// This is equivalent to a cross-join / broadcast in columnar databases: each
/// lower-dimensional variable's values are repeated to fill the full dimension
/// space of the master variable.
///
/// # Errors
///
/// Returns an error if any variable has a dimension not present in the master
/// variable (i.e. a dimension that cannot be broadcast into).
pub(crate) fn extract_merge_variable_dataframe(
    file: &netcdf::File,
    var_names: &[String],
    _filters: &[Box<dyn NCFilter>],
) -> Result<DataFrame, Nc2ParquetError> {
    debug_assert!(!var_names.is_empty(), "var_names must be non-empty");

    // Gather dimension info for each variable
    struct VarInfo {
        name: String,
        dims: Vec<(String, usize)>,
    }

    let var_infos: Vec<VarInfo> = var_names
        .iter()
        .map(|name| {
            let var = file
                .variable(name)
                .ok_or_else(|| Nc2ParquetError::VariableNotFound(name.clone()))?;
            let dims: Vec<(String, usize)> = var
                .dimensions()
                .iter()
                .map(|d| (d.name().to_string(), d.len()))
                .collect();
            Ok(VarInfo {
                name: name.clone(),
                dims,
            })
        })
        .collect::<Result<Vec<_>, Nc2ParquetError>>()?;

    // Find master = variable with the most dimensions.
    // Tie goes to the first encountered.
    let master_idx = var_infos
        .iter()
        .enumerate()
        .max_by_key(|(_, vi)| vi.dims.len())
        .map(|(i, _)| i)
        .unwrap_or(0);

    let master_name = &var_infos[master_idx].name;
    let master_dims = &var_infos[master_idx].dims;
    let master_dim_names: Vec<String> = master_dims.iter().map(|(n, _)| n.clone()).collect();
    let master_sizes: Vec<usize> = master_dims.iter().map(|(_, s)| *s).collect();

    // Verify all variable dims are subsets of master dims.
    for vi in &var_infos {
        for (dim_name, _) in &vi.dims {
            if !master_dim_names.contains(dim_name) {
                return Err(Nc2ParquetError::Extraction(format!(
                    "Cannot merge variable '{}': dimension '{}' not found in master variable '{}' (dims: {:?})",
                    vi.name, dim_name, master_name, master_dim_names,
                )));
            }
        }
    }

    // Extract master variable with dimension columns.
    let master_var = file
        .variable(master_name)
        .ok_or_else(|| Nc2ParquetError::VariableNotFound(master_name.clone()))?;
    let master_dim_mgr = DimensionIndexManager::new(&master_var)?;
    let mut df = extract_data_with_dimension_manager(
        file,
        &master_var,
        master_name,
        &master_dim_mgr,
    )?;

    // Extract and broadcast remaining variables.
    for vi in &var_infos {
        if vi.name == *master_name {
            continue;
        }

        let var = file
            .variable(&vi.name)
            .ok_or_else(|| Nc2ParquetError::VariableNotFound(vi.name.clone()))?;
        let var_dim_mgr = DimensionIndexManager::new(&var)?;
        let values = extract_variable_values_with_dim_manager(&var, &var_dim_mgr)?;

        let expanded =
            expand_values_to_master(&values, &vi.dims, &master_dim_names, &master_sizes);

        df.with_column(Series::new(vi.name.as_str().into(), expanded))?;
    }

    Ok(df)
}

/// Expands values from a variable's native dimension space to the master's
/// dimension space by repeating (broadcasting) values across missing dimensions.
///
/// Each value in `source_values` at position `(d0, d1, ...)` in the source's
/// row-major index space is copied to every master position that shares the
/// same indices on the source's dimensions.
fn expand_values_to_master(
    source_values: &[f32],
    source_dims: &[(String, usize)],
    master_dim_names: &[String],
    master_sizes: &[usize],
) -> Vec<f32> {
    let ndims = master_dim_names.len();

    // Build mapping from source dimension name to its position in master.
    let source_dim_to_master_pos: Vec<usize> = source_dims
        .iter()
        .map(|(name, _)| {
            master_dim_names
                .iter()
                .position(|m| m == name)
                .expect("source dim must exist in master; checked by caller")
        })
        .collect();

    // Compute strides for the source variable's own dimension space (row-major,
    // last dim varies fastest).
    let source_sizes: Vec<usize> = source_dims.iter().map(|(_, s)| *s).collect();
    let mut source_strides = vec![1usize; source_dims.len()];
    for i in (0..source_dims.len().saturating_sub(1)).rev() {
        source_strides[i] = source_strides[i + 1] * source_sizes[i + 1];
    }

    let total: usize = master_sizes.iter().product();
    let mut expanded = Vec::with_capacity(total);

    // Iterate over the master's Cartesian product in row-major order.
    let mut pos = vec![0usize; ndims];
    loop {
        // Compute flat index into the source value array.
        let mut source_idx = 0usize;
        for (si, &mp) in source_dim_to_master_pos.iter().enumerate() {
            source_idx += pos[mp] * source_strides[si];
        }
        expanded.push(source_values[source_idx]);

        // Advance to the next master combination (row-major carry).
        let mut carry = true;
        for d in (0..ndims).rev() {
            if carry {
                pos[d] += 1;
                if pos[d] < master_sizes[d] {
                    carry = false;
                } else {
                    pos[d] = 0;
                }
            }
        }
        if carry {
            break;
        }
    }

    expanded
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

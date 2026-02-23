#[allow(dead_code)] // Used in #[cfg(test)] modules via as_pairs()
type PairResult<'a> = Option<(&'a String, &'a String, &'a Vec<(usize, usize)>)>;

/// Type alias for coordinate triplet result tuple
#[allow(dead_code)] // Used in #[cfg(test)] modules via as_triplets()
type TripletResult<'a> = Option<(
    &'a String,
    &'a String,
    &'a String,
    &'a Vec<(usize, usize, usize)>,
)>;

use crate::errors::Nc2ParquetError;
use serde::Deserialize;

/// Result of applying a filter to NetCDF data.
///
/// This enum encapsulates different types of filter results while preserving
/// dimension information for proper intersection operations.
#[derive(Debug, Clone)]
pub enum FilterResult {
    /// Result for filters that operate on a single dimension.
    ///
    /// Contains the matched indices into that dimension's coordinate array.
    Single {
        /// Name of the filtered dimension.
        dimension: String,
        /// Indices of the matching coordinate values.
        indices: Vec<usize>,
    },
    /// Result for filters that operate on a (latitude, longitude) coordinate pair.
    ///
    /// Each element of `pairs` is `(lat_index, lon_index)`.
    Pairs {
        /// Name of the latitude dimension.
        lat_dimension: String,
        /// Name of the longitude dimension.
        lon_dimension: String,
        /// Matching `(lat_index, lon_index)` coordinate pairs.
        pairs: Vec<(usize, usize)>,
    },
    /// Result for filters that operate on (time, latitude, longitude) triplets.
    ///
    /// Each element of `triplets` is `(time_index, lat_index, lon_index)`.
    Triplets {
        /// Name of the time dimension.
        time_dimension: String,
        /// Name of the latitude dimension.
        lat_dimension: String,
        /// Name of the longitude dimension.
        lon_dimension: String,
        /// Matching `(time_index, lat_index, lon_index)` coordinate triplets.
        triplets: Vec<(usize, usize, usize)>,
    },
}

impl FilterResult {
    #[allow(dead_code)] // Used in #[cfg(test)] modules
    pub(crate) fn as_single(&self) -> Option<(&String, &Vec<usize>)> {
        if let FilterResult::Single { dimension, indices } = self {
            Some((dimension, indices))
        } else {
            None
        }
    }

    #[allow(dead_code)] // Used in #[cfg(test)] modules
    pub(crate) fn as_pairs(&self) -> PairResult<'_> {
        if let FilterResult::Pairs {
            lat_dimension,
            lon_dimension,
            pairs,
        } = self
        {
            Some((lat_dimension, lon_dimension, pairs))
        } else {
            None
        }
    }

    #[allow(dead_code)] // Used in #[cfg(test)] modules
    pub(crate) fn as_triplets(&self) -> TripletResult<'_> {
        if let FilterResult::Triplets {
            time_dimension,
            lat_dimension,
            lon_dimension,
            triplets,
        } = self
        {
            Some((time_dimension, lat_dimension, lon_dimension, triplets))
        } else {
            None
        }
    }

    #[allow(dead_code)] // Used in #[cfg(test)] modules
    pub(crate) fn len(&self) -> usize {
        match self {
            FilterResult::Single { indices, .. } => indices.len(),
            FilterResult::Pairs { pairs, .. } => pairs.len(),
            FilterResult::Triplets { triplets, .. } => triplets.len(),
        }
    }

    #[allow(dead_code)] // Used in #[cfg(test)] modules
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Core trait for filtering NetCDF data along one or more dimensions.
///
/// Implement this trait to create custom filter types. The `apply` method
/// receives an opened NetCDF file and returns the set of matching coordinate
/// indices as a [`FilterResult`].
pub trait NCFilter {
    /// Apply the filter to the given NetCDF file and return matching indices.
    ///
    /// # Errors
    ///
    /// Returns [`Nc2ParquetError::DimensionNotFound`] if a required dimension
    /// variable does not exist in the file, or a [`Nc2ParquetError::NetCdf`]
    /// error if reading coordinate values fails.
    fn apply(&self, file: &netcdf::File) -> Result<FilterResult, Nc2ParquetError>;
}

/// Filter that selects dimension values within a numeric range.
///
/// Values are included when `min_value <= value <= max_value`.
#[derive(Deserialize)]
pub struct NCRangeFilter {
    pub(crate) dimension_name: String,
    pub(crate) min_value: f64,
    pub(crate) max_value: f64,
}

impl NCRangeFilter {
    /// Creates a new range filter for the given dimension and bounds.
    ///
    /// # Arguments
    ///
    /// * `dimension_name` - Name of the NetCDF dimension variable to filter
    /// * `min_value` - Lower bound of the range (inclusive)
    /// * `max_value` - Upper bound of the range (inclusive)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::filters::NCRangeFilter;
    ///
    /// // Select latitudes between 30° and 60° N
    /// let filter = NCRangeFilter::new("latitude", 30.0, 60.0);
    /// ```
    pub fn new(dimension_name: &str, min_value: f64, max_value: f64) -> Self {
        NCRangeFilter {
            dimension_name: dimension_name.to_string(),
            min_value,
            max_value,
        }
    }

    /// Creates a new range filter by deserializing from a JSON string.
    ///
    /// # Arguments
    ///
    /// * `json_str` - JSON string with `dimension_name`, `min_value`, and
    ///   `max_value` fields
    ///
    /// # Errors
    ///
    /// Returns [`Nc2ParquetError::Serialization`] if the JSON is malformed or
    /// missing required fields.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::filters::NCRangeFilter;
    ///
    /// let filter = NCRangeFilter::from_json(
    ///     r#"{"dimension_name":"latitude","min_value":30.0,"max_value":60.0}"#
    /// ).unwrap();
    /// ```
    pub fn from_json(json_str: &str) -> Result<Self, Nc2ParquetError> {
        let f: NCRangeFilter = serde_json::from_str(json_str)
            .map_err(|e| Nc2ParquetError::Serialization(e.to_string()))?;
        Ok(f)
    }
}

impl NCFilter for NCRangeFilter {
    fn apply(&self, file: &netcdf::File) -> Result<FilterResult, Nc2ParquetError> {
        if let Some(var) = file.variable(&self.dimension_name) {
            let values = var.get::<f64, _>(..)?;
            let filtered_indices: Vec<usize> = values
                .iter()
                .enumerate()
                .filter(|(_, val)| **val >= self.min_value && **val <= self.max_value)
                .map(|(idx, _)| idx)
                .collect();
            Ok(FilterResult::Single {
                dimension: self.dimension_name.clone(),
                indices: filtered_indices,
            })
        } else {
            Err(Nc2ParquetError::DimensionNotFound(
                self.dimension_name.clone(),
            ))
        }
    }
}

/// Filter that selects dimension values from an explicit list.
///
/// Only values that exactly match one of the entries in `values` are included.
#[derive(Deserialize)]
pub struct NCListFilter {
    pub(crate) dimension_name: String,
    pub(crate) values: Vec<f64>,
}

impl NCListFilter {
    /// Creates a new list filter for the given dimension and allowed values.
    ///
    /// # Arguments
    ///
    /// * `dimension_name` - Name of the NetCDF dimension variable to filter
    /// * `values` - The discrete values to include in the output
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::filters::NCListFilter;
    ///
    /// // Select only the 1000 hPa, 850 hPa, and 500 hPa pressure levels
    /// let filter = NCListFilter::new("pressure", vec![1000.0, 850.0, 500.0]);
    /// ```
    pub fn new(dimension_name: &str, values: Vec<f64>) -> Self {
        NCListFilter {
            dimension_name: dimension_name.to_string(),
            values,
        }
    }

    /// Creates a new list filter by deserializing from a JSON string.
    ///
    /// # Arguments
    ///
    /// * `json_str` - JSON string with `dimension_name` and `values` fields
    ///
    /// # Errors
    ///
    /// Returns [`Nc2ParquetError::Serialization`] if the JSON is malformed or
    /// missing required fields.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::filters::NCListFilter;
    ///
    /// let filter = NCListFilter::from_json(
    ///     r#"{"dimension_name":"pressure","values":[1000.0,850.0,500.0]}"#
    /// ).unwrap();
    /// ```
    pub fn from_json(json_str: &str) -> Result<Self, Nc2ParquetError> {
        let f: NCListFilter = serde_json::from_str(json_str)
            .map_err(|e| Nc2ParquetError::Serialization(e.to_string()))?;
        Ok(f)
    }
}

impl NCFilter for NCListFilter {
    fn apply(&self, file: &netcdf::File) -> Result<FilterResult, Nc2ParquetError> {
        if let Some(var) = file.variable(&self.dimension_name) {
            let coord_values = var.get::<f64, _>(..)?;
            let filtered_indices: Vec<usize> = coord_values
                .iter()
                .enumerate()
                .filter(|(_, val)| self.values.contains(val))
                .map(|(idx, _)| idx)
                .collect();
            Ok(FilterResult::Single {
                dimension: self.dimension_name.clone(),
                indices: filtered_indices,
            })
        } else {
            Err(Nc2ParquetError::DimensionNotFound(
                self.dimension_name.clone(),
            ))
        }
    }
}

/// Filter that selects grid cells near a set of (latitude, longitude) points.
///
/// A grid cell is included when both its latitude is within `tolerance` of the
/// target latitude **and** its longitude is within `tolerance` of the target longitude.
#[derive(Deserialize)]
pub struct NC2DPointFilter {
    pub(crate) lat_dimension_name: String,
    pub(crate) lon_dimension_name: String,
    pub(crate) points: Vec<(f64, f64)>,
    pub(crate) tolerance: f64,
}

impl NC2DPointFilter {
    /// Creates a new 2D point filter.
    ///
    /// # Arguments
    ///
    /// * `lat_dimension_name` - Name of the latitude dimension variable
    /// * `lon_dimension_name` - Name of the longitude dimension variable
    /// * `points` - Target `(latitude, longitude)` pairs to match
    /// * `tolerance` - Maximum coordinate difference (in the same units as the
    ///   dimension values) to still consider a grid cell a match
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::filters::NC2DPointFilter;
    ///
    /// // Select grid cells within 0.1° of Rio de Janeiro and São Paulo
    /// let filter = NC2DPointFilter::new(
    ///     "latitude",
    ///     "longitude",
    ///     vec![(-22.9, -43.2), (-23.5, -46.6)],
    ///     0.1,
    /// );
    /// ```
    pub fn new(
        lat_dimension_name: &str,
        lon_dimension_name: &str,
        points: Vec<(f64, f64)>,
        tolerance: f64,
    ) -> Self {
        NC2DPointFilter {
            lat_dimension_name: lat_dimension_name.to_string(),
            lon_dimension_name: lon_dimension_name.to_string(),
            points,
            tolerance,
        }
    }

    /// Creates a new 2D point filter by deserializing from a JSON string.
    ///
    /// # Arguments
    ///
    /// * `json_str` - JSON string with `lat_dimension_name`, `lon_dimension_name`,
    ///   `points`, and `tolerance` fields
    ///
    /// # Errors
    ///
    /// Returns [`Nc2ParquetError::Serialization`] if the JSON is malformed or
    /// missing required fields.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::filters::NC2DPointFilter;
    ///
    /// let filter = NC2DPointFilter::from_json(r#"{
    ///     "lat_dimension_name": "latitude",
    ///     "lon_dimension_name": "longitude",
    ///     "points": [[-22.9, -43.2]],
    ///     "tolerance": 0.1
    /// }"#).unwrap();
    /// ```
    pub fn from_json(json_str: &str) -> Result<Self, Nc2ParquetError> {
        let f: NC2DPointFilter = serde_json::from_str(json_str)
            .map_err(|e| Nc2ParquetError::Serialization(e.to_string()))?;
        Ok(f)
    }
}

impl NCFilter for NC2DPointFilter {
    fn apply(&self, file: &netcdf::File) -> Result<FilterResult, Nc2ParquetError> {
        let lat_var = file
            .variable(&self.lat_dimension_name)
            .ok_or_else(|| Nc2ParquetError::DimensionNotFound(self.lat_dimension_name.clone()))?;
        let lon_var = file
            .variable(&self.lon_dimension_name)
            .ok_or_else(|| Nc2ParquetError::DimensionNotFound(self.lon_dimension_name.clone()))?;

        let lat_values = lat_var.get::<f64, _>(..)?;
        let lon_values = lon_var.get::<f64, _>(..)?;

        let mut filtered_indices = Vec::new();

        for &(target_lat, target_lon) in &self.points {
            for (i, &lat) in lat_values.iter().enumerate() {
                if (lat - target_lat).abs() <= self.tolerance {
                    for (j, &lon) in lon_values.iter().enumerate() {
                        if (lon - target_lon).abs() <= self.tolerance {
                            filtered_indices.push((i, j));
                        }
                    }
                }
            }
        }

        Ok(FilterResult::Pairs {
            lat_dimension: self.lat_dimension_name.clone(),
            lon_dimension: self.lon_dimension_name.clone(),
            pairs: filtered_indices,
        })
    }
}

/// Filter that selects grid cells near a set of (latitude, longitude) points
/// at specific time steps.
///
/// A grid cell is included when its time coordinate exactly matches one of the
/// `steps` values **and** both its latitude and longitude are within `tolerance`
/// of one of the target `points`.
#[derive(Deserialize)]
pub struct NC3DPointFilter {
    pub(crate) time_dimension_name: String,
    pub(crate) lat_dimension_name: String,
    pub(crate) lon_dimension_name: String,
    pub(crate) steps: Vec<f64>,
    pub(crate) points: Vec<(f64, f64)>,
    pub(crate) tolerance: f64,
}

impl NC3DPointFilter {
    /// Creates a new 3D point filter.
    ///
    /// # Arguments
    ///
    /// * `time_dimension_name` - Name of the time dimension variable
    /// * `lat_dimension_name` - Name of the latitude dimension variable
    /// * `lon_dimension_name` - Name of the longitude dimension variable
    /// * `steps` - Exact time step values to include (equality match)
    /// * `points` - Target `(latitude, longitude)` pairs to match spatially
    /// * `tolerance` - Maximum coordinate difference for spatial matching
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::filters::NC3DPointFilter;
    ///
    /// // Select time steps 0 and 6 near two spatial locations
    /// let filter = NC3DPointFilter::new(
    ///     "time",
    ///     "latitude",
    ///     "longitude",
    ///     vec![0.0, 6.0],
    ///     vec![(-22.9, -43.2), (-23.5, -46.6)],
    ///     0.1,
    /// );
    /// ```
    pub fn new(
        time_dimension_name: &str,
        lat_dimension_name: &str,
        lon_dimension_name: &str,
        steps: Vec<f64>,
        points: Vec<(f64, f64)>,
        tolerance: f64,
    ) -> Self {
        NC3DPointFilter {
            time_dimension_name: time_dimension_name.to_string(),
            lat_dimension_name: lat_dimension_name.to_string(),
            lon_dimension_name: lon_dimension_name.to_string(),
            steps,
            points,
            tolerance,
        }
    }

    /// Creates a new 3D point filter by deserializing from a JSON string.
    ///
    /// # Arguments
    ///
    /// * `json_str` - JSON string with `time_dimension_name`, `lat_dimension_name`,
    ///   `lon_dimension_name`, `steps`, `points`, and `tolerance` fields
    ///
    /// # Errors
    ///
    /// Returns [`Nc2ParquetError::Serialization`] if the JSON is malformed or
    /// missing required fields.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nc2parquet::filters::NC3DPointFilter;
    ///
    /// let filter = NC3DPointFilter::from_json(r#"{
    ///     "time_dimension_name": "time",
    ///     "lat_dimension_name": "latitude",
    ///     "lon_dimension_name": "longitude",
    ///     "steps": [0.0, 6.0],
    ///     "points": [[-22.9, -43.2]],
    ///     "tolerance": 0.1
    /// }"#).unwrap();
    /// ```
    pub fn from_json(json_str: &str) -> Result<Self, Nc2ParquetError> {
        let f: NC3DPointFilter = serde_json::from_str(json_str)
            .map_err(|e| Nc2ParquetError::Serialization(e.to_string()))?;
        Ok(f)
    }
}

impl NCFilter for NC3DPointFilter {
    fn apply(&self, file: &netcdf::File) -> Result<FilterResult, Nc2ParquetError> {
        let time_var = file
            .variable(&self.time_dimension_name)
            .ok_or_else(|| Nc2ParquetError::DimensionNotFound(self.time_dimension_name.clone()))?;
        let lat_var = file
            .variable(&self.lat_dimension_name)
            .ok_or_else(|| Nc2ParquetError::DimensionNotFound(self.lat_dimension_name.clone()))?;
        let lon_var = file
            .variable(&self.lon_dimension_name)
            .ok_or_else(|| Nc2ParquetError::DimensionNotFound(self.lon_dimension_name.clone()))?;
        let time_values = time_var.get::<f64, _>(..)?;
        let lat_values = lat_var.get::<f64, _>(..)?;
        let lon_values = lon_var.get::<f64, _>(..)?;

        let filtered_time_indices: Vec<usize> = time_values
            .iter()
            .enumerate()
            .filter(|(_, val)| self.steps.contains(val))
            .map(|(idx, _)| idx)
            .collect();

        let mut filtered_indices = Vec::new();

        for &(target_lat, target_lon) in &self.points {
            for (i, &lat) in lat_values.iter().enumerate() {
                if (lat - target_lat).abs() <= self.tolerance {
                    for (j, &lon) in lon_values.iter().enumerate() {
                        if (lon - target_lon).abs() <= self.tolerance {
                            for &t_idx in &filtered_time_indices {
                                filtered_indices.push((t_idx, i, j));
                            }
                        }
                    }
                }
            }
        }

        Ok(FilterResult::Triplets {
            time_dimension: self.time_dimension_name.clone(),
            lat_dimension: self.lat_dimension_name.clone(),
            lon_dimension: self.lon_dimension_name.clone(),
            triplets: filtered_indices,
        })
    }
}

#[allow(dead_code)] // Used in #[cfg(test)] modules
pub(crate) fn filter_factory(json_str: &str) -> Result<Box<dyn NCFilter>, Nc2ParquetError> {
    let v: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| Nc2ParquetError::Serialization(e.to_string()))?;
    if let Some(filter_kind) = v.get("kind").and_then(|t| t.as_str()) {
        match filter_kind {
            "range" => {
                let filter = NCRangeFilter::from_json(json_str)?;
                Ok(Box::new(filter))
            }
            "list" => {
                let filter = NCListFilter::from_json(json_str)?;
                Ok(Box::new(filter))
            }
            "2d_point" => {
                let filter = NC2DPointFilter::from_json(json_str)?;
                Ok(Box::new(filter))
            }
            "3d_point" => {
                let filter = NC3DPointFilter::from_json(json_str)?;
                Ok(Box::new(filter))
            }
            _ => Err(Nc2ParquetError::Filter(format!(
                "Unknown filter kind: {}",
                filter_kind
            ))),
        }
    } else {
        Err(Nc2ParquetError::Filter(
            "Missing 'kind' field in JSON".to_string(),
        ))
    }
}

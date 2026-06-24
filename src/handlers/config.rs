use anyhow::{Context, Result};
use log::debug;
use std::path::Path;

use nc2parquet::{cli::Cli, input::JobConfig};

pub fn load_configuration(
    cli: &Cli,
    input: &Option<String>,
    output: &Option<String>,
    variable: &Option<String>,
    variables: &[String],
    merge_variables: &[String],
) -> Result<JobConfig> {
    // Priority: CLI args > environment variables > config file
    let env_input = std::env::var("NC2PARQUET_INPUT").ok();
    let env_output = std::env::var("NC2PARQUET_OUTPUT").ok();
    let env_variable = std::env::var("NC2PARQUET_VARIABLE").ok();

    if let Some(config_path) = &cli.config {
        let mut config = load_config_file(config_path)?;

        if let Some(env_input_path) = &env_input
            && input.is_none()
        {
            config.nc_key = env_input_path.clone();
        }
        if let Some(env_output_path) = &env_output
            && output.is_none()
        {
            config.parquet_key = env_output_path.clone();
        }
        if let Some(env_var_name) = &env_variable
            && variable.is_none()
        {
            config.variable_name = env_var_name.clone();
        }

        if let Some(input_path) = input {
            config.nc_key = input_path.clone();
        }
        if let Some(output_path) = output {
            config.parquet_key = output_path.clone();
        }
        if let Some(var_name) = variable {
            config.variable_name = var_name.clone();
        }

        return Ok(config);
    }

    let input_path = input.as_ref()
        .or(env_input.as_ref())
        .context("Input file path is required (use --config file, provide INPUT argument, or set NC2PARQUET_INPUT environment variable)")?;

    let output_path = output.as_ref()
        .or(env_output.as_ref())
        .context("Output file path is required (use --config file, provide OUTPUT argument, or set NC2PARQUET_OUTPUT environment variable)")?;

    let var_name = variable.as_ref()
        .or(env_variable.as_ref());

    let use_merge = !merge_variables.is_empty();

    let (effective_var, effective_vars, effective_merge): (String, Option<Vec<String>>, Option<Vec<String>>) =
        if use_merge {
            (merge_variables[0].clone(), None, Some(merge_variables.to_vec()))
        } else {
            match (var_name, variables.is_empty()) {
                (Some(name), _) => (name.clone(), None, None),
                (None, false) => (variables[0].clone(), Some(variables.to_vec()), None),
                (None, true) => anyhow::bail!("Variable name is required (use --config file, --variable or --variables option, or set NC2PARQUET_VARIABLE environment variable)"),
            }
        };

    if let Some(ref vars) = effective_merge {
        debug!(
            "Created configuration from CLI/environment - input: {}, output: {}, merge_variables: {:?}",
            input_path, output_path, vars,
        );
    } else if let Some(ref vars) = effective_vars {
        debug!(
            "Created configuration from CLI/environment - input: {}, output: {}, variables: {:?}",
            input_path, output_path, vars,
        );
    } else {
        debug!(
            "Created configuration from CLI/environment - input: {}, output: {}, variable: {}",
            input_path, output_path, effective_var,
        );
    }

    Ok(JobConfig {
        nc_key: input_path.clone(),
        variable_name: effective_var,
        variable_names: effective_vars,
        merge_variable_names: effective_merge,
        parquet_key: output_path.clone(),
        filters: Vec::new(),
        postprocessing: None,
        output: None,
    })
}

pub fn load_config_file(path: &Path) -> Result<JobConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read configuration file: {}", path.display()))?;

    let config = if path.extension().and_then(|s| s.to_str()) == Some("yaml")
        || path.extension().and_then(|s| s.to_str()) == Some("yml")
    {
        serde_yaml::from_str(&content).context("Failed to parse YAML configuration")?
    } else {
        serde_json::from_str(&content).context("Failed to parse JSON configuration")?
    };

    debug!("Configuration loaded successfully from {}", path.display());
    Ok(config)
}

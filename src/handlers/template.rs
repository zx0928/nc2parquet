use anyhow::{Context, Result};
use log::info;

use nc2duckdb::{
    cli::{Cli, Commands, ConfigFormat, TemplateType},
    input::{FilterConfig, JobConfig, ListParams, OutputTarget, RangeParams},
};

pub async fn handle_template_command(cli: &Cli) -> Result<()> {
    if let Commands::Template {
        template_type,
        output,
        format,
    } = &cli.command
    {
        let template = generate_template(template_type, format)?;

        match output {
            Some(path) => {
                std::fs::write(path, &template).context("Failed to write template file")?;
                info!("Template written to: {}", path.display());
            }
            None => {
                println!("{}", template);
            }
        }
    } else {
        unreachable!("Template command handler called with wrong command type");
    }

    Ok(())
}

pub fn generate_template(template_type: &TemplateType, format: &ConfigFormat) -> Result<String> {
    let config = match template_type {
        TemplateType::Basic => JobConfig {
            nc_key: "input.nc".to_string(),
            variable_name: "temperature".to_string(),
            variable_names: None,
            merge_variable_names: None,
            output: OutputTarget::Parquet {
                parquet_key: "output.parquet".to_string(),
                output: None,
            },
            filters: vec![],
            postprocessing: None,
        },
        TemplateType::S3 => JobConfig {
            nc_key: "s3://my-bucket/input.nc".to_string(),
            variable_name: "temperature".to_string(),
            variable_names: None,
            merge_variable_names: None,
            output: OutputTarget::Parquet {
                parquet_key: "s3://my-bucket/output.parquet".to_string(),
                output: None,
            },
            filters: vec![],
            postprocessing: None,
        },
        TemplateType::MultiFilter => JobConfig {
            nc_key: "weather_data.nc".to_string(),
            variable_name: "temperature".to_string(),
            variable_names: None,
            merge_variable_names: None,
            output: OutputTarget::Parquet {
                parquet_key: "filtered_weather.parquet".to_string(),
                output: None,
            },
            filters: vec![
                FilterConfig::Range {
                    params: RangeParams {
                        dimension_name: "latitude".to_string(),
                        min_value: 30.0,
                        max_value: 60.0,
                    },
                },
                FilterConfig::List {
                    params: ListParams {
                        dimension_name: "pressure".to_string(),
                        values: vec![1000.0, 850.0, 500.0],
                    },
                },
            ],
            postprocessing: None,
        },
        TemplateType::Weather => JobConfig {
            nc_key: "weather_station_data.nc".to_string(),
            variable_name: "temperature".to_string(),
            variable_names: None,
            merge_variable_names: None,
            output: OutputTarget::Parquet {
                parquet_key: "weather_analysis.parquet".to_string(),
                output: None,
            },
            filters: vec![FilterConfig::Range {
                params: RangeParams {
                    dimension_name: "time".to_string(),
                    min_value: 20230101.0,
                    max_value: 20231231.0,
                },
            }],
            postprocessing: None,
        },
        TemplateType::Ocean => JobConfig {
            nc_key: "ocean_temperature.nc".to_string(),
            variable_name: "sea_surface_temperature".to_string(),
            variable_names: None,
            merge_variable_names: None,
            output: OutputTarget::Parquet {
                parquet_key: "sst_analysis.parquet".to_string(),
                output: None,
            },
            filters: vec![FilterConfig::Range {
                params: RangeParams {
                    dimension_name: "depth".to_string(),
                    min_value: 0.0,
                    max_value: 10.0,
                },
            }],
            postprocessing: None,
        },
    };

    match format {
        ConfigFormat::Json => {
            serde_json::to_string_pretty(&config).context("Failed to serialize template to JSON")
        }
        ConfigFormat::Yaml => {
            serde_yaml::to_string(&config).context("Failed to serialize template to YAML")
        }
    }
}

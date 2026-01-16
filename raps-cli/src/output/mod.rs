use anyhow::Result;
use clap::ValueEnum;
use serde::Serialize;
use std::fmt::Display;
use std::io::IsTerminal;
use std::str::FromStr;

pub mod formatter;
use crate::output::formatter::OutputFormatter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Yaml,
    Csv,
    Plain,
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Yaml => write!(f, "yaml"),
            OutputFormat::Csv => write!(f, "csv"),
            OutputFormat::Plain => write!(f, "plain"),
        }
    }
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(OutputFormat::Table),
            "json" => Ok(OutputFormat::Json),
            "yaml" => Ok(OutputFormat::Yaml),
            "csv" => Ok(OutputFormat::Csv),
            "plain" => Ok(OutputFormat::Plain),
            _ => Err(format!("Invalid format: {}", s)),
        }
    }
}

impl OutputFormat {
    /// Determine the output format to use based on the user's choice and the environment.
    pub fn determine(explicit: Option<OutputFormat>) -> Self {
        if let Some(format) = explicit {
            return format;
        }

        // Check environment variable
        if let Ok(env_format) = std::env::var("RAPS_OUTPUT_FORMAT")
            && let Ok(format) = <OutputFormat as FromStr>::from_str(&env_format)
        {
            return format;
        }

        if !std::io::stdout().is_terminal() {
            return OutputFormat::Json;
        }

        OutputFormat::Table
    }

    pub fn supports_colors(&self) -> bool {
        matches!(self, OutputFormat::Table)
    }

    pub fn write<T: Serialize>(&self, data: &T) -> Result<()> {
        let mut stdout = std::io::stdout();
        OutputFormatter::print_output(data, *self, &mut stdout)
    }

    pub fn write_message(&self, message: &str) -> Result<()> {
        match self {
            OutputFormat::Table | OutputFormat::Plain => {
                println!("{}", message);
                Ok(())
            }
            OutputFormat::Json => {
                #[derive(Serialize)]
                struct Message {
                    message: String,
                }
                self.write(&Message {
                    message: message.to_string(),
                })
            }
            OutputFormat::Yaml => {
                #[derive(Serialize)]
                struct Message {
                    message: String,
                }
                self.write(&Message {
                    message: message.to_string(),
                })
            }
            OutputFormat::Csv => {
                println!("{}", message);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests;

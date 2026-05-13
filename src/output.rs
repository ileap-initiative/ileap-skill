use crate::cli::OutputFormat;
use serde_json::Value;

pub fn print_value(value: &Value, format: &OutputFormat) {
    match format {
        OutputFormat::Pretty => {
            println!("{}", serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()))
        }
        OutputFormat::Compact => println!("{value}"),
    }
}

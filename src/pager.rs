use anyhow::Result;
use serde_json::Value;

use crate::cli::OutputFormat;
use crate::output;
use crate::tty::prompt;

pub fn item_count(value: &Value) -> usize {
    match value {
        Value::Object(obj) => obj
            .get("data")
            .and_then(|d| d.as_array())
            .map(|a| a.len())
            .unwrap_or(0),
        Value::Array(arr) => arr.len(),
        _ => 0,
    }
}

pub fn print_page(value: &Value, limit: Option<u32>, output: &OutputFormat) -> Result<bool> {
    output::print_value(value, output);
    let at_boundary = limit.is_some_and(|l| item_count(value) == l as usize);
    if at_boundary {
        let answer = prompt("Next page? [y/N] ")?;
        Ok(matches!(answer.to_lowercase().as_str(), "y" | "yes"))
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn item_count_object_format() {
        assert_eq!(item_count(&json!({"data": [1, 2, 3]})), 3);
    }

    #[test]
    fn item_count_array_format() {
        assert_eq!(item_count(&json!([1, 2])), 2);
    }

    #[test]
    fn item_count_empty_object() {
        assert_eq!(item_count(&json!({"data": []})), 0);
    }

    #[test]
    fn item_count_non_data_value() {
        assert_eq!(item_count(&json!("irrelevant")), 0);
    }
}

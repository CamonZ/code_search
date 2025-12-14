//! Helper utilities for query building.
//!
//! Common functions used across query builders to reduce boilerplate
//! and ensure consistent formatting.

use std::collections::BTreeMap;

/// Format a list of field names as a comma-separated string.
///
/// Used for SELECT clauses and similar constructs.
pub fn format_fields(fields: &[&str]) -> String {
    fields.join(", ")
}

/// Format a list of field references with a prefix (e.g., "m.field").
///
/// Used in AGE/Cypher queries to reference node properties.
pub fn format_fields_with_prefix(fields: &[&str], prefix: &str) -> String {
    fields
        .iter()
        .map(|f| format!("{}.{}", prefix, f))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Format a list of filter expressions with a joiner.
///
/// Joins filters with AND/OR operators and proper formatting.
pub fn format_filters(filters: &[(String, String)], joiner: &str) -> String {
    filters
        .iter()
        .map(|(expr, _)| expr.clone())
        .collect::<Vec<_>>()
        .join(joiner)
}

/// Build a field binding list for Cozo queries.
///
/// Cozo relations need field bindings like `{field1, field2}`.
pub fn format_cozo_bindings(fields: &[&str]) -> String {
    format!("{{{}}}", fields.join(", "))
}

/// Format a parameter map as debug output.
///
/// Useful for logging query parameters during development.
pub fn format_params_debug(params: &BTreeMap<String, cozo::DataValue>) -> String {
    let items: Vec<String> = params
        .iter()
        .map(|(k, v)| format!("{}={:?}", k, v))
        .collect();
    format!("{{{}}}", items.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_fields_single() {
        let result = format_fields(&["name"]);
        assert_eq!(result, "name");
    }

    #[test]
    fn test_format_fields_multiple() {
        let result = format_fields(&["module", "name", "arity"]);
        assert_eq!(result, "module, name, arity");
    }

    #[test]
    fn test_format_fields_with_prefix_single() {
        let result = format_fields_with_prefix(&["name"], "m");
        assert_eq!(result, "m.name");
    }

    #[test]
    fn test_format_fields_with_prefix_multiple() {
        let result = format_fields_with_prefix(&["module", "name"], "node");
        assert_eq!(result, "node.module, node.name");
    }

    #[test]
    fn test_format_filters_and() {
        let filters = vec![
            ("x == $x".to_string(), "$x".to_string()),
            ("y > $y".to_string(), "$y".to_string()),
        ];
        let result = format_filters(&filters, " AND ");
        assert_eq!(result, "x == $x AND y > $y");
    }

    #[test]
    fn test_format_filters_comma() {
        let filters = vec![
            ("a".to_string(), "".to_string()),
            ("b".to_string(), "".to_string()),
            ("c".to_string(), "".to_string()),
        ];
        let result = format_filters(&filters, ", ");
        assert_eq!(result, "a, b, c");
    }

    #[test]
    fn test_format_cozo_bindings() {
        let result = format_cozo_bindings(&["field1", "field2"]);
        assert_eq!(result, "{field1, field2}");
    }

    #[test]
    fn test_format_cozo_bindings_single() {
        let result = format_cozo_bindings(&["name"]);
        assert_eq!(result, "{name}");
    }

    #[test]
    fn test_format_params_debug_empty() {
        let params = BTreeMap::new();
        let result = format_params_debug(&params);
        assert_eq!(result, "{}");
    }

    #[test]
    fn test_format_params_debug_with_values() {
        let mut params = BTreeMap::new();
        params.insert(
            "x".to_string(),
            cozo::DataValue::Num(cozo::Num::Int(42)),
        );
        params.insert(
            "y".to_string(),
            cozo::DataValue::Str("test".into()),
        );

        let result = format_params_debug(&params);
        // Note: BTreeMap preserves order, so x comes before y
        assert!(result.contains("x="));
        assert!(result.contains("y="));
        assert!(result.starts_with("{"));
        assert!(result.ends_with("}"));
    }
}

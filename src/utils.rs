//! Utility functions for code search operations.

use std::collections::BTreeMap;
use crate::types::ModuleGroup;

/// Builds SQL WHERE clause conditions for query patterns (exact or regex matching)
///
/// Handles the common pattern of building conditions that differ between exact and regex modes.
/// Supports different field prefixes and optional leading comma.
///
/// # Examples
///
/// ```ignore
/// let builder = ConditionBuilder::new("module", "module_pattern");
/// let cond = builder.build(false); // "module == $module_pattern"
/// let cond = builder.build(true);  // "regex_matches(module, $module_pattern)"
/// ```
pub struct ConditionBuilder {
    field_name: String,
    param_name: String,
    with_leading_comma: bool,
}

impl ConditionBuilder {
    /// Creates a new condition builder for a field with exact/regex matching
    ///
    /// # Arguments
    /// * `field_name` - The SQL field name (e.g., "module", "caller_module")
    /// * `param_name` - The parameter name (e.g., "module_pattern", "function_pattern")
    pub fn new(field_name: &str, param_name: &str) -> Self {
        Self {
            field_name: field_name.to_string(),
            param_name: param_name.to_string(),
            with_leading_comma: false,
        }
    }

    /// Adds a leading comma to the condition (useful for mid-query conditions)
    pub fn with_leading_comma(mut self) -> Self {
        self.with_leading_comma = true;
        self
    }

    /// Builds the condition string based on use_regex flag
    ///
    /// When `use_regex` is true, uses `regex_matches()`.
    /// When `use_regex` is false, uses exact matching with `==`.
    ///
    /// # Arguments
    /// * `use_regex` - Whether to use regex matching
    ///
    /// # Returns
    /// A condition string ready to be interpolated into a SQL query
    pub fn build(&self, use_regex: bool) -> String {
        let prefix = if self.with_leading_comma { ", " } else { "" };

        if use_regex {
            format!(
                "{}regex_matches({}, ${})",
                prefix, self.field_name, self.param_name
            )
        } else {
            format!("{}{} == ${}", prefix, self.field_name, self.param_name)
        }
    }
}

/// Builder for optional SQL conditions (function, arity, etc.)
///
/// Handles the pattern of generating conditions only when values are present.
/// For function-matching conditions, supports both exact and regex matching.
pub struct OptionalConditionBuilder {
    field_name: String,
    param_name: String,
    with_leading_comma: bool,
    when_none: Option<String>, // Alternative condition when value is None
    supports_regex: bool, // Whether to use regex_matches when value is present
}

impl OptionalConditionBuilder {
    /// Creates a new optional condition builder
    ///
    /// # Arguments
    /// * `field_name` - The SQL field name
    /// * `param_name` - The parameter name
    pub fn new(field_name: &str, param_name: &str) -> Self {
        Self {
            field_name: field_name.to_string(),
            param_name: param_name.to_string(),
            with_leading_comma: false,
            when_none: None,
            supports_regex: false,
        }
    }

    /// Enables regex matching (uses regex_matches when value is present)
    pub fn with_regex(mut self) -> Self {
        self.supports_regex = true;
        self
    }

    /// Adds a leading comma
    pub fn with_leading_comma(mut self) -> Self {
        self.with_leading_comma = true;
        self
    }

    /// Sets an alternative condition when the value is None (e.g., "true" for no-op)
    pub fn when_none(mut self, condition: &str) -> Self {
        self.when_none = Some(condition.to_string());
        self
    }

    /// Builds the condition string
    ///
    /// # Arguments
    /// * `has_value` - Whether the optional value is present
    /// * `use_regex` - Whether to use regex matching (only matters if supports_regex is true)
    ///
    /// # Returns
    /// A condition string, or empty string if no value and no alternative
    pub fn build_with_regex(&self, has_value: bool, use_regex: bool) -> String {
        let prefix = if self.with_leading_comma { ", " } else { "" };

        if has_value {
            if self.supports_regex && use_regex {
                format!(
                    "{}regex_matches({}, ${})",
                    prefix, self.field_name, self.param_name
                )
            } else {
                format!("{}{} == ${}", prefix, self.field_name, self.param_name)
            }
        } else {
            self.when_none
                .as_ref()
                .map(|cond| format!("{}{}", prefix, cond))
                .unwrap_or_default()
        }
    }

    /// Builds the condition string (non-regex version, for backward compatibility)
    ///
    /// # Arguments
    /// * `has_value` - Whether the optional value is present
    ///
    /// # Returns
    /// A condition string, or empty string if no value and no alternative
    pub fn build(&self, has_value: bool) -> String {
        self.build_with_regex(has_value, false)
    }
}

/// Groups items by module into a structured result
///
/// Transforms a vector of source items into (module, entry) tuples and groups them by module
/// using BTreeMap for consistent ordering. Files default to empty string.
///
/// # Arguments
/// * `items` - Vector of items to transform and group
/// * `transform` - Closure that converts source items to (module_name, entry) tuples
///
/// # Returns
/// A vector of ModuleGroup structs, one per module in sorted order
pub fn group_by_module<T, E, F>(items: Vec<T>, transform: F) -> Vec<ModuleGroup<E>>
where
    F: Fn(T) -> (String, E),
{
    group_by_module_with_file(items, |item| {
        let (module, entry) = transform(item);
        (module, entry, String::new())
    })
}

/// Groups items by module with optional file tracking
///
/// Like `group_by_module` but allows specifying a file path for each item.
///
/// # Arguments
/// * `items` - Vector of items to transform and group
/// * `transform` - Closure that converts source items to (module_name, entry, file) tuples
///
/// # Returns
/// A vector of ModuleGroup structs, one per module in sorted order
pub fn group_by_module_with_file<T, E, F>(items: Vec<T>, transform: F) -> Vec<ModuleGroup<E>>
where
    F: Fn(T) -> (String, E, String),
{
    let mut module_map: BTreeMap<String, (String, Vec<E>)> = BTreeMap::new();

    for item in items {
        let (module, entry, file) = transform(item);
        let entry_data = module_map
            .entry(module)
            .or_insert_with(|| (file.clone(), Vec::new()));
        entry_data.1.push(entry);
    }

    module_map
        .into_iter()
        .map(|(name, (file, entries))| ModuleGroup { name, file, entries })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_by_module_empty() {
        let items: Vec<(String, i32)> = vec![];
        let result = group_by_module(items, |(module, item)| (module, item));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_group_by_module_single_module() {
        let items = vec![
            ("math".to_string(), 1),
            ("math".to_string(), 2),
            ("math".to_string(), 3),
        ];
        let result = group_by_module(items, |(module, item)| (module, item));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "math");
        assert_eq!(result[0].entries.len(), 3);
    }

    #[test]
    fn test_group_by_module_multiple_modules() {
        let items = vec![
            ("math".to_string(), 1),
            ("string".to_string(), 2),
            ("math".to_string(), 3),
            ("list".to_string(), 4),
            ("string".to_string(), 5),
        ];
        let result = group_by_module(items, |(module, item)| (module, item));
        assert_eq!(result.len(), 3);
        // Verify sorted order (BTreeMap sorts)
        assert_eq!(result[0].name, "list");
        assert_eq!(result[1].name, "math");
        assert_eq!(result[2].name, "string");
        // Verify items are grouped correctly
        assert_eq!(result[1].entries.len(), 2); // math has 2 items
        assert_eq!(result[2].entries.len(), 2); // string has 2 items
    }

    #[test]
    fn test_condition_builder_exact_match() {
        let builder = ConditionBuilder::new("module", "module_pattern");
        assert_eq!(builder.build(false), "module == $module_pattern");
    }

    #[test]
    fn test_condition_builder_regex_match() {
        let builder = ConditionBuilder::new("module", "module_pattern");
        assert_eq!(builder.build(true), "regex_matches(module, $module_pattern)");
    }

    #[test]
    fn test_condition_builder_with_leading_comma() {
        let builder = ConditionBuilder::new("module", "module_pattern").with_leading_comma();
        assert_eq!(builder.build(false), ", module == $module_pattern");
        assert_eq!(builder.build(true), ", regex_matches(module, $module_pattern)");
    }

    #[test]
    fn test_optional_condition_builder_with_value() {
        let builder = OptionalConditionBuilder::new("arity", "arity");
        assert_eq!(builder.build(true), "arity == $arity");
    }

    #[test]
    fn test_optional_condition_builder_without_value() {
        let builder = OptionalConditionBuilder::new("arity", "arity");
        assert_eq!(builder.build(false), "");
    }

    #[test]
    fn test_optional_condition_builder_with_default() {
        let builder = OptionalConditionBuilder::new("arity", "arity").when_none("true");
        assert_eq!(builder.build(false), "true");
    }

    #[test]
    fn test_optional_condition_builder_with_leading_comma() {
        let builder = OptionalConditionBuilder::new("arity", "arity")
            .with_leading_comma()
            .when_none("true");
        assert_eq!(builder.build(true), ", arity == $arity");
        assert_eq!(builder.build(false), ", true");
    }
}

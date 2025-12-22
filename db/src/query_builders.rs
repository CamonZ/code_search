//! Query condition builders for CozoScript

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

#[cfg(test)]
mod tests {
    use super::*;

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

//! Query condition builders for CozoScript
//!
//! # Regex Validation Strategy
//!
//! This module validates regex patterns using the standard Rust `regex` crate before
//! passing them to CozoDB. While this means patterns are compiled twice (once during
//! validation, once by CozoDB during query execution), this is an intentional design
//! decision that provides significant benefits:
//!
//! - **Same Engine**: CozoDB uses `regex = "1.10.4"` (the same crate we use), so
//!   validation results perfectly match CozoDB's behavior. There are no false positives
//!   or negatives due to engine differences.
//!
//! - **Better UX**: Early validation at the CLI boundary provides clear, actionable error
//!   messages. Without this, users would get cryptic CozoDB query errors that are harder
//!   to understand and debug.
//!
//! - **Acceptable Cost**: Regex compilation is fast (~1ms per pattern), making the
//!   performance overhead negligible compared to the UX improvement.
//!
//! See: https://github.com/cozodb/cozo/blob/main/cozo-core/Cargo.toml for CozoDB's
//! regex dependency version.

use std::error::Error;

/// Validates a regex pattern string
///
/// # Arguments
/// * `pattern` - The regex pattern to validate
///
/// # Returns
/// * `Ok(())` if the pattern is valid
/// * `Err` with a user-friendly error message if the pattern is invalid
///
/// # Examples
/// ```
/// use db::query_builders::validate_regex_pattern;
///
/// assert!(validate_regex_pattern("^hello.*world$").is_ok());
/// assert!(validate_regex_pattern("[invalid").is_err());
/// ```
pub fn validate_regex_pattern(pattern: &str) -> Result<(), Box<dyn Error>> {
    regex::Regex::new(pattern).map_err(|e| -> Box<dyn Error> {
        format!(
            "Invalid regex pattern '{}': {}",
            pattern,
            e.to_string()
        )
        .into()
    })?;
    Ok(())
}

/// Validates multiple regex patterns at once (only if regex mode is enabled)
///
/// This is a convenience helper for query functions that accept multiple optional
/// patterns. It validates all patterns only when `use_regex` is true.
///
/// # Arguments
/// * `use_regex` - Whether regex mode is enabled
/// * `patterns` - Slice of optional pattern strings to validate
///
/// # Returns
/// * `Ok(())` if all patterns are valid (or if `use_regex` is false)
/// * `Err` with a user-friendly error message if any pattern is invalid
///
/// # Examples
/// ```
/// use db::query_builders::validate_regex_patterns;
///
/// // Non-regex mode: accepts any patterns (no validation)
/// assert!(validate_regex_patterns(false, &[Some("[invalid")]).is_ok());
///
/// // Regex mode: validates all patterns
/// assert!(validate_regex_patterns(true, &[Some("^hello$"), Some("world.*")]).is_ok());
/// assert!(validate_regex_patterns(true, &[Some("[invalid")]).is_err());
///
/// // None patterns are skipped
/// assert!(validate_regex_patterns(true, &[Some("valid"), None, Some("also.*valid")]).is_ok());
/// ```
pub fn validate_regex_patterns(
    use_regex: bool,
    patterns: &[Option<&str>],
) -> Result<(), Box<dyn Error>> {
    if !use_regex {
        return Ok(());
    }

    for pattern_opt in patterns {
        if let Some(pattern) = pattern_opt {
            validate_regex_pattern(pattern)?;
        }
    }

    Ok(())
}

/// Builds SQL WHERE clause conditions for query patterns (exact or regex matching)
///
/// Handles the common pattern of building conditions that differ between exact and regex modes.
/// Supports different field prefixes and optional leading comma.
///
/// # Examples
///
/// ```
/// use db::query_builders::ConditionBuilder;
///
/// let builder = ConditionBuilder::new("module", "module_pattern");
/// let cond = builder.build(false); // "module == $module_pattern"
/// let cond = builder.build(true);  // "regex_matches(module, $module_pattern)"
/// ```
pub struct ConditionBuilder {
    field_name: &'static str,
    param_name: &'static str,
    with_leading_comma: bool,
}

impl ConditionBuilder {
    /// Creates a new condition builder for a field with exact/regex matching
    ///
    /// # Arguments
    /// * `field_name` - The SQL field name (e.g., "module", "caller_module")
    /// * `param_name` - The parameter name (e.g., "module_pattern", "function_pattern")
    pub fn new(field_name: &'static str, param_name: &'static str) -> Self {
        Self {
            field_name,
            param_name,
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
    field_name: &'static str,
    param_name: &'static str,
    with_leading_comma: bool,
    when_none: Option<&'static str>, // Alternative condition when value is None
    supports_regex: bool, // Whether to use regex_matches when value is present
}

impl OptionalConditionBuilder {
    /// Creates a new optional condition builder
    ///
    /// # Arguments
    /// * `field_name` - The SQL field name
    /// * `param_name` - The parameter name
    pub fn new(field_name: &'static str, param_name: &'static str) -> Self {
        Self {
            field_name,
            param_name,
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
    pub fn when_none(mut self, condition: &'static str) -> Self {
        self.when_none = Some(condition);
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

    // =========================================================================
    // Regex validation tests
    // =========================================================================

    #[test]
    fn test_validate_regex_pattern_valid() {
        // Simple patterns
        assert!(validate_regex_pattern("hello").is_ok());
        assert!(validate_regex_pattern("^start").is_ok());
        assert!(validate_regex_pattern("end$").is_ok());

        // Character classes
        assert!(validate_regex_pattern("[abc]").is_ok());
        assert!(validate_regex_pattern("[a-z]").is_ok());
        assert!(validate_regex_pattern("[^abc]").is_ok());

        // Quantifiers
        assert!(validate_regex_pattern("a*").is_ok());
        assert!(validate_regex_pattern("a+").is_ok());
        assert!(validate_regex_pattern("a?").is_ok());
        assert!(validate_regex_pattern("a{2,4}").is_ok());

        // Groups and alternation
        assert!(validate_regex_pattern("(foo|bar)").is_ok());
        assert!(validate_regex_pattern("(?:non-capturing)").is_ok());

        // Common real-world patterns
        assert!(validate_regex_pattern(r"^get_user$").is_ok());
        assert!(validate_regex_pattern(r"\.(Accounts|Users)$").is_ok());
        assert!(validate_regex_pattern(r"MyApp\..*\.Service$").is_ok());
    }

    #[test]
    fn test_validate_regex_pattern_invalid() {
        // Unclosed brackets
        let err = validate_regex_pattern("[invalid").unwrap_err();
        assert!(err.to_string().contains("Invalid regex pattern"));
        assert!(err.to_string().contains("[invalid"));

        // Unclosed parenthesis
        let err = validate_regex_pattern("(unclosed").unwrap_err();
        assert!(err.to_string().contains("Invalid regex pattern"));

        // Invalid repetition
        let err = validate_regex_pattern("*invalid").unwrap_err();
        assert!(err.to_string().contains("Invalid regex pattern"));

        // Invalid escape
        let err = validate_regex_pattern(r"\k").unwrap_err();
        assert!(err.to_string().contains("Invalid regex pattern"));

        // Invalid quantifier
        let err = validate_regex_pattern("a{,}").unwrap_err();
        assert!(err.to_string().contains("Invalid regex pattern"));
    }

    #[test]
    fn test_validate_regex_pattern_empty() {
        // Empty pattern is valid (matches everything)
        assert!(validate_regex_pattern("").is_ok());
    }

    #[test]
    fn test_validate_regex_pattern_error_message_format() {
        let err = validate_regex_pattern("[unclosed").unwrap_err();
        let msg = err.to_string();

        // Should contain the pattern itself
        assert!(msg.contains("[unclosed"), "Error should show the pattern: {}", msg);

        // Should contain "Invalid regex pattern"
        assert!(msg.contains("Invalid regex pattern"), "Error should say 'Invalid regex pattern': {}", msg);
    }

    // =========================================================================
    // Regex patterns validation helper tests
    // =========================================================================

    #[test]
    fn test_validate_regex_patterns_non_regex_mode() {
        // Non-regex mode should accept any patterns without validation
        assert!(validate_regex_patterns(false, &[Some("[invalid")]).is_ok());
        assert!(validate_regex_patterns(false, &[Some("*bad"), Some("(unclosed")]).is_ok());
    }

    #[test]
    fn test_validate_regex_patterns_all_valid() {
        // All valid patterns should succeed
        assert!(validate_regex_patterns(true, &[Some("^hello$"), Some("world.*")]).is_ok());
        assert!(validate_regex_patterns(true, &[Some("test")]).is_ok());
    }

    #[test]
    fn test_validate_regex_patterns_with_invalid() {
        // Should fail on first invalid pattern
        let result = validate_regex_patterns(true, &[Some("valid"), Some("[invalid")]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("[invalid"));
    }

    #[test]
    fn test_validate_regex_patterns_with_none() {
        // None patterns should be skipped
        assert!(validate_regex_patterns(true, &[Some("valid"), None, Some("also.*valid")]).is_ok());
        assert!(validate_regex_patterns(true, &[None, None]).is_ok());
    }

    #[test]
    fn test_validate_regex_patterns_empty() {
        // Empty slice should succeed
        assert!(validate_regex_patterns(true, &[]).is_ok());
    }

    #[test]
    fn test_validate_regex_patterns_fails_on_first_invalid() {
        // Should return error from first invalid pattern, not continue
        let result = validate_regex_patterns(true, &[Some("[first_bad"), Some("(second_bad")]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Should show first invalid pattern
        assert!(err.to_string().contains("[first_bad"));
    }

    #[test]
    fn test_validate_regex_patterns_mixed_valid_and_none() {
        // Mix of valid patterns and None should work
        assert!(validate_regex_patterns(true, &[
            Some("^test.*$"),
            None,
            Some("[a-z]+"),
            None,
            Some("\\d{3}")
        ]).is_ok());
    }
}

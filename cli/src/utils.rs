//! Utility functions for code search CLI output and presentation.

use std::borrow::Cow;
use std::collections::BTreeMap;
use regex::Regex;
use db::types::{ModuleGroup, Call};
use crate::dedup::sort_and_deduplicate;

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
        .map(|(name, (file, entries))| ModuleGroup { name, file, entries, function_count: None })
        .collect()
}

/// Groups calls by module and function key, applying sort/deduplicate to each group.
///
/// This is the primary helper for processing call data that follows this pattern:
/// 1. Receive Vec<Call> from a query
/// 2. Group by module and function key using closures
/// 3. Apply sort_and_deduplicate to each function's calls
/// 4. Convert to ModuleGroupResult using entry_fn and file_fn
///
/// # Arguments
/// * `calls` - Vector of Call objects to group
/// * `module_fn` - Closure that extracts the module name from a Call
/// * `key_fn` - Closure that extracts the grouping key (e.g., function info) from a Call
/// * `sort_cmp` - Comparator closure for sorting calls (e.g., by line number)
/// * `dedup_key` - Closure that extracts the deduplication key from a Call
/// * `entry_fn` - Closure that converts (key, sorted/deduped calls) to an entry
/// * `file_fn` - Closure that determines the file path for a module group
///
/// # Returns
/// A tuple of (total_items_count, Vec<ModuleGroup<E>>)
///
/// # Example
/// ```ignore
/// let (total, groups) = group_calls(
///     calls,
///     |call| call.caller.module.clone(),  // group by caller module
///     |call| (call.caller.name.clone(), call.caller.arity),  // key by (name, arity)
///     |a, b| a.line.cmp(&b.line),  // sort by line
///     |c| (c.callee.module.clone(), c.callee.name.clone()),  // dedup by callee
///     |(name, arity), calls| MyEntry { name, arity, calls },  // build entry
///     |_module, _map| String::new(),  // no file tracking
/// );
/// ```
pub fn group_calls<K, E, MF, KF, SC, DK, D, EF, FF>(
    calls: Vec<Call>,
    module_fn: MF,
    key_fn: KF,
    sort_cmp: SC,
    dedup_key: DK,
    entry_fn: EF,
    file_fn: FF,
) -> (usize, Vec<ModuleGroup<E>>)
where
    K: Ord,
    MF: Fn(&Call) -> String,
    KF: Fn(&Call) -> K,
    SC: FnMut(&Call, &Call) -> std::cmp::Ordering + Clone,
    DK: Fn(&Call) -> D + Clone,
    D: Eq + std::hash::Hash,
    EF: Fn(K, Vec<Call>) -> E,
    FF: Fn(&str, &BTreeMap<K, Vec<Call>>) -> String,
{
    // Group by module -> key -> calls
    let mut by_module: BTreeMap<String, BTreeMap<K, Vec<Call>>> = BTreeMap::new();
    for call in calls {
        let module = module_fn(&call);
        let key = key_fn(&call);
        by_module.entry(module).or_default().entry(key).or_default().push(call);
    }

    // Convert to ModuleGroups with sort/dedup, counting total after dedup
    let mut total_items = 0;
    let items = by_module.into_iter().map(|(module_name, mut functions_map)| {
        let file = file_fn(&module_name, &functions_map);

        // Sort and deduplicate each function's calls
        for calls in functions_map.values_mut() {
            sort_and_deduplicate(calls, sort_cmp.clone(), dedup_key.clone());
            total_items += calls.len();
        }

        let entries: Vec<E> = functions_map.into_iter()
            .map(|(key, calls)| entry_fn(key, calls))
            .collect();

        ModuleGroup { name: module_name, file, entries, function_count: None }
    }).collect();

    (total_items, items)
}

/// Converts a two-level nested map into Vec<ModuleGroup<E>>.
///
/// Handles the common pattern of grouping calls by module and function,
/// then converting the nested structure into a flat Vec of ModuleGroups.
///
/// # Arguments
/// * `by_module` - A BTreeMap of modules to (function_key → calls) maps
/// * `entry_builder` - Closure that converts (function_key, calls) to an entry
/// * `file_strategy` - Closure that determines the file path for a module group
///
/// # Returns
/// A vector of ModuleGroup structs, one per module in sorted order
///
/// # Example
/// ```ignore
/// let mut by_module: BTreeMap<String, BTreeMap<(String, i64), Vec<Call>>> = /* ... */;
/// let groups = convert_to_module_groups(
///     by_module,
///     |(name, arity), calls| {
///         CallEntry {
///             function_name: name,
///             arity,
///             count: calls.len(),
///         }
///     },
///     |_module, _map| String::new()  // No file tracking
/// );
/// ```
pub fn convert_to_module_groups<FK, E, F, FileF>(
    by_module: BTreeMap<String, BTreeMap<FK, Vec<Call>>>,
    entry_builder: F,
    file_strategy: FileF,
) -> Vec<ModuleGroup<E>>
where
    FK: Ord,
    F: Fn(FK, Vec<Call>) -> E,
    FileF: Fn(&str, &BTreeMap<FK, Vec<Call>>) -> String,
{
    by_module
        .into_iter()
        .map(|(module_name, functions_map)| {
            let file = file_strategy(&module_name, &functions_map);

            let entries: Vec<E> = functions_map
                .into_iter()
                .map(|(key, calls)| entry_builder(key, calls))
                .collect();

            ModuleGroup {
                name: module_name,
                file,
                entries,
                function_count: None,
            }
        })
        .collect()
}

// =============================================================================
// Type Formatting Utilities
// =============================================================================

/// Formats an Elixir type definition for display.
///
/// Transforms struct type definitions from the internal representation:
/// `@type t() :: %{__struct__: ModuleName, field1: type1, field2: type2}`
///
/// To the more readable Elixir syntax:
/// ```text
/// @type t() :: %ModuleName{
///   field1: type1,
///   field2: type2
/// }
/// ```
///
/// # Arguments
/// * `definition` - The raw type definition string from the database
///
/// # Returns
/// The formatted type definition (borrowed if unchanged, owned if formatted)
pub fn format_type_definition(definition: &str) -> Cow<str> {
    // Check if this is a struct type definition
    if let Some(formatted) = try_format_struct_type(definition) {
        return Cow::Owned(formatted);
    }

    // Return as-is if no transformation needed
    Cow::Borrowed(definition)
}

/// Attempts to format a struct type definition.
///
/// Returns `Some(formatted_string)` if the definition contains a struct pattern,
/// otherwise returns `None`.
fn try_format_struct_type(definition: &str) -> Option<String> {
    // Pattern to match: %{__struct__: ModuleName} or %{__struct__: ModuleName, ...}
    // This captures the struct module name and optionally the remaining fields
    let struct_pattern = Regex::new(
        r"%\{\s*__struct__:\s*([A-Za-z][A-Za-z0-9_.]*(?:\.[A-Za-z][A-Za-z0-9_]*)*)\s*(?:,\s*(.*))?\}"
    ).ok()?;

    if let Some(caps) = struct_pattern.captures(definition) {
        let module_name = caps.get(1)?.as_str();
        let fields_str = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");

        // Parse the fields
        let fields = parse_type_fields(fields_str);

        if fields.is_empty() {
            // Empty struct
            let formatted_struct = format!("%{}{{}}", module_name);
            return Some(definition.replace(caps.get(0)?.as_str(), &formatted_struct));
        }

        // Format with multi-line for readability
        let formatted_fields = fields
            .iter()
            .map(|(name, typ)| format!("  {}: {}", name, typ))
            .collect::<Vec<_>>()
            .join(",\n");

        let formatted_struct = format!("%{}{{\n{}\n}}", module_name, formatted_fields);

        // Replace the struct pattern in the original definition
        Some(definition.replace(caps.get(0)?.as_str(), &formatted_struct))
    } else {
        None
    }
}

/// Parses a comma-separated list of type fields.
///
/// Handles nested types with parentheses, braces, and brackets.
/// For example: `name: String.t(), list: list(integer()), map: map()`
///
/// # Arguments
/// * `fields_str` - The raw fields string without outer braces
///
/// # Returns
/// A vector of (field_name, field_type) tuples
fn parse_type_fields(fields_str: &str) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    let mut current_field = String::new();
    let mut depth = 0; // Track nesting depth for (), {}, []

    for ch in fields_str.chars() {
        match ch {
            '(' | '{' | '[' => {
                depth += 1;
                current_field.push(ch);
            }
            ')' | '}' | ']' => {
                depth -= 1;
                current_field.push(ch);
            }
            ',' if depth == 0 => {
                // Top-level comma - this is a field separator
                if let Some((name, typ)) = parse_single_field(&current_field) {
                    fields.push((name, typ));
                }
                current_field.clear();
            }
            _ => {
                current_field.push(ch);
            }
        }
    }

    // Don't forget the last field
    if let Some((name, typ)) = parse_single_field(&current_field) {
        fields.push((name, typ));
    }

    fields
}

/// Parses a single field definition like "name: String.t()" or "count: integer()".
///
/// # Arguments
/// * `field_str` - A single field definition string
///
/// # Returns
/// `Some((field_name, field_type))` if parsing succeeds, `None` otherwise
fn parse_single_field(field_str: &str) -> Option<(String, String)> {
    let trimmed = field_str.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Find the first colon that separates field name from type
    let colon_pos = trimmed.find(':')?;
    let name = trimmed[..colon_pos].trim().to_string();
    let typ = trimmed[colon_pos + 1..].trim().to_string();

    if name.is_empty() || typ.is_empty() {
        return None;
    }

    Some((name, typ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::types::{Call, FunctionRef};

    // =========================================================================
    // Helper to build Call fixtures
    // =========================================================================

    fn make_call(
        caller_module: &str,
        caller_name: &str,
        caller_arity: i64,
        callee_module: &str,
        callee_name: &str,
        callee_arity: i64,
        line: i64,
    ) -> Call {
        Call {
            caller: FunctionRef::new(caller_module, caller_name, caller_arity),
            callee: FunctionRef::new(callee_module, callee_name, callee_arity),
            line,
            call_type: None,
            depth: None,
        }
    }

    // =========================================================================
    // parse_single_field tests
    // =========================================================================

    #[test]
    fn test_parse_single_field_valid() {
        let result = parse_single_field("name: String.t()");
        assert_eq!(
            result,
            Some(("name".to_string(), "String.t()".to_string()))
        );
    }

    #[test]
    fn test_parse_single_field_empty_string() {
        assert_eq!(parse_single_field(""), None);
    }

    #[test]
    fn test_parse_single_field_whitespace_only() {
        assert_eq!(parse_single_field("   "), None);
    }

    #[test]
    fn test_parse_single_field_no_colon() {
        assert_eq!(parse_single_field("no_colon_here"), None);
    }

    /// Catches the known mutant: `||` replaced with `&&` at line 334.
    /// When name is empty but type is present, it must return None.
    #[test]
    fn test_parse_single_field_empty_name_nonempty_type() {
        // ": integer()" has an empty name before the colon
        assert_eq!(parse_single_field(": integer()"), None);
    }

    /// Catches the known mutant: `||` replaced with `&&` at line 334.
    /// When type is empty but name is present, it must return None.
    #[test]
    fn test_parse_single_field_nonempty_name_empty_type() {
        // "name:" has a name but empty type after the colon
        assert_eq!(parse_single_field("name:"), None);
    }

    #[test]
    fn test_parse_single_field_both_empty() {
        // ":" has both empty name and empty type
        assert_eq!(parse_single_field(":"), None);
    }

    #[test]
    fn test_parse_single_field_trims_whitespace() {
        let result = parse_single_field("  name  :  String.t()  ");
        assert_eq!(
            result,
            Some(("name".to_string(), "String.t()".to_string()))
        );
    }

    #[test]
    fn test_parse_single_field_colon_in_type() {
        // The first colon is the separator; subsequent colons are part of the type
        let result = parse_single_field("key: Keyword.t(:atom)");
        assert_eq!(
            result,
            Some(("key".to_string(), "Keyword.t(:atom)".to_string()))
        );
    }

    // =========================================================================
    // parse_type_fields tests
    // =========================================================================

    #[test]
    fn test_parse_type_fields_empty() {
        let fields = parse_type_fields("");
        assert!(fields.is_empty());
    }

    #[test]
    fn test_parse_type_fields_single_field() {
        let fields = parse_type_fields("name: String.t()");
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0], ("name".to_string(), "String.t()".to_string()));
    }

    #[test]
    fn test_parse_type_fields_simple() {
        let input = "name: String.t(), age: integer()";
        let fields = parse_type_fields(input);

        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0], ("name".to_string(), "String.t()".to_string()));
        assert_eq!(fields[1], ("age".to_string(), "integer()".to_string()));
    }

    #[test]
    fn test_parse_type_fields_with_nested_parens() {
        let input = "list: list(integer()), map: map()";
        let fields = parse_type_fields(input);

        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0], ("list".to_string(), "list(integer())".to_string()));
        assert_eq!(fields[1], ("map".to_string(), "map()".to_string()));
    }

    #[test]
    fn test_parse_type_fields_with_union_types() {
        let input = "status: :ok | :error, reason: String.t() | nil";
        let fields = parse_type_fields(input);

        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0], ("status".to_string(), ":ok | :error".to_string()));
        assert_eq!(fields[1], ("reason".to_string(), "String.t() | nil".to_string()));
    }

    #[test]
    fn test_parse_type_fields_with_nested_braces() {
        let input = "data: %{key: value}, count: integer()";
        let fields = parse_type_fields(input);

        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0], ("data".to_string(), "%{key: value}".to_string()));
        assert_eq!(fields[1], ("count".to_string(), "integer()".to_string()));
    }

    #[test]
    fn test_parse_type_fields_with_nested_brackets() {
        let input = "items: [integer()], name: atom()";
        let fields = parse_type_fields(input);

        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0], ("items".to_string(), "[integer()]".to_string()));
        assert_eq!(fields[1], ("name".to_string(), "atom()".to_string()));
    }

    #[test]
    fn test_parse_type_fields_skips_invalid_fields() {
        // An entry with no colon should be skipped
        let input = "valid_field: integer(), no_colon_here, another: atom()";
        let fields = parse_type_fields(input);

        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].0, "valid_field");
        assert_eq!(fields[1].0, "another");
    }

    // =========================================================================
    // Type formatting tests
    // =========================================================================

    #[test]
    fn test_format_simple_struct_type() {
        let input = "@type t() :: %{__struct__: MyApp.User, name: String.t(), age: integer()}";
        let result = format_type_definition(input);

        assert!(result.contains("%MyApp.User{"));
        assert!(result.contains("name: String.t()"));
        assert!(result.contains("age: integer()"));
    }

    #[test]
    fn test_format_struct_returns_owned() {
        let input = "@type t() :: %{__struct__: MyApp.User, name: String.t()}";
        let result = format_type_definition(input);
        // The result should be an owned (transformed) value
        assert!(matches!(result, Cow::Owned(_)));
    }

    #[test]
    fn test_format_non_struct_returns_borrowed() {
        let input = "@type user_id() :: integer()";
        let result = format_type_definition(input);
        // The result should be borrowed (unchanged)
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn test_format_struct_with_nested_types() {
        let input = "@type t() :: %{__struct__: TradeGym.DataImporter, executions: list(), open_positions: list(), reason: String.t() | nil, status: :ok | :error}";
        let result = format_type_definition(input);

        assert!(result.contains("%TradeGym.DataImporter{"));
        assert!(result.contains("executions: list()"));
        assert!(result.contains("open_positions: list()"));
        assert!(result.contains("reason: String.t() | nil"));
        assert!(result.contains("status: :ok | :error"));
    }

    #[test]
    fn test_format_empty_struct() {
        let input = "@type t() :: %{__struct__: MyApp.Empty}";
        let result = format_type_definition(input);

        // Empty struct should remain compact
        assert!(result.contains("%MyApp.Empty{}"));
    }

    #[test]
    fn test_format_empty_struct_no_fields() {
        let input = "@type t() :: %{__struct__: MyApp.Empty}";
        let result = format_type_definition(input);
        // Should NOT contain newlines since there are no fields
        assert!(!result.contains('\n'));
    }

    #[test]
    fn test_non_struct_type_unchanged() {
        let input = "@type user_id() :: integer()";
        let result = format_type_definition(input);

        assert_eq!(result, input);
    }

    #[test]
    fn test_map_type_unchanged() {
        let input = "@type options() :: %{name: String.t(), age: integer()}";
        let result = format_type_definition(input);

        // Regular maps (without __struct__) should remain unchanged
        assert_eq!(result, input);
    }

    #[test]
    fn test_format_struct_with_complex_types() {
        let input = "@type t() :: %{__struct__: MyApp.State, callbacks: list({atom(), function()}), data: map()}";
        let result = format_type_definition(input);

        assert!(result.contains("%MyApp.State{"));
        assert!(result.contains("callbacks: list({atom(), function()})"));
        assert!(result.contains("data: map()"));
    }

    #[test]
    fn test_opaque_type_unchanged() {
        let input = "@opaque state() :: %{internal: map()}";
        let result = format_type_definition(input);

        assert_eq!(result, input);
    }

    #[test]
    fn test_typep_with_struct() {
        let input = "@typep t() :: %{__struct__: MyApp.Internal, data: term()}";
        let result = format_type_definition(input);

        assert!(result.contains("%MyApp.Internal{"));
        assert!(result.contains("data: term()"));
    }

    #[test]
    fn test_format_struct_multiline_indentation() {
        let input = "@type t() :: %{__struct__: M, a: integer(), b: atom()}";
        let result = format_type_definition(input);
        // Each field should be indented with 2 spaces
        assert!(result.contains("  a: integer()"));
        assert!(result.contains("  b: atom()"));
    }

    #[test]
    fn test_format_struct_with_single_field() {
        let input = "@type t() :: %{__struct__: MyApp.One, field: integer()}";
        let result = format_type_definition(input);

        assert!(result.contains("%MyApp.One{"));
        assert!(result.contains("field: integer()"));
    }

    #[test]
    fn test_try_format_struct_type_returns_none_for_non_struct() {
        assert!(try_format_struct_type("just a plain type").is_none());
    }

    #[test]
    fn test_try_format_struct_type_returns_some_for_struct() {
        let result = try_format_struct_type("%{__struct__: MyApp.Foo, x: integer()}");
        assert!(result.is_some());
        let formatted = result.unwrap();
        assert!(formatted.contains("%MyApp.Foo{"));
    }

    // =========================================================================
    // group_by_module tests
    // =========================================================================

    #[test]
    fn test_group_by_module_empty() {
        let items: Vec<(String, i32)> = vec![];
        let result = group_by_module(items, |(module, item)| (module, item));
        assert!(result.is_empty());
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
        assert_eq!(result[0].entries, vec![1, 2, 3]);
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
        assert_eq!(result[0].entries, vec![4]); // list has 1 item
        assert_eq!(result[1].entries, vec![1, 3]); // math has 2 items
        assert_eq!(result[2].entries, vec![2, 5]); // string has 2 items
    }

    #[test]
    fn test_group_by_module_file_defaults_to_empty() {
        let items = vec![("mod_a".to_string(), 42)];
        let result = group_by_module(items, |(module, item)| (module, item));
        assert_eq!(result[0].file, "");
    }

    #[test]
    fn test_group_by_module_function_count_is_none() {
        let items = vec![("mod_a".to_string(), 42)];
        let result = group_by_module(items, |(module, item)| (module, item));
        assert!(result[0].function_count.is_none());
    }

    #[test]
    fn test_group_by_module_transform_modifies_entries() {
        // The transform can reshape the data
        let items = vec![
            ("m".to_string(), 10),
            ("m".to_string(), 20),
        ];
        let result = group_by_module(items, |(module, val)| (module, val * 2));
        assert_eq!(result[0].entries, vec![20, 40]);
    }

    #[test]
    fn test_group_by_module_transform_assigns_module() {
        // The transform can remap module names
        let items = vec![1, 2, 3];
        let result = group_by_module(items, |val| {
            let module = if val <= 2 { "small" } else { "big" };
            (module.to_string(), val)
        });
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "big");
        assert_eq!(result[0].entries, vec![3]);
        assert_eq!(result[1].name, "small");
        assert_eq!(result[1].entries, vec![1, 2]);
    }

    // =========================================================================
    // group_by_module_with_file tests
    // =========================================================================

    #[test]
    fn test_group_by_module_with_file_empty() {
        let items: Vec<i32> = vec![];
        let result: Vec<ModuleGroup<i32>> =
            group_by_module_with_file(items, |_| ("mod".to_string(), 0, "file".to_string()));
        assert!(result.is_empty());
    }

    #[test]
    fn test_group_by_module_with_file_tracks_file() {
        let items = vec![
            ("mod_a".to_string(), 1, "file_a.ex".to_string()),
        ];
        let result = group_by_module_with_file(items, |(module, item, file)| (module, item, file));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "mod_a");
        assert_eq!(result[0].file, "file_a.ex");
        assert_eq!(result[0].entries, vec![1]);
    }

    #[test]
    fn test_group_by_module_with_file_first_file_wins() {
        // When multiple items share a module, the file from the first item is used
        // (because or_insert_with only inserts on the first encounter)
        let items = vec![
            ("mod_a".to_string(), 1, "first.ex".to_string()),
            ("mod_a".to_string(), 2, "second.ex".to_string()),
        ];
        let result = group_by_module_with_file(items, |(module, item, file)| (module, item, file));
        assert_eq!(result[0].file, "first.ex");
        assert_eq!(result[0].entries, vec![1, 2]);
    }

    #[test]
    fn test_group_by_module_with_file_multiple_modules() {
        let items = vec![
            ("b_mod".to_string(), 1, "b.ex".to_string()),
            ("a_mod".to_string(), 2, "a.ex".to_string()),
            ("b_mod".to_string(), 3, "b.ex".to_string()),
        ];
        let result = group_by_module_with_file(items, |(module, item, file)| (module, item, file));
        assert_eq!(result.len(), 2);
        // BTreeMap sorts alphabetically
        assert_eq!(result[0].name, "a_mod");
        assert_eq!(result[0].file, "a.ex");
        assert_eq!(result[0].entries, vec![2]);
        assert_eq!(result[1].name, "b_mod");
        assert_eq!(result[1].file, "b.ex");
        assert_eq!(result[1].entries, vec![1, 3]);
    }

    #[test]
    fn test_group_by_module_with_file_function_count_is_none() {
        let items = vec![("mod".to_string(), 1, "f.ex".to_string())];
        let result = group_by_module_with_file(items, |(m, i, f)| (m, i, f));
        assert!(result[0].function_count.is_none());
    }

    // =========================================================================
    // group_calls tests
    // =========================================================================

    #[test]
    fn test_group_calls_empty() {
        let calls: Vec<Call> = vec![];
        let (total, groups) = group_calls(
            calls,
            |c| c.caller.module.to_string(),
            |c| c.caller.name.to_string(),
            |a, b| a.line.cmp(&b.line),
            |c| (c.callee.module.to_string(), c.callee.name.to_string()),
            |key, calls| (key, calls.len()),
            |_module, _map| String::new(),
        );
        assert_eq!(total, 0);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_group_calls_single_module_single_key() {
        let calls = vec![
            make_call("ModA", "func1", 1, "ModB", "target", 0, 10),
            make_call("ModA", "func1", 1, "ModC", "other", 0, 20),
        ];
        let (total, groups) = group_calls(
            calls,
            |c| c.caller.module.to_string(),
            |c| c.caller.name.to_string(),
            |a, b| a.line.cmp(&b.line),
            |c| (c.callee.module.to_string(), c.callee.name.to_string()),
            |key, calls| (key, calls.len()),
            |_module, _map| String::new(),
        );
        assert_eq!(total, 2);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "ModA");
        assert_eq!(groups[0].entries.len(), 1); // one key: "func1"
        assert_eq!(groups[0].entries[0], ("func1".to_string(), 2));
    }

    #[test]
    fn test_group_calls_multiple_modules() {
        let calls = vec![
            make_call("ModA", "func1", 1, "Target", "t", 0, 10),
            make_call("ModB", "func2", 1, "Target", "t", 0, 20),
            make_call("ModA", "func1", 1, "Target", "t2", 0, 30),
        ];
        let (total, groups) = group_calls(
            calls,
            |c| c.caller.module.to_string(),
            |c| c.caller.name.to_string(),
            |a, b| a.line.cmp(&b.line),
            |c| (c.callee.module.to_string(), c.callee.name.to_string()),
            |key, calls| (key, calls.len()),
            |_module, _map| String::new(),
        );
        assert_eq!(total, 3);
        assert_eq!(groups.len(), 2);
        // BTreeMap sorts: ModA before ModB
        assert_eq!(groups[0].name, "ModA");
        assert_eq!(groups[1].name, "ModB");
    }

    #[test]
    fn test_group_calls_multiple_keys_per_module() {
        let calls = vec![
            make_call("ModA", "func1", 1, "Target", "t1", 0, 10),
            make_call("ModA", "func2", 2, "Target", "t2", 0, 20),
        ];
        let (total, groups) = group_calls(
            calls,
            |c| c.caller.module.to_string(),
            |c| c.caller.name.to_string(),
            |a, b| a.line.cmp(&b.line),
            |c| c.callee.name.to_string(),
            |key, calls| (key, calls.len()),
            |_module, _map| String::new(),
        );
        assert_eq!(total, 2);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].entries.len(), 2); // two keys: func1, func2
    }

    #[test]
    fn test_group_calls_deduplicates() {
        // Two calls from the same function to the same target should be deduped
        let calls = vec![
            make_call("ModA", "func1", 1, "Target", "same", 0, 10),
            make_call("ModA", "func1", 1, "Target", "same", 0, 20),
        ];
        let (total, groups) = group_calls(
            calls,
            |c| c.caller.module.to_string(),
            |c| c.caller.name.to_string(),
            |a, b| a.line.cmp(&b.line),
            |c| (c.callee.module.to_string(), c.callee.name.to_string()),
            |key, calls| (key, calls.len()),
            |_module, _map| String::new(),
        );
        // After dedup, only 1 remains
        assert_eq!(total, 1);
        assert_eq!(groups[0].entries[0], ("func1".to_string(), 1));
    }

    #[test]
    fn test_group_calls_sorts_by_line() {
        let calls = vec![
            make_call("ModA", "func1", 1, "Target", "t1", 0, 30),
            make_call("ModA", "func1", 1, "Target", "t2", 0, 10),
            make_call("ModA", "func1", 1, "Target", "t3", 0, 20),
        ];
        let (_total, groups) = group_calls(
            calls,
            |c| c.caller.module.to_string(),
            |c| c.caller.name.to_string(),
            |a, b| a.line.cmp(&b.line),
            |c| c.callee.name.to_string(), // unique dedup keys
            |_key, calls| calls.iter().map(|c| c.line).collect::<Vec<_>>(),
            |_module, _map| String::new(),
        );
        // Calls should be sorted by line
        assert_eq!(groups[0].entries[0], vec![10, 20, 30]);
    }

    #[test]
    fn test_group_calls_file_fn_is_used() {
        let calls = vec![
            make_call("ModA", "func1", 1, "Target", "t", 0, 10),
        ];
        let (_total, groups) = group_calls(
            calls,
            |c| c.caller.module.to_string(),
            |c| c.caller.name.to_string(),
            |a, b| a.line.cmp(&b.line),
            |c| c.callee.name.to_string(),
            |key, _calls| key,
            |module, _map| format!("{}.ex", module.to_lowercase()),
        );
        assert_eq!(groups[0].file, "moda.ex");
    }

    #[test]
    fn test_group_calls_function_count_is_none() {
        let calls = vec![
            make_call("ModA", "func1", 1, "Target", "t", 0, 10),
        ];
        let (_total, groups) = group_calls(
            calls,
            |c| c.caller.module.to_string(),
            |c| c.caller.name.to_string(),
            |a, b| a.line.cmp(&b.line),
            |c| c.callee.name.to_string(),
            |key, _calls| key,
            |_module, _map| String::new(),
        );
        assert!(groups[0].function_count.is_none());
    }

    #[test]
    fn test_group_calls_total_counts_after_dedup() {
        // 3 calls, but 2 share a dedup key, so total should be 2
        let calls = vec![
            make_call("ModA", "func1", 1, "Target", "dup", 0, 10),
            make_call("ModA", "func1", 1, "Target", "dup", 0, 20),
            make_call("ModA", "func1", 1, "Target", "unique", 0, 30),
        ];
        let (total, _groups) = group_calls(
            calls,
            |c| c.caller.module.to_string(),
            |c| c.caller.name.to_string(),
            |a, b| a.line.cmp(&b.line),
            |c| (c.callee.module.to_string(), c.callee.name.to_string()),
            |key, calls| (key, calls.len()),
            |_module, _map| String::new(),
        );
        assert_eq!(total, 2);
    }

    // =========================================================================
    // convert_to_module_groups tests
    // =========================================================================

    #[test]
    fn test_convert_to_module_groups_empty() {
        let by_module: BTreeMap<String, BTreeMap<String, Vec<Call>>> = BTreeMap::new();
        let groups: Vec<ModuleGroup<(String, usize)>> = convert_to_module_groups(
            by_module,
            |key, calls| (key, calls.len()),
            |_module, _map| String::new(),
        );
        assert!(groups.is_empty());
    }

    #[test]
    fn test_convert_to_module_groups_single_module() {
        let mut by_module: BTreeMap<String, BTreeMap<String, Vec<Call>>> = BTreeMap::new();
        let mut funcs = BTreeMap::new();
        funcs.insert(
            "func1".to_string(),
            vec![make_call("ModA", "func1", 1, "Target", "t", 0, 10)],
        );
        by_module.insert("ModA".to_string(), funcs);

        let groups = convert_to_module_groups(
            by_module,
            |key, calls| (key, calls.len()),
            |_module, _map| String::new(),
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "ModA");
        assert_eq!(groups[0].entries.len(), 1);
        assert_eq!(groups[0].entries[0], ("func1".to_string(), 1));
    }

    #[test]
    fn test_convert_to_module_groups_multiple_modules_sorted() {
        let mut by_module: BTreeMap<String, BTreeMap<String, Vec<Call>>> = BTreeMap::new();
        for name in &["Zeta", "Alpha", "Mid"] {
            let mut funcs = BTreeMap::new();
            funcs.insert("f".to_string(), vec![]);
            by_module.insert(name.to_string(), funcs);
        }

        let groups = convert_to_module_groups(
            by_module,
            |key, _calls| key,
            |_module, _map| String::new(),
        );

        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].name, "Alpha");
        assert_eq!(groups[1].name, "Mid");
        assert_eq!(groups[2].name, "Zeta");
    }

    #[test]
    fn test_convert_to_module_groups_file_strategy_used() {
        let mut by_module: BTreeMap<String, BTreeMap<String, Vec<Call>>> = BTreeMap::new();
        let mut funcs = BTreeMap::new();
        funcs.insert("f".to_string(), vec![]);
        by_module.insert("MyModule".to_string(), funcs);

        let groups = convert_to_module_groups(
            by_module,
            |key, _calls| key,
            |module, _map| format!("lib/{}.ex", module.to_lowercase()),
        );

        assert_eq!(groups[0].file, "lib/mymodule.ex");
    }

    #[test]
    fn test_convert_to_module_groups_function_count_is_none() {
        let mut by_module: BTreeMap<String, BTreeMap<String, Vec<Call>>> = BTreeMap::new();
        let mut funcs = BTreeMap::new();
        funcs.insert("f".to_string(), vec![]);
        by_module.insert("Mod".to_string(), funcs);

        let groups = convert_to_module_groups(
            by_module,
            |key, _calls| key,
            |_module, _map| String::new(),
        );
        assert!(groups[0].function_count.is_none());
    }

    #[test]
    fn test_convert_to_module_groups_multiple_entries_per_module() {
        let mut by_module: BTreeMap<String, BTreeMap<String, Vec<Call>>> = BTreeMap::new();
        let mut funcs = BTreeMap::new();
        funcs.insert("alpha".to_string(), vec![]);
        funcs.insert("beta".to_string(), vec![]);
        funcs.insert("gamma".to_string(), vec![]);
        by_module.insert("Mod".to_string(), funcs);

        let groups = convert_to_module_groups(
            by_module,
            |key, _calls| key,
            |_module, _map| String::new(),
        );

        assert_eq!(groups[0].entries.len(), 3);
        // BTreeMap sorts keys
        assert_eq!(groups[0].entries[0], "alpha");
        assert_eq!(groups[0].entries[1], "beta");
        assert_eq!(groups[0].entries[2], "gamma");
    }

    #[test]
    fn test_convert_to_module_groups_entry_builder_receives_calls() {
        let mut by_module: BTreeMap<String, BTreeMap<String, Vec<Call>>> = BTreeMap::new();
        let mut funcs = BTreeMap::new();
        funcs.insert(
            "f".to_string(),
            vec![
                make_call("Mod", "f", 1, "T", "a", 0, 10),
                make_call("Mod", "f", 1, "T", "b", 0, 20),
            ],
        );
        by_module.insert("Mod".to_string(), funcs);

        let groups = convert_to_module_groups(
            by_module,
            |_key, calls| calls.len(),
            |_module, _map| String::new(),
        );

        assert_eq!(groups[0].entries[0], 2);
    }
}

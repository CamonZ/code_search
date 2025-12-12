//! Utility functions for code search operations.

use std::collections::BTreeMap;
use crate::types::ModuleGroup;

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
}

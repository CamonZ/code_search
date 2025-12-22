use serde::Serialize;

/// Generic result structure for commands that group entries by module
/// Used by calls_from, calls_to, depends_on, depended_by
#[derive(Debug, Default, Serialize)]
pub struct ModuleGroupResult<E> {
    pub module_pattern: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_pattern: Option<String>,
    pub total_items: usize,
    pub items: Vec<ModuleGroup<E>>,
}

/// Generic result structure for commands with module grouping and multiple filter options
/// Used by function, specs, types
#[derive(Debug, Default, Serialize)]
pub struct ModuleCollectionResult<E> {
    pub module_pattern: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_filter: Option<String>,
    pub total_items: usize,
    pub items: Vec<ModuleGroup<E>>,
}

/// A module with a collection of generic entries
#[derive(Debug, Default, Serialize)]
pub struct ModuleGroup<E> {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub file: String,
    pub entries: Vec<E>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_count: Option<i64>,
}

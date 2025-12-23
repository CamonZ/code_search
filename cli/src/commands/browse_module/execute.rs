use std::cmp::Ordering;
use std::error::Error;

use serde::Serialize;

use super::{BrowseModuleCmd, DefinitionKind};
use crate::commands::Execute;
use db::queries::file::find_functions_in_module;
use db::queries::specs::find_specs;
use db::queries::types::find_types;
use db::queries::structs::{find_struct_fields, group_fields_into_structs, FieldInfo};

/// Result of browsing definitions in a module
#[derive(Debug, Serialize)]
pub struct BrowseModuleResult {
    /// The module name or file pattern that was searched
    pub search_term: String,

    /// The definition kind filter applied (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind_filter: Option<DefinitionKind>,

    /// Project that was searched
    pub project: String,

    /// Total number of definitions found (before limit applied)
    pub total_items: usize,

    /// All matching definitions, flattened array with type discriminant
    pub definitions: Vec<Definition>,
}

/// A single definition from any category (function, spec, type, or struct)
///
/// Uses serde(tag = "type") to add a discriminant field to JSON output,
/// making it easy for consumers to identify the definition type.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum Definition {
    /// A function definition with location and signature
    Function {
        module: String,
        #[serde(skip_serializing_if = "String::is_empty")]
        file: String,
        name: String,
        arity: i64,
        line: i64,
        start_line: i64,
        end_line: i64,
        kind: String,
        #[serde(skip_serializing_if = "String::is_empty")]
        args: String,
        #[serde(skip_serializing_if = "String::is_empty")]
        return_type: String,
        #[serde(skip_serializing_if = "String::is_empty")]
        pattern: String,
        #[serde(skip_serializing_if = "String::is_empty")]
        guard: String,
    },

    /// A spec definition (@spec or @callback)
    Spec {
        module: String,
        name: String,
        arity: i64,
        line: i64,
        kind: String,
        #[serde(skip_serializing_if = "String::is_empty")]
        inputs: String,
        #[serde(skip_serializing_if = "String::is_empty")]
        returns: String,
        #[serde(skip_serializing_if = "String::is_empty")]
        full: String,
    },

    /// A type definition (@type, @typep, or @opaque)
    Type {
        module: String,
        name: String,
        line: i64,
        kind: String,
        #[serde(skip_serializing_if = "String::is_empty")]
        params: String,
        #[serde(skip_serializing_if = "String::is_empty")]
        definition: String,
    },

    /// A struct definition with fields
    Struct {
        module: String,
        name: String,
        fields: Vec<FieldInfo>,
    },
}

impl Definition {
    /// Get the module name for this definition
    pub fn module(&self) -> &str {
        match self {
            Definition::Function { module, .. } => module,
            Definition::Spec { module, .. } => module,
            Definition::Type { module, .. } => module,
            Definition::Struct { module, .. } => module,
        }
    }

    /// Get the line number for this definition
    pub fn line(&self) -> i64 {
        match self {
            Definition::Function { line, .. } => *line,
            Definition::Spec { line, .. } => *line,
            Definition::Type { line, .. } => *line,
            Definition::Struct { .. } => 0, // Structs don't have a single line
        }
    }
}

impl Execute for BrowseModuleCmd {
    type Output = BrowseModuleResult;

    fn execute(self, db: &db::DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let mut definitions = Vec::new();

        // Determine what to query based on kind filter
        let should_query_functions = self.kind.is_none() || matches!(self.kind, Some(DefinitionKind::Functions));
        let should_query_specs = self.kind.is_none() || matches!(self.kind, Some(DefinitionKind::Specs));
        let should_query_types = self.kind.is_none() || matches!(self.kind, Some(DefinitionKind::Types));
        let should_query_structs = self.kind.is_none() || matches!(self.kind, Some(DefinitionKind::Structs));

        // Query functions (from function_locations table for file + line info)
        if should_query_functions {
            let funcs = find_functions_in_module(
                db,
                &self.module_or_file,
                &self.common.project,
                self.common.regex,
                self.common.limit,
            )?;

            for func in funcs {
                // Filter by name if specified
                if let Some(ref name_filter) = self.name
                    && !func.name.contains(name_filter) {
                        continue;
                    }

                definitions.push(Definition::Function {
                    module: func.module,
                    file: func.file,
                    name: func.name,
                    arity: func.arity,
                    line: func.line,
                    start_line: func.start_line,
                    end_line: func.end_line,
                    kind: func.kind,
                    args: String::new(), // Not in function_locations
                    return_type: String::new(), // Not in function_locations
                    pattern: func.pattern,
                    guard: func.guard,
                });
            }
        }

        // Query specs
        if should_query_specs {
            let specs = find_specs(
                db,
                &self.module_or_file,
                self.name.as_deref(),
                None, // kind filter (optional, not used for browse)
                &self.common.project,
                self.common.regex,
                self.common.limit,
            )?;

            for spec in specs {
                definitions.push(Definition::Spec {
                    module: spec.module,
                    name: spec.name,
                    arity: spec.arity,
                    line: spec.line,
                    kind: spec.kind,
                    inputs: spec.inputs_string,
                    returns: spec.return_string,
                    full: spec.full,
                });
            }
        }

        // Query types
        if should_query_types {
            let types = find_types(
                db,
                &self.module_or_file,
                self.name.as_deref(),
                None, // kind filter (optional, not used for browse)
                &self.common.project,
                self.common.regex,
                self.common.limit,
            )?;

            for type_def in types {
                definitions.push(Definition::Type {
                    module: type_def.module,
                    name: type_def.name,
                    line: type_def.line,
                    kind: type_def.kind,
                    params: type_def.params,
                    definition: type_def.definition,
                });
            }
        }

        // Query structs
        if should_query_structs {
            let fields = find_struct_fields(db, &self.module_or_file, &self.common.project, self.common.regex, self.common.limit)?;
            let structs = group_fields_into_structs(fields);

            for struct_def in structs {
                // Filter by name if specified
                if let Some(ref name_filter) = self.name
                    && !struct_def.module.contains(name_filter) {
                        continue;
                    }

                definitions.push(Definition::Struct {
                    module: struct_def.module.clone(),
                    name: struct_def.module.clone(), // Struct name is same as module for now
                    fields: struct_def.fields,
                });
            }
        }

        // Sort by module, then by line number
        definitions.sort_by(|a, b| {
            match a.module().cmp(b.module()) {
                Ordering::Equal => a.line().cmp(&b.line()),
                other => other,
            }
        });

        let total_items = definitions.len();

        // Apply limit
        definitions.truncate(self.common.limit as usize);

        Ok(BrowseModuleResult {
            search_term: self.module_or_file,
            kind_filter: self.kind,
            project: self.common.project,
            total_items,
            definitions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_definition_sort_order() {
        let defs = vec![
            Definition::Function {
                module: "B".to_string(),
                file: String::new(),
                name: "f".to_string(),
                arity: 0,
                line: 10,
                start_line: 10,
                end_line: 10,
                kind: "def".to_string(),
                args: String::new(),
                return_type: String::new(),
                pattern: String::new(),
                guard: String::new(),
            },
            Definition::Function {
                module: "A".to_string(),
                file: String::new(),
                name: "f".to_string(),
                arity: 0,
                line: 20,
                start_line: 20,
                end_line: 20,
                kind: "def".to_string(),
                args: String::new(),
                return_type: String::new(),
                pattern: String::new(),
                guard: String::new(),
            },
            Definition::Function {
                module: "A".to_string(),
                file: String::new(),
                name: "f".to_string(),
                arity: 0,
                line: 5,
                start_line: 5,
                end_line: 5,
                kind: "def".to_string(),
                args: String::new(),
                return_type: String::new(),
                pattern: String::new(),
                guard: String::new(),
            },
        ];

        let mut sorted = defs.clone();
        sorted.sort_by(|a, b| match a.module().cmp(b.module()) {
            Ordering::Equal => a.line().cmp(&b.line()),
            other => other,
        });

        // Should be: A/5, A/20, B/10
        assert_eq!(sorted[0].module(), "A");
        assert_eq!(sorted[0].line(), 5);
        assert_eq!(sorted[1].module(), "A");
        assert_eq!(sorted[1].line(), 20);
        assert_eq!(sorted[2].module(), "B");
        assert_eq!(sorted[2].line(), 10);
    }
}

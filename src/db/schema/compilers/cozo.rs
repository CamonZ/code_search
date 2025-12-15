//! Cozo Datalog DDL compiler.
//!
//! Generates Cozo Datalog DDL (`:create relation { ... }`) from backend-agnostic
//! schema definitions. The output format is deterministic and matches the current
//! schema strings exactly (whitespace-normalized).

use crate::db::schema::definition::SchemaRelation;

/// Compiler for generating Cozo Datalog DDL from schema definitions.
pub struct CozoCompiler;

impl CozoCompiler {
    /// Generate Cozo DDL for a single relation.
    ///
    /// Produces output in the format:
    /// ```cozo
    /// :create relation_name {
    ///     key_field1: Type1,
    ///     key_field2: Type2
    ///     =>
    ///     value_field1: Type1 default "...",
    ///     value_field2: Type2
    /// }
    /// ```
    pub fn compile_relation(relation: &SchemaRelation) -> String {
        let key_fields = relation
            .key_fields
            .iter()
            .map(|f| format!("    {}: {}", f.name, f.data_type.cozo_type()))
            .collect::<Vec<_>>()
            .join(",\n");

        let value_fields = relation
            .value_fields
            .iter()
            .map(|f| {
                if let Some(default) = f.default {
                    // For Int types, don't quote the default value; for others, quote it
                    match f.data_type {
                        crate::db::schema::definition::DataType::Int => {
                            format!(
                                "    {}: {} default {}",
                                f.name,
                                f.data_type.cozo_type(),
                                default
                            )
                        }
                        _ => {
                            format!(
                                "    {}: {} default \"{}\"",
                                f.name,
                                f.data_type.cozo_type(),
                                default
                            )
                        }
                    }
                } else {
                    format!("    {}: {}", f.name, f.data_type.cozo_type())
                }
            })
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            ":create {} {{\n{}\n    =>\n{}\n}}",
            relation.name, key_fields, value_fields
        )
    }

    /// Generate DDL for all relations.
    ///
    /// Takes a slice of relation references and returns a vector of compiled DDL strings.
    pub fn compile_all(relations: &[&SchemaRelation]) -> Vec<String> {
        relations
            .iter()
            .map(|rel| Self::compile_relation(rel))
            .collect()
    }

    /// Generate Cozo :put statement for batch insert.
    ///
    /// Produces output in the format:
    /// ```cozo
    /// ?[col1, col2, col3, ...] <- [[val1, val2, val3], [val4, val5, val6], ...]
    /// :put table_name { key1, key2 => val1, val2 }
    /// ```
    ///
    /// # Arguments
    /// * `relation` - The schema relation definition
    /// * `row_literals` - Pre-formatted row strings like `["val1", "val2", 3]`
    ///
    /// # Example
    /// ```ignore
    /// let rows = vec![
    ///     r#"["proj", "MyApp", "", "unknown"]"#.to_string(),
    ///     r#"["proj", "MyApp.User", "", "unknown"]"#.to_string(),
    /// ];
    /// let script = CozoCompiler::compile_insert(&MODULES, &rows);
    /// // Returns:
    /// // ?[project, name, file, source] <- [["proj", "MyApp", "", "unknown"], ["proj", "MyApp.User", "", "unknown"]]
    /// // :put modules { project, name => file, source }
    /// ```
    pub fn compile_insert(relation: &SchemaRelation, row_literals: &[String]) -> String {
        let all_columns = relation
            .all_fields()
            .map(|f| f.name)
            .collect::<Vec<_>>()
            .join(", ");

        let key_columns = relation
            .key_fields
            .iter()
            .map(|f| f.name)
            .collect::<Vec<_>>()
            .join(", ");

        let value_columns = relation
            .value_fields
            .iter()
            .map(|f| f.name)
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "?[{}] <- [{}]\n:put {} {{ {} => {} }}",
            all_columns,
            row_literals.join(", "),
            relation.name,
            key_columns,
            value_columns,
        )
    }

    /// Generate Cozo :rm statement for deleting rows by project.
    ///
    /// Produces output in the format:
    /// ```cozo
    /// ?[key1, key2, ...] := *table{project: $project, key1, key2, ...}
    /// :rm table {key1, key2, ...}
    /// ```
    ///
    /// # Arguments
    /// * `relation` - The schema relation definition
    ///
    /// # Example
    /// ```ignore
    /// let script = CozoCompiler::compile_delete_by_project(&MODULES);
    /// // Returns:
    /// // ?[project, name] := *modules{project: $project, project, name}
    /// // :rm modules {project, name}
    /// ```
    pub fn compile_delete_by_project(relation: &SchemaRelation) -> String {
        let key_columns = relation
            .key_fields
            .iter()
            .map(|f| f.name)
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "?[{}] := *{}{{project: $project, {}}}\n:rm {} {{{}}}",
            key_columns, relation.name, key_columns, relation.name, key_columns,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::relations::*;
    use crate::queries::schema::*;

    /// Helper to normalize whitespace for comparison.
    fn normalize_whitespace(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn test_modules_compilation() {
        let compiled = CozoCompiler::compile_relation(&MODULES);

        // Must contain Cozo syntax
        assert!(compiled.contains(":create modules"));
        assert!(compiled.contains("project: String"));
        assert!(compiled.contains("name: String"));
        assert!(compiled.contains("file: String default \"\""));
        assert!(compiled.contains("source: String default \"unknown\""));

        // Must have key/value separator
        assert!(compiled.contains("=>"));

        // Must match current schema (whitespace-normalized)
        let compiled_normalized = normalize_whitespace(&compiled);
        let current_normalized = normalize_whitespace(SCHEMA_MODULES.trim());
        assert_eq!(compiled_normalized, current_normalized);
    }

    #[test]
    fn test_functions_compilation() {
        let compiled = CozoCompiler::compile_relation(&FUNCTIONS);

        assert!(compiled.contains(":create functions"));
        assert!(compiled.contains("project: String"));
        assert!(compiled.contains("module: String"));
        assert!(compiled.contains("name: String"));
        assert!(compiled.contains("arity: Int"));
        assert!(compiled.contains("return_type: String default \"\""));
        assert!(compiled.contains("args: String default \"\""));
        assert!(compiled.contains("source: String default \"unknown\""));
        assert!(compiled.contains("=>"));

        // Must match current schema (whitespace-normalized)
        let compiled_normalized = normalize_whitespace(&compiled);
        let current_normalized = normalize_whitespace(SCHEMA_FUNCTIONS.trim());
        assert_eq!(compiled_normalized, current_normalized);
    }

    #[test]
    fn test_calls_compilation() {
        let compiled = CozoCompiler::compile_relation(&CALLS);

        assert!(compiled.contains(":create calls"));
        assert!(compiled.contains("project: String"));
        assert!(compiled.contains("caller_module: String"));
        assert!(compiled.contains("caller_function: String"));
        assert!(compiled.contains("callee_module: String"));
        assert!(compiled.contains("callee_function: String"));
        assert!(compiled.contains("callee_arity: Int"));
        assert!(compiled.contains("file: String"));
        assert!(compiled.contains("line: Int"));
        assert!(compiled.contains("column: Int"));
        assert!(compiled.contains("call_type: String default \"remote\""));
        assert!(compiled.contains("caller_kind: String default \"\""));
        assert!(compiled.contains("callee_args: String default \"\""));

        // Must match current schema (whitespace-normalized)
        let compiled_normalized = normalize_whitespace(&compiled);
        let current_normalized = normalize_whitespace(SCHEMA_CALLS.trim());
        assert_eq!(compiled_normalized, current_normalized);
    }

    #[test]
    fn test_struct_fields_compilation() {
        let compiled = CozoCompiler::compile_relation(&STRUCT_FIELDS);

        assert!(compiled.contains(":create struct_fields"));
        assert!(compiled.contains("project: String"));
        assert!(compiled.contains("module: String"));
        assert!(compiled.contains("field: String"));
        assert!(compiled.contains("default_value: String"));
        assert!(compiled.contains("required: Bool"));
        assert!(compiled.contains("inferred_type: String"));

        // Must match current schema (whitespace-normalized)
        let compiled_normalized = normalize_whitespace(&compiled);
        let current_normalized = normalize_whitespace(SCHEMA_STRUCT_FIELDS.trim());
        assert_eq!(compiled_normalized, current_normalized);
    }

    #[test]
    fn test_function_locations_compilation() {
        let compiled = CozoCompiler::compile_relation(&FUNCTION_LOCATIONS);

        assert!(compiled.contains(":create function_locations"));
        assert!(compiled.contains("project: String"));
        assert!(compiled.contains("module: String"));
        assert!(compiled.contains("name: String"));
        assert!(compiled.contains("arity: Int"));
        assert!(compiled.contains("line: Int"));
        assert!(compiled.contains("file: String"));
        assert!(compiled.contains("source_file_absolute: String default \"\""));
        assert!(compiled.contains("column: Int"));
        assert!(compiled.contains("kind: String"));
        assert!(compiled.contains("start_line: Int"));
        assert!(compiled.contains("end_line: Int"));
        assert!(compiled.contains("pattern: String default \"\""));
        assert!(compiled.contains("guard: String default \"\""));
        assert!(compiled.contains("source_sha: String default \"\""));
        assert!(compiled.contains("ast_sha: String default \"\""));
        assert!(compiled.contains("complexity: Int default 1"));
        assert!(compiled.contains("max_nesting_depth: Int default 0"));
        assert!(compiled.contains("generated_by: String default \"\""));
        assert!(compiled.contains("macro_source: String default \"\""));

        // Must match current schema (whitespace-normalized)
        let compiled_normalized = normalize_whitespace(&compiled);
        let current_normalized = normalize_whitespace(SCHEMA_FUNCTION_LOCATIONS.trim());
        assert_eq!(compiled_normalized, current_normalized);
    }

    #[test]
    fn test_specs_compilation() {
        let compiled = CozoCompiler::compile_relation(&SPECS);

        assert!(compiled.contains(":create specs"));
        assert!(compiled.contains("project: String"));
        assert!(compiled.contains("module: String"));
        assert!(compiled.contains("name: String"));
        assert!(compiled.contains("arity: Int"));
        assert!(compiled.contains("kind: String"));
        assert!(compiled.contains("line: Int"));
        assert!(compiled.contains("inputs_string: String default \"\""));
        assert!(compiled.contains("return_string: String default \"\""));
        assert!(compiled.contains("full: String default \"\""));

        // Must match current schema (whitespace-normalized)
        let compiled_normalized = normalize_whitespace(&compiled);
        let current_normalized = normalize_whitespace(SCHEMA_SPECS.trim());
        assert_eq!(compiled_normalized, current_normalized);
    }

    #[test]
    fn test_types_compilation() {
        let compiled = CozoCompiler::compile_relation(&TYPES);

        assert!(compiled.contains(":create types"));
        assert!(compiled.contains("project: String"));
        assert!(compiled.contains("module: String"));
        assert!(compiled.contains("name: String"));
        assert!(compiled.contains("kind: String"));
        assert!(compiled.contains("params: String default \"\""));
        assert!(compiled.contains("line: Int"));
        assert!(compiled.contains("definition: String default \"\""));

        // Must match current schema (whitespace-normalized)
        let compiled_normalized = normalize_whitespace(&compiled);
        let current_normalized = normalize_whitespace(SCHEMA_TYPES.trim());
        assert_eq!(compiled_normalized, current_normalized);
    }

    #[test]
    fn test_compile_all() {
        let compiled = CozoCompiler::compile_all(&ALL_RELATIONS);
        assert_eq!(compiled.len(), 7, "Should compile all 7 relations");

        // Verify each relation is in the output
        assert!(compiled[0].contains(":create modules"));
        assert!(compiled[1].contains(":create functions"));
        assert!(compiled[2].contains(":create calls"));
        assert!(compiled[3].contains(":create struct_fields"));
        assert!(compiled[4].contains(":create function_locations"));
        assert!(compiled[5].contains(":create specs"));
        assert!(compiled[6].contains(":create types"));
    }

    #[test]
    fn test_compile_all_matches_current_schemas() {
        let compiled = CozoCompiler::compile_all(&ALL_RELATIONS);

        let schemas = vec![
            (SCHEMA_MODULES, "modules"),
            (SCHEMA_FUNCTIONS, "functions"),
            (SCHEMA_CALLS, "calls"),
            (SCHEMA_STRUCT_FIELDS, "struct_fields"),
            (SCHEMA_FUNCTION_LOCATIONS, "function_locations"),
            (SCHEMA_SPECS, "specs"),
            (SCHEMA_TYPES, "types"),
        ];

        for (compiled_output, (current_schema, relation_name)) in
            compiled.iter().zip(schemas.iter())
        {
            let compiled_normalized = normalize_whitespace(compiled_output);
            let current_normalized = normalize_whitespace(current_schema.trim());
            assert_eq!(
                compiled_normalized, current_normalized,
                "Compiled {} should match current schema",
                relation_name
            );
        }
    }

    #[test]
    fn test_output_format_structure() {
        let compiled = CozoCompiler::compile_relation(&MODULES);

        // Check structure
        assert!(compiled.starts_with(":create"));
        assert!(compiled.contains("{"));
        assert!(compiled.contains("}"));
        assert!(compiled.ends_with("}"));

        // Check indentation
        let lines: Vec<&str> = compiled.lines().collect();
        assert!(lines.len() > 3, "Should have multiple lines");

        // First line should be :create
        assert!(lines[0].starts_with(":create"));

        // Middle lines should be indented
        for line in &lines[1..lines.len() - 1] {
            if !line.trim().is_empty() {
                assert!(
                    line.starts_with("    ") || line.starts_with("=>"),
                    "Non-empty lines should be indented or be the separator"
                );
            }
        }

        // Last line should be closing brace
        assert_eq!(lines[lines.len() - 1], "}");
    }

    #[test]
    fn test_all_relations_compile_without_panic() {
        for relation in ALL_RELATIONS {
            let _compiled = CozoCompiler::compile_relation(relation);
            // If we got here without panicking, the test passes
        }
    }

    #[test]
    fn test_compile_insert_modules() {
        let rows = vec![r#"["proj", "MyApp", "", "unknown"]"#.to_string()];
        let script = CozoCompiler::compile_insert(&MODULES, &rows);

        assert!(script.contains("?[project, name, file, source]"));
        assert!(script.contains("<-"));
        assert!(script.contains(":put modules"));
        assert!(script.contains("project, name => file, source"));
    }

    #[test]
    fn test_compile_insert_multiple_rows() {
        let rows = vec![
            r#"["proj", "MyApp", "", "unknown"]"#.to_string(),
            r#"["proj", "MyApp.User", "", "unknown"]"#.to_string(),
        ];
        let script = CozoCompiler::compile_insert(&MODULES, &rows);

        // Should contain both rows
        assert!(script.contains("MyApp"));
        assert!(script.contains("MyApp.User"));
    }

    #[test]
    fn test_compile_insert_functions() {
        let rows = vec![r#"["proj", "MyApp", "start", 0, "", "", "unknown"]"#.to_string()];
        let script = CozoCompiler::compile_insert(&FUNCTIONS, &rows);

        assert!(script.contains("?[project, module, name, arity, return_type, args, source]"));
        assert!(script.contains(":put functions"));
        assert!(script.contains("project, module, name, arity => return_type, args, source"));
    }

    #[test]
    fn test_compile_insert_calls() {
        let rows = vec![
            r#"["proj", "MyApp", "start/0", "Logger", "info", 1, "lib/app.ex", 10, 5, "remote", "def", ""]"#
                .to_string(),
        ];
        let script = CozoCompiler::compile_insert(&CALLS, &rows);

        assert!(script.contains(":put calls"));
        // CALLS has 9 key fields and 3 value fields
        assert!(script.contains(
            "project, caller_module, caller_function, callee_module, callee_function, callee_arity, file, line, column => call_type, caller_kind, callee_args"
        ));
    }

    #[test]
    fn test_compile_delete_by_project_modules() {
        let script = CozoCompiler::compile_delete_by_project(&MODULES);

        assert!(script.contains("?[project, name]"));
        assert!(script.contains("*modules{project: $project"));
        assert!(script.contains(":rm modules {project, name}"));
    }

    #[test]
    fn test_compile_delete_by_project_functions() {
        let script = CozoCompiler::compile_delete_by_project(&FUNCTIONS);

        assert!(script.contains("?[project, module, name, arity]"));
        assert!(script.contains("*functions{project: $project"));
        assert!(script.contains(":rm functions {project, module, name, arity}"));
    }

    #[test]
    fn test_compile_delete_by_project_calls() {
        let script = CozoCompiler::compile_delete_by_project(&CALLS);

        assert!(script.contains("*calls{project: $project"));
        assert!(script.contains(":rm calls"));
    }

    #[test]
    fn test_compile_insert_all_relations() {
        // Verify all relations can generate valid insert statements
        for relation in ALL_RELATIONS {
            let rows = vec!["[]".to_string()]; // Empty row just for syntax check
            let script = CozoCompiler::compile_insert(relation, &rows);
            assert!(script.contains(":put"), "Should contain :put for {}", relation.name);
            assert!(
                script.contains(relation.name),
                "Should contain relation name for {}",
                relation.name
            );
        }
    }

    #[test]
    fn test_compile_delete_all_relations() {
        // Verify all relations can generate valid delete statements
        for relation in ALL_RELATIONS {
            let script = CozoCompiler::compile_delete_by_project(relation);
            assert!(script.contains(":rm"), "Should contain :rm for {}", relation.name);
            assert!(
                script.contains("$project"),
                "Should contain $project for {}",
                relation.name
            );
        }
    }
}

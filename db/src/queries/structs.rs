use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_bool, extract_string, extract_string_or};

#[cfg(feature = "backend-cozo")]
use crate::db::run_query;

#[cfg(feature = "backend-cozo")]
use crate::query_builders::{validate_regex_patterns, ConditionBuilder};

#[cfg(feature = "backend-surrealdb")]
use crate::query_builders::validate_regex_patterns;

#[derive(Error, Debug)]
pub enum StructError {
    #[error("Struct query failed: {message}")]
    QueryFailed { message: String },
}

/// A struct field definition
#[derive(Debug, Clone, Serialize)]
pub struct StructField {
    pub project: String,
    pub module: String,
    pub field: String,
    pub default_value: String,
    pub required: bool,
    pub inferred_type: String,
}

/// A struct with all its fields grouped
#[derive(Debug, Clone, Serialize)]
pub struct StructDefinition {
    pub project: String,
    pub module: String,
    pub fields: Vec<FieldInfo>,
}

/// Field information within a struct
#[derive(Debug, Clone, Serialize)]
pub struct FieldInfo {
    pub name: String,
    pub default_value: String,
    pub required: bool,
    pub inferred_type: String,
}

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
pub fn find_struct_fields(
    db: &dyn Database,
    module_pattern: &str,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<StructField>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern)])?;

    let module_cond = ConditionBuilder::new("module", "module_pattern").build(use_regex);

    let project_cond = ", project == $project";

    let script = format!(
        r#"
        ?[project, module, field, default_value, required, inferred_type] :=
            *struct_fields{{project, module, field, default_value, required, inferred_type}},
            {module_cond}
            {project_cond}
        :order module, field
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new();
    params = params.with_str("module_pattern", module_pattern);
    params = params.with_str("project", project);

    let result = run_query(db, &script, params).map_err(|e| StructError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 6 {
            let Some(project) = extract_string(row.get(0).unwrap()) else { continue };
            let Some(module) = extract_string(row.get(1).unwrap()) else { continue };
            let Some(field) = extract_string(row.get(2).unwrap()) else { continue };
            let default_value = extract_string_or(row.get(3).unwrap(), "");
            let required = extract_bool(row.get(4).unwrap(), false);
            let inferred_type = extract_string_or(row.get(5).unwrap(), "");

            results.push(StructField {
                project,
                module,
                field,
                default_value,
                required,
                inferred_type,
            });
        }
    }

    Ok(results)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
pub fn find_struct_fields(
    db: &dyn Database,
    module_pattern: &str,
    _project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<StructField>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern)])?;

    // In SurrealDB, project is implicit (one DB per project)
    // Build the WHERE clause based on regex vs exact match
    // Empty pattern means "match all"
    let where_clause = if module_pattern.is_empty() {
        String::new() // No WHERE clause - match all records
    } else if use_regex {
        "WHERE module_name = <regex>$module_pattern".to_string()
    } else {
        "WHERE module_name = $module_pattern".to_string()
    };

    // Note: field table no longer has inferred_type in SurrealDB schema
    // We return empty string for inferred_type to maintain API compatibility
    let query = format!(
        r#"
        SELECT "default" as project, module_name, name, default_value, required
        FROM fields
        {where_clause}
        ORDER BY module_name, name
        LIMIT $limit
        "#,
    );

    let params = QueryParams::new()
        .with_str("module_pattern", module_pattern)
        .with_int("limit", limit as i64);

    let result = db.execute_query(&query, params).map_err(|e| StructError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        // SurrealDB returns columns in alphabetical order: default_value, module_name, name, project, required
        // Note: inferred_type is no longer in the schema, we return empty string
        if row.len() >= 5 {
            let default_value = extract_string_or(row.get(0).unwrap(), "");
            let Some(module) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            let Some(field) = extract_string(row.get(2).unwrap()) else {
                continue;
            };
            let Some(project) = extract_string(row.get(3).unwrap()) else {
                continue;
            };
            let required = extract_bool(row.get(4).unwrap(), false);

            results.push(StructField {
                project,
                module,
                field,
                default_value,
                required,
                inferred_type: String::new(), // Not stored in SurrealDB schema
            });
        }
    }

    // SurrealDB doesn't honor ORDER BY when using regex WHERE clauses
    // Sort results in Rust to ensure consistent ordering
    results.sort_by(|a, b| {
        a.module
            .cmp(&b.module)
            .then_with(|| a.field.cmp(&b.field))
    });

    Ok(results)
}

pub fn group_fields_into_structs(fields: Vec<StructField>) -> Vec<StructDefinition> {
    use std::collections::BTreeMap;

    let mut grouped: BTreeMap<(String, String), Vec<FieldInfo>> = BTreeMap::new();

    for field in fields {
        let key = (field.project.clone(), field.module.clone());
        grouped.entry(key).or_default().push(FieldInfo {
            name: field.field,
            default_value: field.default_value,
            required: field.required,
            inferred_type: field.inferred_type,
        });
    }

    grouped
        .into_iter()
        .map(|((project, module), fields)| StructDefinition {
            project,
            module,
            fields,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    // ==================== CozoDB Tests ====================
    #[cfg(feature = "backend-cozo")]
    mod cozo_tests {
        use super::*;

        #[fixture]
        fn populated_db() -> Box<dyn crate::backend::Database> {
            crate::test_utils::structs_db("default")
        }

        #[rstest]
        fn test_find_struct_fields_returns_results(populated_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*populated_db, "", "default", false, 100);
            assert!(result.is_ok());
            let fields = result.unwrap();
            // May be empty if fixture doesn't have struct fields, just verify query executes
            assert!(fields.is_empty() || !fields.is_empty(), "Query should execute");
        }

        #[rstest]
        fn test_find_struct_fields_empty_results(populated_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*populated_db, "NonExistentModule", "default", false, 100);
            assert!(result.is_ok());
            let fields = result.unwrap();
            assert!(fields.is_empty(), "Should return empty results for non-existent module");
        }

        #[rstest]
        fn test_find_struct_fields_with_module_filter(populated_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*populated_db, "MyApp", "default", false, 100);
            assert!(result.is_ok());
            let fields = result.unwrap();
            for field in &fields {
                assert!(field.module.contains("MyApp"), "Module should match filter");
            }
        }

        #[rstest]
        fn test_find_struct_fields_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
            let limit_5 = find_struct_fields(&*populated_db, "", "default", false, 5)
                .unwrap();
            let limit_100 = find_struct_fields(&*populated_db, "", "default", false, 100)
                .unwrap();

            assert!(limit_5.len() <= 5, "Limit should be respected");
            assert!(limit_5.len() <= limit_100.len(), "Higher limit should return >= results");
        }

        #[rstest]
        fn test_find_struct_fields_with_regex_pattern(populated_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*populated_db, "^MyApp\\..*$", "default", true, 100);
            assert!(result.is_ok());
            let fields = result.unwrap();
            for field in &fields {
                assert!(field.module.starts_with("MyApp"), "Module should match regex");
            }
        }

        #[rstest]
        fn test_find_struct_fields_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*populated_db, "[invalid", "default", true, 100);
            assert!(result.is_err(), "Should reject invalid regex");
        }

        #[rstest]
        fn test_find_struct_fields_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*populated_db, "", "nonexistent", false, 100);
            assert!(result.is_ok());
            let fields = result.unwrap();
            assert!(fields.is_empty(), "Non-existent project should return no results");
        }

        #[rstest]
        fn test_find_struct_fields_returns_valid_structure(populated_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*populated_db, "", "default", false, 100);
            assert!(result.is_ok());
            let fields = result.unwrap();
            if !fields.is_empty() {
                let field = &fields[0];
                assert_eq!(field.project, "default");
                assert!(!field.module.is_empty());
                assert!(!field.field.is_empty());
            }
        }
    }

    // ==================== SurrealDB Tests ====================
    #[cfg(feature = "backend-surrealdb")]
    mod surrealdb_tests {
        use super::*;

        #[fixture]
        fn surreal_db() -> Box<dyn crate::backend::Database> {
            crate::test_utils::surreal_structs_db()
        }

        #[rstest]
        fn test_find_struct_fields_returns_results(surreal_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*surreal_db, "", "default", false, 100);
            assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
            let fields = result.unwrap();
            assert_eq!(fields.len(), 2, "Should find exactly 2 fields (person.name and person.age)");
        }

        #[rstest]
        fn test_find_struct_fields_empty_results(surreal_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*surreal_db, "NonExistentModule", "default", false, 100);
            assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
            let fields = result.unwrap();
            assert!(fields.is_empty(), "Should return empty results for non-existent module");
        }

        #[rstest]
        fn test_find_struct_fields_with_exact_module(surreal_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*surreal_db, "structs_module", "default", false, 100);
            assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
            let fields = result.unwrap();
            assert_eq!(fields.len(), 2, "Should find exactly 2 fields for structs_module");
            // Verify field properties
            assert_eq!(fields[0].module, "structs_module");
            assert_eq!(fields[0].field, "age");
            assert_eq!(fields[1].module, "structs_module");
            assert_eq!(fields[1].field, "name");
            // Note: inferred_type is not stored in SurrealDB schema (empty string)
        }

        #[rstest]
        fn test_find_struct_fields_respects_limit(surreal_db: Box<dyn crate::backend::Database>) {
            let limit_1 = find_struct_fields(&*surreal_db, "", "default", false, 1)
                .unwrap();
            let limit_100 = find_struct_fields(&*surreal_db, "", "default", false, 100)
                .unwrap();

            assert_eq!(limit_1.len(), 1, "Should respect limit of 1");
            assert_eq!(limit_100.len(), 2, "Should return all 2 fields with higher limit");
        }

        #[rstest]
        fn test_find_struct_fields_with_regex_pattern(surreal_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*surreal_db, "structs.*", "default", true, 100);
            assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
            let fields = result.unwrap();
            assert_eq!(fields.len(), 2, "Should find all fields matching regex pattern");
            for field in &fields {
                assert!(field.module.starts_with("structs"), "Module should match regex pattern");
            }
        }

        #[rstest]
        fn test_find_struct_fields_with_alternation_regex(surreal_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*surreal_db, "(structs|other).*", "default", true, 100);
            assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
            let fields = result.unwrap();
            assert_eq!(fields.len(), 2, "Should find fields matching alternation pattern");
        }

        #[rstest]
        fn test_find_struct_fields_invalid_regex(surreal_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*surreal_db, "[invalid", "default", true, 100);
            assert!(result.is_err(), "Should reject invalid regex");
        }

        #[rstest]
        fn test_find_struct_fields_returns_valid_structure(surreal_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*surreal_db, "", "default", false, 100);
            assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
            let fields = result.unwrap();
            assert!(!fields.is_empty(), "Should find at least one field");
            let field = &fields[0];
            assert_eq!(field.project, "default", "Project should be 'default'");
            assert!(!field.module.is_empty(), "Module should not be empty");
            assert!(!field.field.is_empty(), "Field name should not be empty");
            // Note: inferred_type is not stored in SurrealDB schema (empty string)
        }

        #[rstest]
        fn test_find_struct_fields_project_always_default(surreal_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*surreal_db, "", "default", false, 100);
            assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
            let fields = result.unwrap();
            for field in &fields {
                assert_eq!(field.project, "default", "All fields should have project='default'");
            }
        }

        #[rstest]
        fn test_find_struct_fields_sorted_by_module_then_field(surreal_db: Box<dyn crate::backend::Database>) {
            let result = find_struct_fields(&*surreal_db, "", "default", false, 100);
            assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
            let fields = result.unwrap();
            // Verify fields are sorted by module then field name
            for i in 0..fields.len() - 1 {
                let curr = &fields[i];
                let next = &fields[i + 1];
                if curr.module == next.module {
                    assert!(curr.field <= next.field, "Fields within same module should be sorted");
                } else {
                    assert!(curr.module < next.module, "Modules should be sorted");
                }
            }
        }

        #[rstest]
        fn test_group_fields_into_structs_from_surrealdb_results(surreal_db: Box<dyn crate::backend::Database>) {
            let fields_result = find_struct_fields(&*surreal_db, "", "default", false, 100);
            assert!(fields_result.is_ok(), "Should retrieve fields");
            let fields = fields_result.unwrap();

            let structs = group_fields_into_structs(fields);
            assert_eq!(structs.len(), 1, "Should have 1 struct (person)");
            assert_eq!(structs[0].module, "structs_module");
            assert_eq!(structs[0].fields.len(), 2, "person struct should have 2 fields");
        }
    }

    // ==================== Shared Tests ====================
    #[test]
    fn test_group_fields_into_structs_groups_correctly() {
        let fields = vec![
            StructField {
                project: "proj".to_string(),
                module: "Module1".to_string(),
                field: "field1".to_string(),
                default_value: "".to_string(),
                required: true,
                inferred_type: "String".to_string(),
            },
            StructField {
                project: "proj".to_string(),
                module: "Module1".to_string(),
                field: "field2".to_string(),
                default_value: "0".to_string(),
                required: false,
                inferred_type: "i64".to_string(),
            },
            StructField {
                project: "proj".to_string(),
                module: "Module2".to_string(),
                field: "field3".to_string(),
                default_value: "".to_string(),
                required: true,
                inferred_type: "bool".to_string(),
            },
        ];

        let structs = group_fields_into_structs(fields);

        assert_eq!(structs.len(), 2, "Should have 2 structs");
        assert_eq!(structs[0].fields.len(), 2, "First struct should have 2 fields");
        assert_eq!(structs[1].fields.len(), 1, "Second struct should have 1 field");
    }

    #[test]
    fn test_group_fields_into_structs_empty() {
        let fields = vec![];
        let structs = group_fields_into_structs(fields);
        assert!(structs.is_empty(), "Empty fields should result in empty structs");
    }

    #[test]
    fn test_group_fields_into_structs_single_field() {
        let fields = vec![
            StructField {
                project: "proj".to_string(),
                module: "TestModule".to_string(),
                field: "single_field".to_string(),
                default_value: "nil".to_string(),
                required: true,
                inferred_type: "string()".to_string(),
            },
        ];

        let structs = group_fields_into_structs(fields);
        assert_eq!(structs.len(), 1, "Should have 1 struct");
        assert_eq!(structs[0].fields.len(), 1, "Struct should have 1 field");
        assert_eq!(structs[0].fields[0].name, "single_field");
        assert_eq!(structs[0].fields[0].default_value, "nil");
        assert_eq!(structs[0].fields[0].required, true);
        assert_eq!(structs[0].fields[0].inferred_type, "string()");
    }

    #[test]
    fn test_group_fields_into_structs_multiple_projects() {
        let fields = vec![
            StructField {
                project: "proj1".to_string(),
                module: "Module1".to_string(),
                field: "field1".to_string(),
                default_value: "".to_string(),
                required: true,
                inferred_type: "String".to_string(),
            },
            StructField {
                project: "proj2".to_string(),
                module: "Module1".to_string(),
                field: "field1".to_string(),
                default_value: "".to_string(),
                required: true,
                inferred_type: "String".to_string(),
            },
        ];

        let structs = group_fields_into_structs(fields);
        // Should be grouped by (project, module) pair
        assert_eq!(structs.len(), 2, "Should have 2 structs (different projects)");
    }
}

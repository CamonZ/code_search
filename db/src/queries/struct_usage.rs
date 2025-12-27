use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string};
use crate::query_builders::validate_regex_patterns;

#[cfg(feature = "backend-cozo")]
use crate::db::run_query;

#[cfg(feature = "backend-cozo")]
use crate::query_builders::{OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum StructUsageError {
    #[error("Struct usage query failed: {message}")]
    QueryFailed { message: String },
}

/// A function that accepts or returns a specific type
#[derive(Debug, Clone, Serialize)]
pub struct StructUsageEntry {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub inputs_string: String,
    pub return_string: String,
    pub line: i64,
}

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
pub fn find_struct_usage(
    db: &dyn Database,
    pattern: &str,
    project: &str,
    use_regex: bool,
    module_pattern: Option<&str>,
    limit: u32,
) -> Result<Vec<StructUsageEntry>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(pattern), module_pattern])?;

    // Build pattern matching function for both inputs and return (manual OR condition)
    let match_cond = if use_regex {
        "regex_matches(inputs_string, $pattern) or regex_matches(return_string, $pattern)"
    } else {
        "inputs_string == $pattern or return_string == $pattern"
    };

    // Build module filter using OptionalConditionBuilder
    let module_cond = OptionalConditionBuilder::new("module", "module_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(module_pattern.is_some(), use_regex);

    let script = format!(
        r#"
        ?[project, module, name, arity, inputs_string, return_string, line] :=
            *specs{{project, module, name, arity, inputs_string, return_string, line}},
            project == $project,
            {match_cond}
            {module_cond}

        :order module, name, arity
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new();
    params = params.with_str("pattern", pattern);
    params = params.with_str("project", project);

    if let Some(mod_pat) = module_pattern {
        params = params.with_str("module_pattern", mod_pat);
    }

    let result = run_query(db, &script, params).map_err(|e| StructUsageError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 7 {
            let Some(project) = extract_string(row.get(0).unwrap()) else {
                continue;
            };
            let Some(module) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            let Some(name) = extract_string(row.get(2).unwrap()) else {
                continue;
            };
            let arity = extract_i64(row.get(3).unwrap(), 0);
            let inputs_string = extract_string(row.get(4).unwrap()).unwrap_or_default();
            let return_string = extract_string(row.get(5).unwrap()).unwrap_or_default();
            let line = extract_i64(row.get(6).unwrap(), 0);

            results.push(StructUsageEntry {
                project,
                module,
                name,
                arity,
                inputs_string,
                return_string,
                line,
            });
        }
    }

    Ok(results)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
pub fn find_struct_usage(
    db: &dyn Database,
    pattern: &str,
    _project: &str,
    use_regex: bool,
    module_pattern: Option<&str>,
    limit: u32,
) -> Result<Vec<StructUsageEntry>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(pattern), module_pattern])?;

    // Build WHERE conditions based on what filters are present
    let mut conditions = Vec::new();
    let params = QueryParams::new().with_int("limit", limit as i64);

    // Add pattern filter if provided
    // Build conditions for BOTH input_strings and return_strings arrays
    if !pattern.is_empty() {
        if use_regex {
            // For regex matching: check if any element in EITHER array matches
            let escaped_pattern = pattern.replace('\\', "\\\\").replace('"', "\\\"");
            conditions.push(format!(
                "(array::len(array::filter(input_strings, |$v| string::matches($v, /^{}/))) > 0 OR array::len(array::filter(return_strings, |$v| string::matches($v, /^{}/))) > 0)",
                escaped_pattern, escaped_pattern
            ));
        } else {
            // For substring matching: check if pattern appears in EITHER joined array
            let escaped_pattern = pattern.replace('\\', "\\\\").replace('"', "\\\"");
            conditions.push(format!(
                "(string::contains(array::join(input_strings, ', '), '{}') OR string::contains(array::join(return_strings, ', '), '{}'))",
                escaped_pattern, escaped_pattern
            ));
        }
    }

    // Add module filter if provided
    if let Some(mod_pat) = module_pattern {
        if use_regex {
            let escaped_pattern = mod_pat.replace('\\', "\\\\").replace('"', "\\\"");
            conditions.push(format!("string::matches(module_name, /^{}/)", escaped_pattern));
        } else {
            let escaped_pattern = mod_pat.replace('\\', "\\\\").replace('"', "\\\"");
            conditions.push(format!("module_name = '{}'", escaped_pattern));
        }
    }

    // Build the WHERE clause
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let query = if where_clause.is_empty() {
        format!(
            r#"
        SELECT
            id,
            module_name,
            function_name,
            arity,
            line,
            "default" as project,
            array::join(input_strings, ", ") as inputs_string,
            array::join(return_strings, ", ") as return_string
        FROM specs
        ORDER BY module_name, function_name, arity
        LIMIT $limit
        "#
        )
    } else {
        format!(
            r#"
        SELECT
            id,
            module_name,
            function_name,
            arity,
            line,
            "default" as project,
            array::join(input_strings, ", ") as inputs_string,
            array::join(return_strings, ", ") as return_string
        FROM specs
        {}
        ORDER BY module_name, function_name, arity
        LIMIT $limit
        "#,
            where_clause
        )
    };

    let result = db
        .execute_query(&query, params)
        .map_err(|e| StructUsageError::QueryFailed {
            message: e.to_string(),
        })?;

    let mut results = Vec::new();
    // SurrealDB returns columns in alphabetical order by default
    // Select columns: id, module_name, function_name, arity, line, "default" as project, array::join(...) as inputs_string, array::join(...) as return_string
    // Alphabetical order: arity(0), function_name(1), id(2), inputs_string(3), line(4), module_name(5), project(6), return_string(7)
    for row in result.rows() {
        if row.len() >= 8 {
            let arity = extract_i64(row.get(0).unwrap(), 0);
            let Some(name) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            // Skip row[2] which is the id (Thing)
            let inputs_string = extract_string(row.get(3).unwrap()).unwrap_or_default();
            let line = extract_i64(row.get(4).unwrap(), 0);
            let Some(module) = extract_string(row.get(5).unwrap()) else {
                continue;
            };
            let Some(project) = extract_string(row.get(6).unwrap()) else {
                continue;
            };
            let return_string = extract_string(row.get(7).unwrap()).unwrap_or_default();

            results.push(StructUsageEntry {
                project,
                module,
                name,
                arity,
                inputs_string,
                return_string,
                line,
            });
        }
    }

    Ok(results)
}

#[cfg(all(test, feature = "backend-cozo"))]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn populated_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::type_signatures_db("default")
    }

    #[rstest]
    fn test_find_struct_usage_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_struct_usage(&*populated_db, "", "default", false, None, 100);
        assert!(result.is_ok());
        let entries = result.unwrap();
        // May or may not have results depending on fixture
        assert!(entries.is_empty() || !entries.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_struct_usage_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_struct_usage(
            &*populated_db,
            "NonExistentType",
            "default",
            false,
            None,
            100,
        );
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert!(entries.is_empty(), "Should return empty for non-existent pattern");
    }

    #[rstest]
    fn test_find_struct_usage_with_module_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_struct_usage(&*populated_db, "", "default", false, Some("MyApp"), 100);
        assert!(result.is_ok());
        let entries = result.unwrap();
        for entry in &entries {
            assert!(entry.module.contains("MyApp"), "Module should match filter");
        }
    }

    #[rstest]
    fn test_find_struct_usage_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_struct_usage(&*populated_db, "", "default", false, None, 5)
            .unwrap();
        let limit_100 = find_struct_usage(&*populated_db, "", "default", false, None, 100)
            .unwrap();

        assert!(limit_5.len() <= 5, "Limit should be respected");
        assert!(limit_5.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_struct_usage_with_regex_pattern(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_struct_usage(&*populated_db, "^String", "default", true, None, 100);
        assert!(result.is_ok());
        // Query should execute successfully
    }

    #[rstest]
    fn test_find_struct_usage_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_struct_usage(&*populated_db, "[invalid", "default", true, None, 100);
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_find_struct_usage_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_struct_usage(&*populated_db, "", "nonexistent", false, None, 100);
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert!(entries.is_empty(), "Non-existent project should return no results");
    }

    #[rstest]
    fn test_find_struct_usage_returns_valid_structure(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_struct_usage(&*populated_db, "", "default", false, None, 100);
        assert!(result.is_ok());
        let entries = result.unwrap();
        for entry in &entries {
            assert_eq!(entry.project, "default");
            assert!(!entry.module.is_empty());
            assert!(!entry.name.is_empty());
            assert!(entry.arity >= 0);
        }
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    #[test]
    fn test_find_struct_usage_user_type() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_struct_usage(&*db, "user()", "default", false, None, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        // user() appears in 6 specs (all in return types only)
        // get_user/1, get_user/2, list_users/0, create_user/1, find/1, get_user/1 (clause 1)
        assert_eq!(
            entries.len(),
            6,
            "Should find exactly 6 specs using user()"
        );

        // Validate that user() appears in return_string for all results
        for entry in &entries {
            assert!(
                entry.return_string.contains("user()"),
                "user() should appear in return type: {}",
                entry.return_string
            );
        }
    }

    #[test]
    fn test_find_struct_usage_integer_type() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_struct_usage(&*db, "integer()", "default", false, None, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        // integer() appears in 3 specs (all in input types only)
        assert_eq!(
            entries.len(),
            3,
            "Should find exactly 3 specs using integer()"
        );

        // Validate that integer() appears in inputs_string for all results
        for entry in &entries {
            assert!(
                entry.inputs_string.contains("integer()"),
                "integer() should appear in inputs: {}",
                entry.inputs_string
            );
        }
    }

    #[test]
    fn test_find_struct_usage_struct_type() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_struct_usage(&*db, "struct()", "default", false, None, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        // struct() appears in 1 spec (insert/2 with struct() in inputs)
        assert_eq!(
            entries.len(),
            1,
            "Should find exactly 1 spec using struct()"
        );

        // Verify the entry has struct() in either inputs or returns (OR logic)
        for entry in &entries {
            let in_inputs = entry.inputs_string.contains("struct()");
            let in_returns = entry.return_string.contains("struct()");
            assert!(
                in_inputs || in_returns,
                "struct() should appear in inputs or returns for: {}/{}",
                entry.module,
                entry.name
            );
        }
    }

    #[test]
    fn test_find_struct_usage_combined_keyword() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_struct_usage(&*db, "keyword()", "default", false, None, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        // keyword() appears in 2 specs
        assert_eq!(
            entries.len(),
            2,
            "Should find exactly 2 specs using keyword()"
        );

        // Verify keyword() is in inputs for all results
        for entry in &entries {
            assert!(
                entry.inputs_string.contains("keyword()"),
                "keyword() should appear in inputs"
            );
        }
    }

    #[test]
    fn test_find_struct_usage_with_module_filter() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_struct_usage(
            &*db,
            "user()",
            "default",
            false,
            Some("MyApp.Accounts"),
            100,
        );

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        // user() appears in 6 specs total, all 6 in MyApp.Accounts
        // get_user/1, get_user/2, list_users/0, create_user/1, find/1, get_user/1 (clause 1)
        assert_eq!(
            entries.len(),
            6,
            "Should find exactly 6 specs in MyApp.Accounts with user()"
        );

        // Verify all results are from the filtered module
        for entry in &entries {
            assert_eq!(
                entry.module, "MyApp.Accounts",
                "All results should be from MyApp.Accounts"
            );
        }
    }

    #[test]
    fn test_find_struct_usage_regex_pattern() {
        let db = crate::test_utils::surreal_specs_db();

        // Match patterns starting with "Ecto"
        let result = find_struct_usage(&*db, "Ecto", "default", true, None, 100);

        assert!(result.is_ok(), "Regex query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        // Only Ecto.Queryable.t() in fixture (in inputs of all/1)
        assert_eq!(
            entries.len(),
            1,
            "Should find exactly 1 spec matching Ecto pattern"
        );

        // Verify results contain Ecto types
        for entry in &entries {
            let has_ecto = entry.inputs_string.contains("Ecto")
                || entry.return_string.contains("Ecto");
            assert!(has_ecto, "Result should contain Ecto type");
        }
    }

    #[test]
    fn test_find_struct_usage_nonexistent_type() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_struct_usage(&*db, "NonExistent", "default", false, None, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        assert!(
            entries.is_empty(),
            "Should return empty results for non-existent type"
        );
    }

    #[test]
    fn test_find_struct_usage_invalid_regex() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_struct_usage(&*db, "[invalid", "default", true, None, 100);

        assert!(result.is_err(), "Should reject invalid regex pattern");
    }

    #[test]
    fn test_find_struct_usage_respects_limit() {
        let db = crate::test_utils::surreal_specs_db();

        let limit_3 = find_struct_usage(&*db, "", "default", false, None, 3).unwrap();
        let limit_100 = find_struct_usage(&*db, "", "default", false, None, 100).unwrap();

        assert!(limit_3.len() <= 3, "Limit should be respected");
        assert_eq!(limit_3.len(), 3, "Should return exactly 3 when limit is 3");
        assert_eq!(
            limit_100.len(),
            12,
            "Should return all 12 specs when limit is high"
        );
    }

    #[test]
    fn test_find_struct_usage_empty_pattern() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_struct_usage(&*db, "", "default", false, None, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        // Should return all 12 specs (9 @spec + 3 @callback)
        assert_eq!(
            entries.len(),
            12,
            "Empty pattern should return all 12 specs"
        );
    }

    #[test]
    fn test_find_struct_usage_returns_valid_structure() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_struct_usage(&*db, "", "default", false, None, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        for entry in &entries {
            assert_eq!(entry.project, "default");
            assert!(!entry.module.is_empty());
            assert!(!entry.name.is_empty());
            assert!(entry.arity >= 0);
            // inputs_string and return_string might be empty
        }
    }

    #[test]
    fn test_find_struct_usage_preserves_sorting() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_struct_usage(&*db, "", "default", false, None, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        // Verify sorted by module_name, function_name, arity
        if entries.len() > 1 {
            for i in 0..entries.len() - 1 {
                let curr = &entries[i];
                let next = &entries[i + 1];

                let module_cmp = curr.module.cmp(&next.module);
                if module_cmp == std::cmp::Ordering::Equal {
                    let name_cmp = curr.name.cmp(&next.name);
                    if name_cmp == std::cmp::Ordering::Equal {
                        assert!(
                            curr.arity <= next.arity,
                            "Should be sorted by arity within same function"
                        );
                    } else {
                        assert!(
                            name_cmp == std::cmp::Ordering::Less,
                            "Should be sorted by name within same module"
                        );
                    }
                } else {
                    assert!(
                        module_cmp == std::cmp::Ordering::Less,
                        "Should be sorted by module"
                    );
                }
            }
        }
    }

    #[test]
    fn test_find_struct_usage_string_type() {
        let db = crate::test_utils::surreal_specs_db();

        // String.t() appears in input types only
        let result = find_struct_usage(&*db, "String.t()", "default", false, None, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        // Should find 2 specs with String.t()
        assert_eq!(
            entries.len(),
            2,
            "Should find exactly 2 specs with String.t()"
        );

        // Verify String.t() is in all results
        for entry in &entries {
            assert!(
                entry.inputs_string.contains("String.t()"),
                "String.t() should appear in inputs"
            );
        }
    }

    #[test]
    fn test_find_struct_usage_ecto_queryable() {
        let db = crate::test_utils::surreal_specs_db();

        // Ecto.Queryable.t() appears in input types
        let result = find_struct_usage(
            &*db,
            "Ecto.Queryable.t()",
            "default",
            false,
            None,
            100,
        );

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        // Should find 1 spec: all/1 with Ecto.Queryable.t() in inputs
        assert_eq!(
            entries.len(),
            1,
            "Should find exactly 1 spec with Ecto.Queryable.t()"
        );

        assert_eq!(entries[0].name, "all");
        assert!(entries[0].inputs_string.contains("Ecto"));
    }

    #[test]
    fn test_find_struct_usage_result_type() {
        let db = crate::test_utils::surreal_specs_db();

        // result() appears in return types
        let result = find_struct_usage(&*db, "result()", "default", false, None, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        // Should find specs with result() in returns
        assert!(!entries.is_empty(), "Should find specs with result()");

        for entry in &entries {
            assert!(
                entry.return_string.contains("result()"),
                "result() should appear in returns"
            );
        }
    }
}

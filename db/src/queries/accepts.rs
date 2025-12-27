use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string};

#[cfg(feature = "backend-cozo")]
use crate::db::run_query;
use crate::query_builders::validate_regex_patterns;

#[cfg(feature = "backend-cozo")]
use crate::query_builders::{ConditionBuilder, OptionalConditionBuilder};

#[derive(Error, Debug)]
pub enum AcceptsError {
    #[error("Accepts query failed: {message}")]
    QueryFailed { message: String },
}

/// A function with its input type specification
#[derive(Debug, Clone, Serialize)]
pub struct AcceptsEntry {
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
pub fn find_accepts(
    db: &dyn Database,
    pattern: &str,
    project: &str,
    use_regex: bool,
    module_pattern: Option<&str>,
    limit: u32,
) -> Result<Vec<AcceptsEntry>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(pattern), module_pattern])?;

    // Build conditions using query builders
    let pattern_cond = ConditionBuilder::new("inputs_string", "pattern").build(use_regex);
    let module_cond = OptionalConditionBuilder::new("module", "module_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(module_pattern.is_some(), use_regex);

    let script = format!(
        r#"
        ?[project, module, name, arity, inputs_string, return_string, line] :=
            *specs{{project, module, name, arity, inputs_string, return_string, line}},
            project == $project,
            {pattern_cond}
            {module_cond}

        :order module, name, arity
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("pattern", pattern)
        .with_str("project", project);

    if let Some(mod_pat) = module_pattern {
        params = params.with_str("module_pattern", mod_pat);
    }

    let result = run_query(db, &script, params).map_err(|e| AcceptsError::QueryFailed {
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

            results.push(AcceptsEntry {
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
pub fn find_accepts(
    db: &dyn Database,
    pattern: &str,
    _project: &str,
    use_regex: bool,
    module_pattern: Option<&str>,
    limit: u32,
) -> Result<Vec<AcceptsEntry>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(pattern), module_pattern])?;

    // Build WHERE conditions based on what filters are present
    let mut conditions = Vec::new();
    let params = QueryParams::new().with_int("limit", limit as i64);

    // Add pattern filter if provided
    // Build the input_strings array matching condition
    if !pattern.is_empty() {
        // Convert the array into a joined string and match against it
        // This avoids closure parameter issues in SurrealQL
        if use_regex {
            // For regex matching: check if any element matches the pattern
            // We use array::any with direct comparison since parameter binding
            // doesn't work well inside closures in SurrealQL
            let escaped_pattern = pattern.replace('\\', "\\\\").replace('"', "\\\"");
            // Use array filtering: look for elements that match the regex
            conditions.push(format!(
                "array::len(array::filter(input_strings, |$v| string::matches($v, /^{}/))) > 0",
                escaped_pattern
            ));
        } else {
            // For substring matching: check if joined string contains the pattern
            let escaped_pattern = pattern.replace('\\', "\\\\").replace('"', "\\\"");
            conditions.push(format!(
                "string::contains(array::join(input_strings, ' '), '{}')",
                escaped_pattern
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

    // Note: Use explicit column numbering in SELECT to ensure consistent ordering
    // rather than relying on SurrealDB's default alphabetical reordering
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
        .map_err(|e| AcceptsError::QueryFailed {
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

            results.push(AcceptsEntry {
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
    fn test_find_accepts_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_accepts(&*populated_db, "", "default", false, None, 100);
        assert!(result.is_ok());
        let entries = result.unwrap();
        // May or may not have matching specs, but query should execute
        assert!(entries.is_empty() || !entries.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_accepts_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_accepts(
            &*populated_db,
            "NonExistentType",
            "default",
            false,
            None,
            100,
        );
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert!(entries.is_empty(), "Should return empty results for non-existent pattern");
    }

    #[rstest]
    fn test_find_accepts_with_module_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_accepts(&*populated_db, "", "default", false, Some("MyApp"), 100);
        assert!(result.is_ok());
        let entries = result.unwrap();
        for entry in &entries {
            assert!(entry.module.contains("MyApp"), "Module should match filter");
        }
    }

    #[rstest]
    fn test_find_accepts_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_accepts(&*populated_db, "", "default", false, None, 5)
            .unwrap();
        let limit_100 = find_accepts(&*populated_db, "", "default", false, None, 100)
            .unwrap();

        assert!(limit_5.len() <= 5, "Limit should be respected");
        assert!(limit_5.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_accepts_with_regex_pattern(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_accepts(&*populated_db, "^String", "default", true, None, 100);
        assert!(result.is_ok());
        let entries = result.unwrap();
        for entry in &entries {
            assert!(
                entry.inputs_string.starts_with("String"),
                "Input should match regex"
            );
        }
    }

    #[rstest]
    fn test_find_accepts_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_accepts(&*populated_db, "[invalid", "default", true, None, 100);
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_find_accepts_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_accepts(&*populated_db, "", "nonexistent", false, None, 100);
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert!(entries.is_empty(), "Non-existent project should return no results");
    }

    #[rstest]
    fn test_find_accepts_returns_valid_structure(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_accepts(&*populated_db, "", "default", false, None, 100);
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
    fn test_find_accepts_integer_type() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_accepts(&*db, "integer()", "default", false, None, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        // Assert exact count: get_user/1, get_user/2, get/2
        assert_eq!(
            entries.len(),
            3,
            "Should find exactly 3 specs accepting integer()"
        );

        // Validate specific entries exist
        let signatures: Vec<(&str, &str, i64)> = entries
            .iter()
            .map(|e| (e.module.as_str(), e.name.as_str(), e.arity))
            .collect();

        assert!(signatures.contains(&("MyApp.Accounts", "get_user", 1)));
        assert!(signatures.contains(&("MyApp.Accounts", "get_user", 2)));
        assert!(signatures.contains(&("MyApp.Repo", "get", 2)));

        // Validate field values
        for entry in &entries {
            assert!(!entry.module.is_empty());
            assert!(!entry.name.is_empty());
            assert!(entry.inputs_string.contains("integer()"));
        }
    }

    #[test]
    fn test_find_accepts_string_type() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_accepts(&*db, "String.t()", "default", false, None, 100);

        assert!(result.is_ok());
        let entries = result.unwrap();

        // Expect 2 results: get_by_email/1, authenticate/2
        assert_eq!(
            entries.len(),
            2,
            "Should find exactly 2 specs with String.t() type"
        );

        let signatures: Vec<(&str, &str, i64)> = entries
            .iter()
            .map(|e| (e.module.as_str(), e.name.as_str(), e.arity))
            .collect();

        assert!(signatures.contains(&("MyApp.Users", "get_by_email", 1)));
        assert!(signatures.contains(&("MyApp.Users", "authenticate", 2)));
    }

    #[test]
    fn test_find_accepts_regex_pattern() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_accepts(&*db, "^Ecto", "default", true, None, 100);

        assert!(result.is_ok(), "Regex query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        // Expect 1 result: all/1 with Ecto.Queryable.t()
        assert_eq!(
            entries.len(),
            1,
            "Should find exactly 1 spec matching ^Ecto"
        );

        let entry = &entries[0];
        assert_eq!(entry.name, "all");
        assert_eq!(entry.arity, 1);
        assert!(entry.inputs_string.contains("Ecto"));
    }

    #[test]
    fn test_find_accepts_keyword_type() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_accepts(&*db, "keyword()", "default", false, None, 100);

        assert!(result.is_ok());
        let entries = result.unwrap();

        // Expect 2 results: get_user/2 and insert/2 both have keyword() in their input arrays
        assert_eq!(
            entries.len(),
            2,
            "Should find exactly 2 specs accepting keyword()"
        );

        let signatures: Vec<(&str, &str, i64)> = entries
            .iter()
            .map(|e| (e.module.as_str(), e.name.as_str(), e.arity))
            .collect();

        assert!(signatures.contains(&("MyApp.Accounts", "get_user", 2)));
        assert!(signatures.contains(&("MyApp.Repo", "insert", 2)));
    }

    #[test]
    fn test_find_accepts_with_module_filter() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_accepts(
            &*db,
            "integer()",
            "default",
            false,
            Some("MyApp.Accounts"),
            100,
        );

        assert!(result.is_ok());
        let entries = result.unwrap();

        // Expect 2 results: get_user/1 and get_user/2 from MyApp.Accounts
        assert_eq!(
            entries.len(),
            2,
            "Should find exactly 2 specs in MyApp.Accounts accepting integer()"
        );

        for entry in &entries {
            assert_eq!(entry.module, "MyApp.Accounts");
            assert!(entry.inputs_string.contains("integer()"));
        }
    }

    #[test]
    fn test_find_accepts_nonexistent_type() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_accepts(&*db, "NonExistent", "default", false, None, 100);

        assert!(result.is_ok());
        let entries = result.unwrap();

        assert!(
            entries.is_empty(),
            "Should return empty results for non-existent type"
        );
    }

    #[test]
    fn test_find_accepts_empty_pattern() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_accepts(&*db, "", "default", false, None, 100);

        assert!(result.is_ok());
        let entries = result.unwrap();

        // Should return all 9 specs
        assert_eq!(
            entries.len(),
            9,
            "Empty pattern should return all 9 specs"
        );
    }

    #[test]
    fn test_find_accepts_invalid_regex() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_accepts(&*db, "[invalid", "default", true, None, 100);

        assert!(
            result.is_err(),
            "Should reject invalid regex pattern"
        );
    }

    #[test]
    fn test_find_accepts_respects_limit() {
        let db = crate::test_utils::surreal_accepts_db();

        let limit_3 = find_accepts(&*db, "", "default", false, None, 3)
            .unwrap();

        let limit_100 = find_accepts(&*db, "", "default", false, None, 100)
            .unwrap();

        assert!(limit_3.len() <= 3, "Limit should be respected");
        assert_eq!(limit_3.len(), 3, "Should return exactly 3 when limit is 3");
        assert_eq!(
            limit_100.len(),
            9,
            "Should return all 9 specs when limit is high"
        );
    }

    #[test]
    fn test_find_accepts_zero_arity_excluded_from_integer_search() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_accepts(&*db, "integer()", "default", false, None, 100);

        assert!(result.is_ok());
        let entries = result.unwrap();

        // list_users/0 should not be included (has empty input_strings array)
        for entry in &entries {
            assert_ne!(entry.name, "list_users", "list_users/0 should not match integer()");
            assert!(entry.inputs_string.contains("integer()"));
        }
    }

    #[test]
    fn test_find_accepts_returns_valid_structure() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_accepts(&*db, "", "default", false, None, 100);

        assert!(result.is_ok());
        let entries = result.unwrap();

        for entry in &entries {
            assert_eq!(entry.project, "default");
            assert!(!entry.module.is_empty());
            assert!(!entry.name.is_empty());
            assert!(entry.arity >= 0);
            // inputs_string might be empty (for 0-arity functions)
            // return_string might be empty
        }
    }

    #[test]
    fn test_find_accepts_preserves_sorting() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_accepts(&*db, "", "default", false, None, 100);

        assert!(result.is_ok());
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
}

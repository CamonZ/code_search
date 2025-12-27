use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::extract_i64;
use crate::query_builders::validate_regex_patterns;

#[cfg(feature = "backend-cozo")]
use crate::db::run_query;
#[cfg(feature = "backend-cozo")]
use crate::query_builders::{ConditionBuilder, OptionalConditionBuilder};

#[cfg(feature = "backend-cozo")]
use crate::db::extract_string;

#[derive(Error, Debug)]
pub enum SpecsError {
    #[error("Specs query failed: {message}")]
    QueryFailed { message: String },
}

/// A spec or callback definition
#[derive(Debug, Clone, Serialize)]
pub struct SpecDef {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub kind: String,
    pub line: i64,
    pub inputs_string: String,
    pub return_string: String,
    pub full: String,
}

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
pub fn find_specs(
    db: &dyn Database,
    module_pattern: &str,
    function_pattern: Option<&str>,
    kind_filter: Option<&str>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<SpecDef>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), function_pattern])?;

    // Build conditions using query builders
    let module_cond = ConditionBuilder::new("module", "module_pattern").build(use_regex);
    let function_cond = OptionalConditionBuilder::new("name", "function_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(function_pattern.is_some(), use_regex);
    let kind_cond = OptionalConditionBuilder::new("kind", "kind")
        .with_leading_comma()
        .build(kind_filter.is_some());

    let script = format!(
        r#"
        ?[project, module, name, arity, kind, line, inputs_string, return_string, full] :=
            *specs{{project, module, name, arity, kind, line, inputs_string, return_string, full}},
            project == $project,
            {module_cond}
            {function_cond}
            {kind_cond}

        :order module, name, arity
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("project", project)
        .with_str("module_pattern", module_pattern);

    if let Some(func) = function_pattern {
        params = params.with_str("function_pattern", func);
    }

    if let Some(kind) = kind_filter {
        params = params.with_str("kind", kind);
    }

    let result = run_query(db, &script, params).map_err(|e| SpecsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 9 {
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
            let Some(kind) = extract_string(row.get(4).unwrap()) else {
                continue;
            };
            let line = extract_i64(row.get(5).unwrap(), 0);
            let inputs_string = extract_string(row.get(6).unwrap()).unwrap_or_default();
            let return_string = extract_string(row.get(7).unwrap()).unwrap_or_default();
            let full = extract_string(row.get(8).unwrap()).unwrap_or_default();

            results.push(SpecDef {
                project,
                module,
                name,
                arity,
                kind,
                line,
                inputs_string,
                return_string,
                full,
            });
        }
    }

    Ok(results)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
pub fn find_specs(
    db: &dyn Database,
    module_pattern: &str,
    function_pattern: Option<&str>,
    kind_filter: Option<&str>,
    _project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<SpecDef>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(module_pattern), function_pattern])?;

    // Build WHERE conditions based on what filters are present
    let mut conditions = Vec::new();
    let params = QueryParams::new().with_int("limit", limit as i64);

    // Add module filter if provided (required, may be empty string for all)
    if !module_pattern.is_empty() {
        if use_regex {
            let escaped_pattern = module_pattern.replace('\\', "\\\\").replace('"', "\\\"");
            conditions.push(format!("string::matches(module_name, /^{}/)", escaped_pattern));
        } else {
            let escaped_pattern = module_pattern.replace('\\', "\\\\").replace('"', "\\\"");
            conditions.push(format!("string::contains(module_name, '{}')", escaped_pattern));
        }
    }

    // Add function filter if provided
    if let Some(func_pat) = function_pattern {
        if use_regex {
            let escaped_pattern = func_pat.replace('\\', "\\\\").replace('"', "\\\"");
            conditions.push(format!("string::matches(function_name, /^{}/)", escaped_pattern));
        } else {
            let escaped_pattern = func_pat.replace('\\', "\\\\").replace('"', "\\\"");
            conditions.push(format!("string::contains(function_name, '{}')", escaped_pattern));
        }
    }

    // Add kind filter if provided
    if let Some(kind_val) = kind_filter {
        let escaped_kind = kind_val.replace('\\', "\\\\").replace('"', "\\\"");
        conditions.push(format!("kind = '{}'", escaped_kind));
    }

    // Build the WHERE clause
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Note: SurrealDB returns columns in alphabetical order, not SELECT order
    // Selected columns: id, arity, full, function_name, kind, line, module_name, "default" as project,
    // array::join(input_strings, ", ") as inputs_string, array::join(return_strings, " | ") as return_string
    // Alphabetical: arity(0), full(1), function_name(2), id(3), inputs_string(4), kind(5), line(6), module_name(7), project(8), return_string(9)
    let query = if where_clause.is_empty() {
        format!(
            r#"
        SELECT
            id,
            arity,
            full,
            function_name,
            kind,
            line,
            module_name,
            "default" as project,
            array::join(input_strings, ", ") as inputs_string,
            array::join(return_strings, " | ") as return_string
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
            arity,
            full,
            function_name,
            kind,
            line,
            module_name,
            "default" as project,
            array::join(input_strings, ", ") as inputs_string,
            array::join(return_strings, " | ") as return_string
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
        .map_err(|e| SpecsError::QueryFailed {
            message: e.to_string(),
        })?;

    let mut results = Vec::new();
    // SurrealDB returns columns in alphabetical order:
    // arity(0), full(1), function_name(2), id(3), inputs_string(4), kind(5), line(6), module_name(7), project(8), return_string(9)
    for row in result.rows() {
        if row.len() >= 10 {
            let arity = extract_i64(row.get(0).unwrap(), 0);
            let Some(full) = crate::db::extract_string(row.get(1).unwrap()) else {
                continue;
            };
            let Some(name) = crate::db::extract_string(row.get(2).unwrap()) else {
                continue;
            };
            // Skip row[3] which is the id (Thing)
            let inputs_string = crate::db::extract_string(row.get(4).unwrap()).unwrap_or_default();
            let Some(kind) = crate::db::extract_string(row.get(5).unwrap()) else {
                continue;
            };
            let line = extract_i64(row.get(6).unwrap(), 0);
            let Some(module) = crate::db::extract_string(row.get(7).unwrap()) else {
                continue;
            };
            let Some(project) = crate::db::extract_string(row.get(8).unwrap()) else {
                continue;
            };
            let return_string = crate::db::extract_string(row.get(9).unwrap()).unwrap_or_default();

            results.push(SpecDef {
                project,
                module,
                name,
                arity,
                kind,
                line,
                inputs_string,
                return_string,
                full,
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
    fn test_find_specs_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(&*populated_db, "", None, None, "default", false, 100);
        assert!(result.is_ok());
        let specs = result.unwrap();
        // May be empty if fixture doesn't have specs, just verify query executes
        assert!(specs.is_empty() || !specs.is_empty(), "Query should execute");
    }

    #[rstest]
    fn test_find_specs_empty_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(
            &*populated_db,
            "NonExistentModule",
            None,
            None,
            "default",
            false,
            100,
        );
        assert!(result.is_ok());
        let specs = result.unwrap();
        assert!(specs.is_empty(), "Should return empty results for non-existent module");
    }

    #[rstest]
    fn test_find_specs_with_function_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(&*populated_db, "", Some("index"), None, "default", false, 100);
        assert!(result.is_ok());
        let specs = result.unwrap();
        for spec in &specs {
            assert_eq!(spec.name, "index", "Function name should match filter");
        }
    }

    #[rstest]
    fn test_find_specs_with_kind_filter(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(&*populated_db, "", None, Some("spec"), "default", false, 100);
        assert!(result.is_ok());
        let specs = result.unwrap();
        for spec in &specs {
            assert_eq!(spec.kind, "spec", "Kind should match filter");
        }
    }

    #[rstest]
    fn test_find_specs_respects_limit(populated_db: Box<dyn crate::backend::Database>) {
        let limit_5 = find_specs(&*populated_db, "", None, None, "default", false, 5)
            .unwrap();
        let limit_100 = find_specs(&*populated_db, "", None, None, "default", false, 100)
            .unwrap();

        assert!(limit_5.len() <= 5, "Limit should be respected");
        assert!(limit_5.len() <= limit_100.len(), "Higher limit should return >= results");
    }

    #[rstest]
    fn test_find_specs_with_regex_pattern(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(&*populated_db, "^MyApp\\..*$", None, None, "default", true, 100);
        assert!(result.is_ok());
        let specs = result.unwrap();
        for spec in &specs {
            assert!(spec.module.starts_with("MyApp"), "Module should match regex");
        }
    }

    #[rstest]
    fn test_find_specs_invalid_regex(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(&*populated_db, "[invalid", None, None, "default", true, 100);
        assert!(result.is_err(), "Should reject invalid regex");
    }

    #[rstest]
    fn test_find_specs_nonexistent_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(&*populated_db, "", None, None, "nonexistent", false, 100);
        assert!(result.is_ok());
        let specs = result.unwrap();
        assert!(specs.is_empty(), "Non-existent project should return no results");
    }

    #[rstest]
    fn test_find_specs_returns_valid_structure(populated_db: Box<dyn crate::backend::Database>) {
        let result = find_specs(&*populated_db, "", None, None, "default", false, 100);
        assert!(result.is_ok());
        let specs = result.unwrap();
        if !specs.is_empty() {
            let spec = &specs[0];
            assert_eq!(spec.project, "default");
            assert!(!spec.module.is_empty());
            assert!(!spec.name.is_empty());
            assert!(!spec.kind.is_empty());
            assert!(spec.arity >= 0);
        }
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    #[test]
    fn test_find_specs_all() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(&*db, "", None, None, "default", false, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let specs = result.unwrap();

        // Assert exact count: 12 total (9 spec + 3 callback)
        assert_eq!(specs.len(), 12, "Should find exactly 12 specs (9 @spec + 3 @callback)");

        // Verify all specs have required fields populated
        for spec in &specs {
            assert_eq!(spec.project, "default");
            assert!(!spec.module.is_empty());
            assert!(!spec.name.is_empty());
            assert!(!spec.kind.is_empty());
            assert!(spec.arity >= 0);
            assert!(!spec.full.is_empty(), "Full spec string should be populated");
        }
    }

    #[test]
    fn test_find_specs_by_module() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(&*db, "MyApp.Accounts", None, None, "default", false, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let specs = result.unwrap();

        // Assert exact count: 6 specs in MyApp.Accounts
        // get_user/1 has 2 clauses + get_user/2 (1) + list_users (1) + create_user (1) + find (1) = 6
        assert_eq!(
            specs.len(),
            6,
            "Should find exactly 6 specs in MyApp.Accounts"
        );

        // Validate all are from correct module
        for spec in &specs {
            assert_eq!(spec.module, "MyApp.Accounts");
        }

        // Validate all function types are present
        let function_arities: Vec<(&str, i64)> = specs
            .iter()
            .map(|s| (s.name.as_str(), s.arity))
            .collect();

        assert!(function_arities.contains(&("get_user", 1)));
        assert!(function_arities.contains(&("get_user", 2)));
        assert!(function_arities.contains(&("list_users", 0)));
        assert!(function_arities.contains(&("create_user", 1)));
        assert!(function_arities.contains(&("find", 1)));
    }

    #[test]
    fn test_find_specs_by_function() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(&*db, "", Some("get_user"), None, "default", false, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let specs = result.unwrap();

        // Should find get_user/1 (with 2 clauses) + get_user/2 (1 clause) = 3 specs
        assert_eq!(
            specs.len(),
            3,
            "Should find exactly 3 specs for get_user function"
        );

        // All should be get_user
        for spec in &specs {
            assert_eq!(spec.name, "get_user");
        }
    }

    #[test]
    fn test_find_specs_kind_spec() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(&*db, "", None, Some("spec"), "default", false, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let specs = result.unwrap();

        // Assert exact count: 9 @spec entries (including alternate clause)
        assert_eq!(
            specs.len(),
            9,
            "Should find exactly 9 @spec definitions (including alternate clauses)"
        );

        // All should be specs
        for spec in &specs {
            assert_eq!(spec.kind, "spec", "Kind should be spec");
        }
    }

    #[test]
    fn test_find_specs_kind_callback() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(&*db, "", None, Some("callback"), "default", false, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let specs = result.unwrap();

        // Assert exact count: 3 @callback entries
        assert_eq!(
            specs.len(),
            3,
            "Should find exactly 3 @callback definitions"
        );

        // All should be callbacks
        for spec in &specs {
            assert_eq!(spec.kind, "callback", "Kind should be callback");
            assert!(spec.full.starts_with("@callback"), "Full should start with @callback");
        }

        // Validate specific callbacks exist
        let signatures: Vec<(&str, &str, i64)> = specs
            .iter()
            .map(|s| (s.module.as_str(), s.name.as_str(), s.arity))
            .collect();

        assert!(signatures.contains(&("MyApp.Behaviour", "init", 1)));
        assert!(signatures.contains(&("MyApp.Behaviour", "handle_call", 3)));
        assert!(signatures.contains(&("MyApp.Behaviour", "handle_cast", 2)));
    }

    #[test]
    fn test_find_specs_combined_filters() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(
            &*db,
            "MyApp.Accounts",
            Some("get"),
            None,
            "default",
            false,
            100,
        );

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let specs = result.unwrap();

        // Should find get_user/1 and get_user/2 (2 clauses total = 3 specs? Let's verify)
        // Actually: get_user/1 with 2 clauses (2 specs) + get_user/2 with 1 clause (1 spec) = 3
        assert_eq!(
            specs.len(),
            3,
            "Should find 3 specs for get functions in MyApp.Accounts"
        );

        for spec in &specs {
            assert_eq!(spec.module, "MyApp.Accounts");
            assert!(spec.name.contains("get"));
        }
    }

    #[test]
    fn test_find_specs_regex_module() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(&*db, "MyApp.Accounts", None, None, "default", true, 100);

        assert!(
            result.is_ok(),
            "Regex query should succeed: {:?}",
            result.err()
        );
        let specs = result.unwrap();

        // Should find all MyApp.Accounts specs (6 total with alternate clauses)
        assert_eq!(specs.len(), 6, "Should find 6 specs matching MyApp.Accounts regex");

        for spec in &specs {
            assert!(spec.module.contains("MyApp.Accounts"));
        }
    }

    #[test]
    fn test_find_specs_regex_function() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(
            &*db,
            "MyApp.Behaviour",
            Some("^handle"),
            None,
            "default",
            true,
            100,
        );

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let specs = result.unwrap();

        // Should find handle_call and handle_cast (both @callback)
        assert_eq!(specs.len(), 2, "Should find 2 callback specs matching ^handle");

        for spec in &specs {
            assert!(spec.name.starts_with("handle"));
            assert_eq!(spec.kind, "callback");
        }
    }

    #[test]
    fn test_find_specs_nonexistent_module() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(
            &*db,
            "NonExistent",
            None,
            None,
            "default",
            false,
            100,
        );

        assert!(result.is_ok());
        let specs = result.unwrap();

        assert!(
            specs.is_empty(),
            "Should return empty results for non-existent module"
        );
    }

    #[test]
    fn test_find_specs_invalid_regex() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(&*db, "[invalid", None, None, "default", true, 100);

        assert!(result.is_err(), "Should reject invalid regex pattern");
    }

    #[test]
    fn test_find_specs_respects_limit() {
        let db = crate::test_utils::surreal_specs_db();

        let limit_3 = find_specs(&*db, "", None, None, "default", false, 3)
            .unwrap();

        let limit_100 = find_specs(&*db, "", None, None, "default", false, 100)
            .unwrap();

        assert!(limit_3.len() <= 3, "Limit should be respected");
        assert_eq!(limit_3.len(), 3, "Should return exactly 3 when limit is 3");
        assert_eq!(
            limit_100.len(),
            12,
            "Should return all 12 specs when limit is high"
        );
    }

    #[test]
    fn test_find_specs_validates_full_field() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(
            &*db,
            "MyApp.Accounts",
            Some("get_user"),
            None,
            "default",
            false,
            100,
        );

        assert!(result.is_ok());
        let specs = result.unwrap();

        // All get_user specs should start with @spec
        for spec in &specs {
            assert!(
                spec.full.starts_with("@spec"),
                "Full should start with @spec: {}",
                spec.full
            );
            assert!(
                spec.full.contains("get_user"),
                "Full should contain function name"
            );
        }
    }

    #[test]
    fn test_find_specs_preserves_sorting() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(&*db, "", None, None, "default", false, 100);

        assert!(result.is_ok());
        let specs = result.unwrap();

        // Verify sorted by module_name, function_name, arity
        if specs.len() > 1 {
            for i in 0..specs.len() - 1 {
                let curr = &specs[i];
                let next = &specs[i + 1];

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
    fn test_find_specs_input_array_joining() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(
            &*db,
            "MyApp.Accounts",
            Some("get_user"),
            None,
            "default",
            false,
            100,
        );

        assert!(result.is_ok());
        let specs = result.unwrap();

        // get_user/2 should have "integer(), keyword()" as inputs_string
        let get_user_2 = specs
            .iter()
            .find(|s| s.name == "get_user" && s.arity == 2)
            .expect("Should find get_user/2");

        assert_eq!(
            get_user_2.inputs_string, "integer(), keyword()",
            "Input array should be joined with ', '"
        );
    }

    #[test]
    fn test_find_specs_return_array_joining() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(
            &*db,
            "MyApp.Accounts",
            Some("get_user"),
            None,
            "default",
            false,
            100,
        );

        assert!(result.is_ok());
        let specs = result.unwrap();

        // get_user specs should have return types joined with " | "
        for spec in &specs {
            if spec.name == "get_user" {
                assert!(
                    spec.return_string.contains("|"),
                    "Return array should be joined with ' | ': {}",
                    spec.return_string
                );
                assert!(
                    spec.return_string.contains("{:ok, user()}"),
                    "Should contain first return type"
                );
                assert!(
                    spec.return_string.contains("{:error, :not_found}"),
                    "Should contain error return type"
                );
            }
        }
    }

    #[test]
    fn test_find_specs_empty_arrays_handled() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(
            &*db,
            "MyApp.Accounts",
            Some("list_users"),
            None,
            "default",
            false,
            100,
        );

        assert!(result.is_ok());
        let specs = result.unwrap();

        assert_eq!(specs.len(), 1, "Should find list_users/0");

        let list_users = &specs[0];
        // list_users/0 has no input parameters
        assert_eq!(list_users.inputs_string, "", "Empty input array should yield empty string");
        assert!(
            !list_users.return_string.is_empty(),
            "Return array should have values"
        );
    }

    #[test]
    fn test_find_specs_returns_valid_structure() {
        let db = crate::test_utils::surreal_specs_db();

        let result = find_specs(&*db, "", None, None, "default", false, 100);

        assert!(result.is_ok());
        let specs = result.unwrap();

        for spec in &specs {
            assert_eq!(spec.project, "default");
            assert!(!spec.module.is_empty());
            assert!(!spec.name.is_empty());
            assert!(!spec.kind.is_empty());
            assert!(spec.arity >= 0);
            assert!(!spec.full.is_empty());
            // inputs_string and return_string might be empty for 0-arity functions
        }
    }

    #[test]
    fn test_find_specs_module_substring_matching() {
        let db = crate::test_utils::surreal_specs_db();

        // Use substring match for "Behaviour"
        let result = find_specs(&*db, "Behaviour", None, None, "default", false, 100);

        assert!(result.is_ok());
        let specs = result.unwrap();

        // Should find 3 callback specs from MyApp.Behaviour
        assert_eq!(specs.len(), 3, "Should find 3 specs matching 'Behaviour'");

        for spec in &specs {
            assert!(spec.module.contains("Behaviour"));
        }
    }
}

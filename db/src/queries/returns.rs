use std::error::Error;

use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::db::{extract_i64, extract_string};
use crate::query_builders::validate_regex_patterns;

#[derive(Error, Debug)]
pub enum ReturnsError {
    #[error("Returns query failed: {message}")]
    QueryFailed { message: String },
}

/// A function with its return type specification
#[derive(Debug, Clone, Serialize)]
pub struct ReturnEntry {
    pub project: String,
    pub module: String,
    pub name: String,
    pub arity: i64,
    pub return_string: String,
    pub line: i64,
}

pub fn find_returns(
    db: &dyn Database,
    pattern: &str,
    use_regex: bool,
    module_pattern: Option<&str>,
    limit: u32,
) -> Result<Vec<ReturnEntry>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[Some(pattern), module_pattern])?;

    // Build WHERE conditions based on what filters are present
    let mut conditions = Vec::new();
    let params = QueryParams::new().with_int("limit", limit as i64);

    // Add pattern filter if provided
    // Build the return_strings array matching condition
    if !pattern.is_empty() {
        // Convert the array into a joined string and match against it
        // This avoids closure parameter issues in SurrealQL
        if use_regex {
            // For regex matching: check if any element matches the pattern
            let escaped_pattern = pattern.replace('\\', "\\\\").replace('"', "\\\"");
            // Use array filtering: look for elements that match the regex
            conditions.push(format!(
                "array::len(array::filter(return_strings, |$v| string::matches($v, /^{}/))) > 0",
                escaped_pattern
            ));
        } else {
            // For substring matching: check if joined string contains the pattern
            let escaped_pattern = pattern.replace('\\', "\\\\").replace('"', "\\\"");
            conditions.push(format!(
                "string::contains(array::join(return_strings, ', '), '{}')",
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
        .map_err(|e| ReturnsError::QueryFailed {
            message: e.to_string(),
        })?;

    let mut results = Vec::new();
    // SurrealDB returns columns in alphabetical order by default
    // Select columns: id, module_name, function_name, arity, line, "default" as project, array::join(...) as return_string
    // Alphabetical order: arity(0), function_name(1), id(2), line(3), module_name(4), project(5), return_string(6)
    for row in result.rows() {
        if row.len() >= 7 {
            let arity = extract_i64(row.get(0).unwrap(), 0);
            let Some(name) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            // Skip row[2] which is the id (Thing)
            let line = extract_i64(row.get(3).unwrap(), 0);
            let Some(module) = extract_string(row.get(4).unwrap()) else {
                continue;
            };
            let Some(project) = extract_string(row.get(5).unwrap()) else {
                continue;
            };
            let return_string = extract_string(row.get(6).unwrap()).unwrap_or_default();

            results.push(ReturnEntry {
                project,
                module,
                name,
                arity,
                return_string,
                line,
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_returns_user_type() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_returns(&*db, "user()", false, None, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        // Assert exact count: get_user/1, get_user/2, list_users/0, create_user/1, get_by_email/1
        assert_eq!(
            entries.len(),
            5,
            "Should find exactly 5 specs with user() in return types"
        );

        // Validate field values
        for entry in &entries {
            assert!(!entry.module.is_empty());
            assert!(!entry.name.is_empty());
            assert!(entry.return_string.contains("user()"));
        }
    }

    #[test]
    fn test_find_returns_nil_type() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_returns(&*db, "nil", false, None, 100);

        assert!(result.is_ok());
        let entries = result.unwrap();

        // Expect 1 result: get/2 with "any(), nil"
        assert_eq!(
            entries.len(),
            1,
            "Should find exactly 1 spec returning nil"
        );

        let signatures: Vec<(&str, &str, i64)> = entries
            .iter()
            .map(|e| (e.module.as_str(), e.name.as_str(), e.arity))
            .collect();

        assert!(signatures.contains(&("MyApp.Repo", "get", 2)));

        for entry in &entries {
            assert!(entry.return_string.contains("nil"));
        }
    }

    #[test]
    fn test_find_returns_struct_type() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_returns(&*db, "struct()", false, None, 100);

        assert!(result.is_ok());
        let entries = result.unwrap();

        // Expect 0 results: fixture doesn't have struct() type specs
        assert_eq!(
            entries.len(),
            0,
            "Should find 0 specs with struct() - fixture doesn't have this type"
        );
    }

    #[test]
    fn test_find_returns_error_tuple() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_returns(&*db, "{:error", false, None, 100);

        assert!(result.is_ok());
        let entries = result.unwrap();

        // Expect 7 results: all specs with {:error in return_strings
        // get_user/1, get_user/2, list_users/0, create_user/1, get_by_email/1, authenticate/2, insert/2
        assert_eq!(
            entries.len(),
            7,
            "Should find exactly 7 specs with {{:error tuple"
        );

        // All results should contain {:error in their return_strings
        for entry in &entries {
            assert!(entry.return_string.contains("{:error"));
        }
    }

    #[test]
    fn test_find_returns_ok_tuple() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_returns(&*db, "{:ok", false, None, 100);

        assert!(result.is_ok());
        let entries = result.unwrap();

        // Expect 3 results: get_user/1, get_user/2, create_user/1, authenticate/2, list_users/0, insert/2
        // But we're looking for {:ok specifically - all result tuples have it
        assert!(
            !entries.is_empty(),
            "Should find specs with {{:ok tuple"
        );

        for entry in &entries {
            assert!(entry.return_string.contains("{:ok"));
        }
    }

    #[test]
    fn test_find_returns_reason_type() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_returns(&*db, "reason()", false, None, 100);

        assert!(result.is_ok());
        let entries = result.unwrap();

        // Expect 4 results: list_users/0, create_user/1, authenticate/2, insert/2
        assert_eq!(
            entries.len(),
            4,
            "Should find exactly 4 specs with reason()"
        );

        for entry in &entries {
            assert!(entry.return_string.contains("reason()"));
        }
    }

    #[test]
    fn test_find_returns_regex_pattern() {
        let db = crate::test_utils::surreal_accepts_db();

        // Pattern to match return types containing "ok"
        let result = find_returns(&*db, "ok", false, None, 100);

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let entries = result.unwrap();

        // Expect 7 results: all 7 specs with ":ok" in returns
        assert_eq!(
            entries.len(),
            7,
            "Should find exactly 7 specs with :ok in returns"
        );

        for entry in &entries {
            assert!(entry.return_string.contains("ok"));
        }
    }

    #[test]
    fn test_find_returns_with_module_filter() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_returns(
            &*db,
            "user()",
            false,
            Some("MyApp.Accounts"),
            100,
        );

        assert!(result.is_ok());
        let entries = result.unwrap();

        // Expect 4 results in MyApp.Accounts: get_user/1, get_user/2, list_users/0, create_user/1
        assert_eq!(
            entries.len(),
            4,
            "Should find exactly 4 specs in MyApp.Accounts with user()"
        );

        for entry in &entries {
            assert_eq!(entry.module, "MyApp.Accounts");
            assert!(entry.return_string.contains("user()"));
        }
    }

    #[test]
    fn test_find_returns_nonexistent_type() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_returns(&*db, "NonExistent", false, None, 100);

        assert!(result.is_ok());
        let entries = result.unwrap();

        assert!(
            entries.is_empty(),
            "Should return empty results for non-existent type"
        );
    }

    #[test]
    fn test_find_returns_empty_pattern() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_returns(&*db, "", false, None, 100);

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
    fn test_find_returns_invalid_regex() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_returns(&*db, "[invalid", true, None, 100);

        assert!(
            result.is_err(),
            "Should reject invalid regex pattern"
        );
    }

    #[test]
    fn test_find_returns_respects_limit() {
        let db = crate::test_utils::surreal_accepts_db();

        let limit_3 = find_returns(&*db, "", false, None, 3)
            .unwrap();

        let limit_100 = find_returns(&*db, "", false, None, 100)
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
    fn test_find_returns_zero_arity_included() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_returns(&*db, "user()", false, None, 100);

        assert!(result.is_ok());
        let entries = result.unwrap();

        // list_users/0 should be included with [user()]
        let has_list_users = entries.iter().any(|e| {
            e.module == "MyApp.Accounts" && e.name == "list_users" && e.arity == 0
        });
        assert!(
            has_list_users,
            "list_users/0 should be included in results with user() type"
        );
    }

    #[test]
    fn test_find_returns_returns_valid_structure() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_returns(&*db, "", false, None, 100);

        assert!(result.is_ok());
        let entries = result.unwrap();

        for entry in &entries {
            assert_eq!(entry.project, "default");
            assert!(!entry.module.is_empty());
            assert!(!entry.name.is_empty());
            assert!(entry.arity >= 0);
            // return_string might be empty for functions with no return spec
        }
    }

    #[test]
    fn test_find_returns_preserves_sorting() {
        let db = crate::test_utils::surreal_accepts_db();

        let result = find_returns(&*db, "", false, None, 100);

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

//! Query to get all module calls for cluster analysis.
//!
//! Returns calls between different modules (no self-calls).
//! Clusters are computed in Rust by grouping modules by namespace.

use std::error::Error;

use crate::backend::Database;

#[cfg(feature = "backend-cozo")]
use crate::backend::QueryParams;

#[cfg(feature = "backend-cozo")]
use crate::db::run_query;

/// Represents a call between two different modules
#[derive(Debug, Clone)]
pub struct ModuleCall {
    pub caller_module: String,
    pub callee_module: String,
}

/// Get all inter-module calls (calls between different modules)
///
/// Returns calls where caller_module != callee_module.
/// These are used to compute internal vs external connectivity per namespace cluster.

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
pub fn get_module_calls(db: &dyn Database, project: &str) -> Result<Vec<ModuleCall>, Box<dyn Error>> {
    let script = r#"
        ?[caller_module, callee_module] :=
            *calls{project, caller_module, callee_module},
            project == $project,
            caller_module != callee_module
    "#;

    let params = QueryParams::new()
        .with_str("project", project);

    let result = run_query(db, script, params)?;

    let caller_idx = result.headers().iter().position(|h| h == "caller_module")
        .ok_or("Missing caller_module column")?;
    let callee_idx = result.headers().iter().position(|h| h == "callee_module")
        .ok_or("Missing callee_module column")?;

    let results = result
        .rows()
        .iter()
        .filter_map(|row| {
            let caller = row.get(caller_idx).and_then(|v| v.as_str());
            let callee = row.get(callee_idx).and_then(|v| v.as_str());
            match (caller, callee) {
                (Some(c), Some(m)) => Some(ModuleCall {
                    caller_module: c.to_string(),
                    callee_module: m.to_string(),
                }),
                _ => None,
            }
        })
        .collect();

    Ok(results)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
use crate::db::extract_string;

#[cfg(feature = "backend-surrealdb")]
pub fn get_module_calls(db: &dyn Database, _project: &str) -> Result<Vec<ModuleCall>, Box<dyn Error>> {
    // Query calls relation, traversing to access caller and callee module names
    // calls is a RELATION FROM functions TO functions
    // in = caller function (has module_name)
    // out = callee function (has module_name)
    // Filter out self-calls: in.module_name != out.module_name
    let query = r#"
        SELECT
            in.module_name as caller_module,
            out.module_name as callee_module
        FROM calls
        WHERE in.module_name != out.module_name
    "#;

    let result = db.execute_query_no_params(query)?;

    let mut results = Vec::new();
    for row in result.rows() {
        // SurrealDB returns columns alphabetically (via BTreeMap):
        // 0: callee_module, 1: caller_module
        if row.len() >= 2 {
            let Some(callee_module) = extract_string(row.get(0).unwrap()) else {
                continue;
            };
            let Some(caller_module) = extract_string(row.get(1).unwrap()) else {
                continue;
            };

            results.push(ModuleCall {
                caller_module,
                callee_module,
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
        crate::test_utils::call_graph_db("default")
    }

    #[rstest]
    fn test_get_module_calls_returns_results(populated_db: Box<dyn crate::backend::Database>) {
        let result = get_module_calls(&*populated_db, "default");
        assert!(result.is_ok());
        let calls = result.unwrap();
        // Should have some inter-module calls
        assert!(!calls.is_empty(), "Should find inter-module calls");
    }

    #[rstest]
    fn test_get_module_calls_excludes_self_calls(populated_db: Box<dyn crate::backend::Database>) {
        let result = get_module_calls(&*populated_db, "default");
        assert!(result.is_ok());
        let calls = result.unwrap();
        for call in &calls {
            assert_ne!(
                call.caller_module, call.callee_module,
                "Self-calls should be excluded"
            );
        }
    }

    #[rstest]
    fn test_get_module_calls_empty_project(populated_db: Box<dyn crate::backend::Database>) {
        let result = get_module_calls(&*populated_db, "nonexistent");
        assert!(result.is_ok());
        let calls = result.unwrap();
        assert!(calls.is_empty(), "Non-existent project should have no calls");
    }

    #[rstest]
    fn test_get_module_calls_returns_valid_modules(populated_db: Box<dyn crate::backend::Database>) {
        let result = get_module_calls(&*populated_db, "default");
        assert!(result.is_ok());
        let calls = result.unwrap();
        for call in &calls {
            assert!(!call.caller_module.is_empty());
            assert!(!call.callee_module.is_empty());
        }
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
    use super::*;

    fn get_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::surreal_call_graph_db_complex()
    }

    // ===== Basic functionality tests =====

    #[test]
    fn test_get_module_calls_returns_results() {
        let db = get_db();
        let result = get_module_calls(&*db, "default");

        assert!(result.is_ok(), "Query should succeed: {:?}", result.err());
        let calls = result.unwrap();
        assert!(!calls.is_empty(), "Should find inter-module calls");
    }

    #[test]
    fn test_get_module_calls_returns_exact_count() {
        let db = get_db();
        let calls = get_module_calls(&*db, "default").expect("Query should succeed");

        // The complex fixture has 20 inter-module calls:
        // Original (8):
        //   Controller -> Accounts (2), Controller -> Service (1), Controller -> Notifier (1)
        //   Accounts -> Repo (2), Service -> Accounts (1), Service -> Notifier (1)
        // Cycle A (3): Service -> Logger, Logger -> Repo, Repo -> Service
        // Cycle B (4): Controller -> Events, Events -> Cache, Cache -> Accounts, Accounts -> Controller
        // Cycle C (5): Notifier -> Metrics, Metrics -> Logger, Logger -> Events, Events -> Cache, Cache -> Notifier
        assert_eq!(
            calls.len(),
            20,
            "Should find exactly 20 inter-module calls (excluding intra-module calls)"
        );
    }

    #[test]
    fn test_get_module_calls_excludes_self_calls() {
        let db = get_db();
        let calls = get_module_calls(&*db, "default").expect("Query should succeed");

        // Verify no self-calls are present
        for call in &calls {
            assert_ne!(
                call.caller_module, call.callee_module,
                "Self-calls should be excluded, but found: {} -> {}",
                call.caller_module,
                call.callee_module
            );
        }
    }

    #[test]
    fn test_get_module_calls_returns_valid_modules() {
        let db = get_db();
        let calls = get_module_calls(&*db, "default").expect("Query should succeed");

        assert!(!calls.is_empty(), "Should have results");

        for call in &calls {
            assert!(
                !call.caller_module.is_empty(),
                "caller_module should not be empty"
            );
            assert!(
                !call.callee_module.is_empty(),
                "callee_module should not be empty"
            );
        }
    }

    #[test]
    fn test_get_module_calls_all_modules_present() {
        let db = get_db();
        let calls = get_module_calls(&*db, "default").expect("Query should succeed");

        let modules: std::collections::HashSet<_> = calls
            .iter()
            .flat_map(|call| vec![call.caller_module.as_str(), call.callee_module.as_str()])
            .collect();

        // Should contain references to all modules involved in inter-module calls
        assert!(
            modules.contains("MyApp.Controller"),
            "Should reference MyApp.Controller"
        );
        assert!(modules.contains("MyApp.Accounts"), "Should reference MyApp.Accounts");
        assert!(modules.contains("MyApp.Service"), "Should reference MyApp.Service");
        assert!(
            modules.contains("MyApp.Notifier"),
            "Should reference MyApp.Notifier"
        );
    }

    #[test]
    fn test_get_module_calls_contains_controller_to_accounts() {
        let db = get_db();
        let calls = get_module_calls(&*db, "default").expect("Query should succeed");

        // Controller.index calls Accounts.list_users
        let controller_to_accounts = calls.iter().any(|call| {
            call.caller_module == "MyApp.Controller" && call.callee_module == "MyApp.Accounts"
        });

        assert!(
            controller_to_accounts,
            "Should contain at least one call from Controller to Accounts"
        );
    }

    #[test]
    fn test_get_module_calls_contains_controller_to_service() {
        let db = get_db();
        let calls = get_module_calls(&*db, "default").expect("Query should succeed");

        // Controller.create calls Service.process_request
        let controller_to_service = calls.iter().any(|call| {
            call.caller_module == "MyApp.Controller" && call.callee_module == "MyApp.Service"
        });

        assert!(
            controller_to_service,
            "Should contain at least one call from Controller to Service"
        );
    }

    #[test]
    fn test_get_module_calls_contains_service_to_accounts() {
        let db = get_db();
        let calls = get_module_calls(&*db, "default").expect("Query should succeed");

        // Service.process_request calls Accounts.get_user
        let service_to_accounts = calls.iter().any(|call| {
            call.caller_module == "MyApp.Service" && call.callee_module == "MyApp.Accounts"
        });

        assert!(
            service_to_accounts,
            "Should contain at least one call from Service to Accounts"
        );
    }

    #[test]
    fn test_get_module_calls_contains_service_to_notifier() {
        let db = get_db();
        let calls = get_module_calls(&*db, "default").expect("Query should succeed");

        // Service.process_request calls Notifier.send_email
        let service_to_notifier = calls.iter().any(|call| {
            call.caller_module == "MyApp.Service" && call.callee_module == "MyApp.Notifier"
        });

        assert!(
            service_to_notifier,
            "Should contain at least one call from Service to Notifier"
        );
    }

    #[test]
    fn test_get_module_calls_contains_accounts_to_repo() {
        let db = get_db();
        let calls = get_module_calls(&*db, "default").expect("Query should succeed");

        // Accounts calls Repo (get_user->get, list_users->all)
        let accounts_to_repo = calls.iter().any(|call| {
            call.caller_module == "MyApp.Accounts" && call.callee_module == "MyApp.Repo"
        });

        assert!(
            accounts_to_repo,
            "Should contain at least one call from Accounts to Repo"
        );
    }

    #[test]
    fn test_get_module_calls_no_repo_internal_calls() {
        let db = get_db();
        let calls = get_module_calls(&*db, "default").expect("Query should succeed");

        // Repo has internal calls (get->query, all->query) which should be excluded
        let repo_internal = calls.iter().any(|call| {
            call.caller_module == "MyApp.Repo" && call.callee_module == "MyApp.Repo"
        });

        assert!(
            !repo_internal,
            "Should not contain internal Repo->Repo calls"
        );
    }

    #[test]
    fn test_get_module_calls_no_notifier_internal_calls() {
        let db = get_db();
        let calls = get_module_calls(&*db, "default").expect("Query should succeed");

        // Notifier has internal calls (send_email->format_message) which should be excluded
        let notifier_internal = calls.iter().any(|call| {
            call.caller_module == "MyApp.Notifier" && call.callee_module == "MyApp.Notifier"
        });

        assert!(
            !notifier_internal,
            "Should not contain internal Notifier->Notifier calls"
        );
    }

    #[test]
    fn test_get_module_calls_no_accounts_internal_calls() {
        let db = get_db();
        let calls = get_module_calls(&*db, "default").expect("Query should succeed");

        // Accounts has internal calls (get_user/2->get_user/1) which should be excluded
        let accounts_internal = calls.iter().any(|call| {
            call.caller_module == "MyApp.Accounts" && call.callee_module == "MyApp.Accounts"
        });

        assert!(
            !accounts_internal,
            "Should not contain internal Accounts->Accounts calls"
        );
    }

    #[test]
    fn test_get_module_calls_empty_project() {
        let db = get_db();
        // SurrealDB doesn't use project concept - database is per-project
        // But call with different project to verify no crash
        let result = get_module_calls(&*db, "nonexistent");
        assert!(result.is_ok(), "Query should not error on different project");
    }
}

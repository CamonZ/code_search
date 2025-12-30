use std::error::Error;

use clap::ValueEnum;
use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::query_builders::validate_regex_patterns;

use crate::db::{extract_i64, extract_string};

/// What type of hotspots to find
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum HotspotKind {
    /// Functions with most incoming calls (most called)
    #[default]
    Incoming,
    /// Functions with most outgoing calls (calls many things)
    Outgoing,
    /// Functions with highest total (incoming + outgoing)
    Total,
    /// Functions with highest ratio of incoming to outgoing calls (boundary functions)
    Ratio,
}

#[derive(Error, Debug)]
pub enum HotspotsError {
    #[error("Hotspots query failed: {message}")]
    QueryFailed { message: String },
}

/// A function hotspot with call counts
#[derive(Debug, Clone, Serialize)]
pub struct Hotspot {
    pub module: String,
    pub function: String,
    pub incoming: i64,
    pub outgoing: i64,
    pub total: i64,
    pub ratio: f64,
}

/// Get lines of code per module (sum of function line counts)
pub fn get_module_loc(
    db: &dyn Database,
    _project: &str,
    module_pattern: Option<&str>,
    use_regex: bool,
) -> Result<std::collections::HashMap<String, i64>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    let module_clause = if module_pattern.is_some() {
        if use_regex {
            "WHERE string::matches(module_name, $module_pattern)"
        } else {
            "WHERE module_name = $module_pattern"
        }
    } else {
        ""
    };

    // LOC per module is sum of (end_line - start_line + 1) for all clauses
    let query = format!(
        r#"
        SELECT module_name, math::sum(end_line - start_line + 1) as loc
        FROM clauses
        {module_clause}
        GROUP BY module_name
        ORDER BY loc DESC
        "#
    );

    let mut params = QueryParams::new();
    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = db.execute_query(&query, params).map_err(|e| HotspotsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut loc_map = std::collections::HashMap::new();
    for row in result.rows() {
        // SurrealDB returns columns alphabetically: loc, module_name
        if row.len() >= 2 {
            let loc = extract_i64(row.get(0).unwrap(), 0);
            let Some(module) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            loc_map.insert(module, loc);
        }
    }

    Ok(loc_map)
}

/// Get function count per module
pub fn get_function_counts(
    db: &dyn Database,
    _project: &str,
    module_pattern: Option<&str>,
    use_regex: bool,
) -> Result<std::collections::HashMap<String, i64>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    let module_clause = if module_pattern.is_some() {
        if use_regex {
            "WHERE string::matches(module_name, $module_pattern)"
        } else {
            "WHERE module_name = $module_pattern"
        }
    } else {
        ""
    };

    // Query clauses table, count unique functions per module
    // Group by module_name, function_name, arity to count distinct function signatures
    let query = format!(
        r#"
        SELECT module_name, count() as function_count
        FROM (
            SELECT module_name, function_name, arity
            FROM clauses
            {module_clause}
            GROUP BY module_name, function_name, arity
        )
        GROUP BY module_name
        ORDER BY function_count DESC
        "#
    );

    let mut params = QueryParams::new();
    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = db.execute_query(&query, params).map_err(|e| HotspotsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut counts = std::collections::HashMap::new();
    for row in result.rows() {
        // SurrealDB returns columns alphabetically: function_count, module_name
        if row.len() >= 2 {
            let function_count = extract_i64(row.get(0).unwrap(), 0);
            let Some(module) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            counts.insert(module, function_count);
        }
    }

    Ok(counts)
}

/// Get module-level connectivity (aggregated incoming/outgoing calls)
///
/// Returns a HashMap of module name -> (incoming, outgoing) call counts.
/// This aggregates function-level hotspots to module level at the database layer,
/// avoiding the need to fetch all function hotspots.
pub fn get_module_connectivity(
    db: &dyn Database,
    _project: &str,
    module_pattern: Option<&str>,
    use_regex: bool,
) -> Result<std::collections::HashMap<String, (i64, i64)>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // For module connectivity, we query the calls table and count distinct
    // module pairs in Rust (SurrealDB GROUP BY returns only 1 row unexpectedly).

    // Query all calls - we'll filter and count distinct modules in Rust
    let query = if let Some(_) = module_pattern {
        if use_regex {
            r#"SELECT in.module_name as source, out.module_name as target FROM calls WHERE in.module_name = <regex>$module_pattern OR out.module_name = <regex>$module_pattern"#.to_string()
        } else {
            r#"SELECT in.module_name as source, out.module_name as target FROM calls WHERE in.module_name = $module_pattern OR out.module_name = $module_pattern"#.to_string()
        }
    } else {
        r#"SELECT in.module_name as source, out.module_name as target FROM calls"#.to_string()
    };

    // Execute query to get all call pairs
    let mut params = QueryParams::new();
    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }
    let result = db.execute_query(&query, params).map_err(|e| HotspotsError::QueryFailed {
        message: e.to_string(),
    })?;

    // Count distinct modules for incoming (sources per target) and outgoing (targets per source)
    let mut outgoing_sets: std::collections::HashMap<String, std::collections::HashSet<String>> =
        std::collections::HashMap::new();
    let mut incoming_sets: std::collections::HashMap<String, std::collections::HashSet<String>> =
        std::collections::HashMap::new();

    // Process results - columns are alphabetical: source, target
    for row in result.rows() {
        if row.len() >= 2 {
            let Some(source) = extract_string(row.get(0).unwrap()) else {
                continue;
            };
            let Some(target) = extract_string(row.get(1).unwrap()) else {
                continue;
            };
            // For outgoing: source -> set of targets
            outgoing_sets.entry(source.clone()).or_default().insert(target.clone());
            // For incoming: target -> set of sources
            incoming_sets.entry(target).or_default().insert(source);
        }
    }

    // Build connectivity map with (incoming, outgoing) counts
    let mut connectivity: std::collections::HashMap<String, (i64, i64)> =
        std::collections::HashMap::new();

    for (module, targets) in &outgoing_sets {
        connectivity.entry(module.clone()).or_insert((0, 0)).1 = targets.len() as i64;
    }

    for (module, sources) in &incoming_sets {
        connectivity.entry(module.clone()).or_insert((0, 0)).0 = sources.len() as i64;
    }

    // If a module pattern is specified, filter to only include matching modules
    if let Some(pattern) = module_pattern {
        if use_regex {
            let re = regex::Regex::new(pattern)
                .map_err(|e| HotspotsError::QueryFailed { message: e.to_string() })?;
            connectivity.retain(|module, _| re.is_match(module));
        } else {
            connectivity.retain(|module, _| module == pattern);
        }
    }

    Ok(connectivity)
}

pub fn find_hotspots(
    db: &dyn Database,
    kind: HotspotKind,
    module_pattern: Option<&str>,
    _project: &str,
    use_regex: bool,
    limit: u32,
    _exclude_generated: bool,
    require_outgoing: bool,
) -> Result<Vec<Hotspot>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // SurrealDB has a bug where GROUP BY with field traversal (out.module_name) doesn't work
    // (see https://github.com/surrealdb/surrealdb/issues/2695)
    // Workaround: GROUP BY the whole record (out/in), then extract fields from the Thing ID
    // The Thing ID is an array: [module_name, function_name, arity]

    // Helper to extract (module, function) from a Thing ID's array
    fn extract_function_key(val: &dyn crate::Value) -> Option<(String, String)> {
        val.as_thing_id()
            .and_then(|thing| thing.as_array())
            .and_then(|arr| {
                let module = arr.first().and_then(|v| v.as_str())?;
                let function = arr.get(1).and_then(|v| v.as_str())?;
                Some((module.to_string(), function.to_string()))
            })
    }

    // Query incoming call counts: GROUP BY callee (out)
    // In graph edges: out = callee (target), so grouping by out gives us incoming counts
    let incoming_query = "SELECT out, count() as cnt FROM calls GROUP BY out";
    let incoming_result = db.execute_query(incoming_query, QueryParams::new())
        .map_err(|e| HotspotsError::QueryFailed {
            message: format!("Failed to get incoming calls: {}", e),
        })?;

    // Query outgoing call counts: GROUP BY caller (in)
    // In graph edges: in = caller (source), so grouping by in gives us outgoing counts
    let outgoing_query = "SELECT in, count() as cnt FROM calls GROUP BY in";
    let outgoing_result = db.execute_query(outgoing_query, QueryParams::new())
        .map_err(|e| HotspotsError::QueryFailed {
            message: format!("Failed to get outgoing calls: {}", e),
        })?;

    // Build count hashmaps from query results
    // Key: (module, function), Value: count
    let mut incoming_counts: std::collections::HashMap<(String, String), i64> =
        std::collections::HashMap::new();
    let mut outgoing_counts: std::collections::HashMap<(String, String), i64> =
        std::collections::HashMap::new();

    // Process incoming results - headers are alphabetically sorted: ["cnt", "out"]
    for row in incoming_result.rows() {
        if row.len() >= 2 {
            let cnt = row.get(0).and_then(|v| v.as_i64()).unwrap_or(0);
            if let Some(key) = row.get(1).and_then(|v| extract_function_key(v)) {
                incoming_counts.insert(key, cnt);
            }
        }
    }

    // Process outgoing results - headers are alphabetically sorted: ["cnt", "in"]
    for row in outgoing_result.rows() {
        if row.len() >= 2 {
            let cnt = row.get(0).and_then(|v| v.as_i64()).unwrap_or(0);
            if let Some(key) = row.get(1).and_then(|v| extract_function_key(v)) {
                outgoing_counts.insert(key, cnt);
            }
        }
    }

    // Helper to find column index by header name
    fn find_col(headers: &[String], name: &str) -> Option<usize> {
        headers.iter().position(|h| h == name)
    }

    // Get all functions to combine incoming and outgoing
    let functions_query = "SELECT module_name as module, name as function FROM functions";
    let functions_result = db.execute_query(functions_query, QueryParams::new())
        .map_err(|e| HotspotsError::QueryFailed {
            message: format!("Failed to get functions: {}", e),
        })?;

    let mut hotspots = Vec::new();
    let func_headers = functions_result.headers();
    let func_mod_idx = find_col(func_headers, "module");
    let func_fn_idx = find_col(func_headers, "function");

    if let (Some(mod_idx), Some(fn_idx)) = (func_mod_idx, func_fn_idx) {
        for row in functions_result.rows() {
            if row.len() >= 2 {
                if let (Some(module), Some(function)) = (
                    row.get(mod_idx).and_then(|v| extract_string(v)),
                    row.get(fn_idx).and_then(|v| extract_string(v)),
                ) {
                    let key = (module.clone(), function.clone());
                    let incoming = *incoming_counts.get(&key).unwrap_or(&0);
                    let outgoing = *outgoing_counts.get(&key).unwrap_or(&0);
                    let total = incoming + outgoing;
                    let ratio = if outgoing == 0 {
                        if incoming > 0 { 9999.0 } else { 0.0 }
                    } else {
                        incoming as f64 / outgoing as f64
                    };

                    // Apply filters
                    if require_outgoing && outgoing == 0 {
                        continue;
                    }

                    hotspots.push(Hotspot {
                        module,
                        function,
                        incoming,
                        outgoing,
                        total,
                        ratio,
                    });
                }
            }
        }
    }

    // Filter by module pattern if specified
    if let Some(pattern) = module_pattern {
        if use_regex {
            let re = regex::Regex::new(pattern)
                .map_err(|e| HotspotsError::QueryFailed { message: e.to_string() })?;
            hotspots.retain(|h| re.is_match(&h.module));
        } else {
            hotspots.retain(|h| h.module == pattern);
        }
    }

    // Sort by the specified kind
    match kind {
        HotspotKind::Incoming => hotspots.sort_by(|a, b| b.incoming.cmp(&a.incoming)),
        HotspotKind::Outgoing => hotspots.sort_by(|a, b| b.outgoing.cmp(&a.outgoing)),
        HotspotKind::Total => hotspots.sort_by(|a, b| b.total.cmp(&a.total)),
        HotspotKind::Ratio => hotspots.sort_by(|a, b| b.ratio.partial_cmp(&a.ratio).unwrap_or(std::cmp::Ordering::Equal)),
    }

    // Apply limit
    hotspots.truncate(limit as usize);

    Ok(hotspots)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The complex fixture contains:
    // - 5 modules: Controller (3 funcs), Accounts (4), Service (2), Repo (4), Notifier (2)
    // - 15 functions total
    // - 12 call edges forming a realistic call graph
    fn get_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::surreal_call_graph_db_complex()
    }

    // ===== get_function_counts tests =====

    #[test]
    fn test_get_function_counts_exact_module_count() {
        let db = get_db();
        let counts = get_function_counts(&*db, "default", None, false)
            .expect("Query should succeed");

        // 9 modules: Controller, Accounts, Service, Repo, Notifier, Logger, Events, Cache, Metrics
        assert_eq!(counts.len(), 9, "Should have exactly 9 modules");
    }

    #[test]
    fn test_get_function_counts_exact_values_per_module() {
        let db = get_db();
        let counts = get_function_counts(&*db, "default", None, false)
            .expect("Query should succeed");

        // Verify exact function counts per module from fixture
        assert_eq!(
            counts.get("MyApp.Controller"),
            Some(&6),
            "Controller should have 6 functions (index, show, create, handle_event, format_display, __generated__)"
        );
        assert_eq!(
            counts.get("MyApp.Accounts"),
            Some(&8),
            "Accounts should have 8 functions (get_user/1, get_user/2, list_users, validate_email, __struct__, notify_change, format_name, __generated__)"
        );
        assert_eq!(
            counts.get("MyApp.Service"),
            Some(&4),
            "Service should have 4 functions (process_request, transform_data, get_context, validate)"
        );
        assert_eq!(
            counts.get("MyApp.Repo"),
            Some(&5),
            "Repo should have 5 functions (get, all, insert, query, validate)"
        );
        assert_eq!(
            counts.get("MyApp.Notifier"),
            Some(&3),
            "Notifier should have 3 functions (send_email, format_message, on_cache_update)"
        );
        assert_eq!(
            counts.get("MyApp.Logger"),
            Some(&3),
            "Logger should have 3 functions (log_query, log_metric, debug)"
        );
        assert_eq!(
            counts.get("MyApp.Events"),
            Some(&3),
            "Events should have 3 functions (publish, emit, subscribe)"
        );
        assert_eq!(
            counts.get("MyApp.Cache"),
            Some(&3),
            "Cache should have 3 functions (invalidate, store, fetch)"
        );
        assert_eq!(
            counts.get("MyApp.Metrics"),
            Some(&2),
            "Metrics should have 2 functions (record, increment)"
        );
    }

    #[test]
    fn test_get_function_counts_total_is_thirtyone() {
        let db = get_db();
        let counts = get_function_counts(&*db, "default", None, false)
            .expect("Query should succeed");

        let total: i64 = counts.values().sum();
        assert_eq!(total, 37, "Total function count should be 37");
    }

    #[test]
    fn test_get_function_counts_controller_pattern() {
        let db = get_db();
        let counts = get_function_counts(&*db, "default", Some("MyApp.Controller"), false)
            .expect("Query should succeed");

        assert_eq!(counts.len(), 1, "Should match exactly 1 module");
        assert_eq!(
            counts.get("MyApp.Controller"),
            Some(&6),
            "Controller should have 6 functions"
        );
    }

    #[test]
    fn test_get_function_counts_regex_pattern() {
        let db = get_db();
        let counts = get_function_counts(&*db, "default", Some("^MyApp\\.Accounts$"), true)
            .expect("Query should succeed");

        assert_eq!(counts.len(), 1, "Should match exactly 1 module");
        assert_eq!(
            counts.get("MyApp.Accounts"),
            Some(&8),
            "Accounts should have 8 functions"
        );
    }

    #[test]
    fn test_get_function_counts_nonexistent_module() {
        let db = get_db();
        let counts = get_function_counts(&*db, "default", Some("NonExistent"), false)
            .expect("Query should succeed");

        assert!(counts.is_empty(), "Should return empty for non-existent module");
    }

    #[test]
    fn test_get_function_counts_invalid_regex() {
        let db = get_db();
        let result = get_function_counts(&*db, "default", Some("[invalid"), true);

        assert!(result.is_err(), "Should reject invalid regex pattern");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Invalid regex"),
            "Error should mention invalid regex: {}",
            err
        );
    }

    // ===== get_module_loc tests =====
    // LOC is calculated as sum of (end_line - start_line + 1) per clause.
    // In the test fixture, start_line == end_line, so each clause has LOC=1.
    // Total module LOC = number of clauses in that module.

    #[test]
    fn test_get_module_loc_returns_module_count() {
        let db = get_db();
        let loc_map = get_module_loc(&*db, "default", None, false)
            .expect("Query should succeed");

        // 9 modules should have LOC data
        assert_eq!(loc_map.len(), 9, "Should have LOC for all 9 modules");
    }

    #[test]
    fn test_get_module_loc_exact_values() {
        let db = get_db();
        let loc_map = get_module_loc(&*db, "default", None, false)
            .expect("Query should succeed");

        // Each clause has LOC=1, so module LOC = number of clauses
        // Controller: 10 clauses (index x2, show x2, create x3, handle_event, format_display, __generated__)
        assert_eq!(loc_map.get("MyApp.Controller"), Some(&10), "Controller LOC");
        // Accounts: 8 clauses (get_user/1 x2, get_user/2, list_users, notify_change, validate_email, __struct__, format_name) + 1 __generated__ = 9
        assert_eq!(loc_map.get("MyApp.Accounts"), Some(&9), "Accounts LOC");
        // Service: 5 clauses (process_request x3, transform_data, get_context) + 1 validate = 6
        assert_eq!(loc_map.get("MyApp.Service"), Some(&6), "Service LOC");
        // Repo: 4 clauses + 1 validate = 5
        assert_eq!(loc_map.get("MyApp.Repo"), Some(&5), "Repo LOC");
        // Notifier: 3 clauses
        assert_eq!(loc_map.get("MyApp.Notifier"), Some(&3), "Notifier LOC");
    }

    #[test]
    fn test_get_module_loc_with_pattern() {
        let db = get_db();
        let loc_map = get_module_loc(&*db, "default", Some("MyApp.Accounts"), false)
            .expect("Query should succeed");

        assert_eq!(loc_map.len(), 1, "Should match exactly 1 module");
        assert_eq!(loc_map.get("MyApp.Accounts"), Some(&9), "Accounts should have 9 LOC");
    }

    #[test]
    fn test_get_module_loc_invalid_regex() {
        let db = get_db();
        let result = get_module_loc(&*db, "default", Some("[invalid"), true);

        assert!(result.is_err(), "Should reject invalid regex pattern");
    }

    // ===== get_module_connectivity tests =====
    // Tests connectivity based on the 24 call edges in the fixture (12 original + 12 for cycles)

    #[test]
    fn test_get_module_connectivity_exact_module_count() {
        let db = get_db();
        let connectivity = get_module_connectivity(&*db, "default", None, false)
            .expect("Query should succeed");

        // 9 modules: Controller, Accounts, Service, Repo, Notifier, Logger, Events, Cache, Metrics
        assert_eq!(connectivity.len(), 9, "Should have exactly 9 modules");
    }

    #[test]
    fn test_get_module_connectivity_controller_values() {
        let db = get_db();
        let connectivity = get_module_connectivity(&*db, "default", None, false)
            .expect("Query should succeed");

        // Controller: 1 incoming unique module (Accounts)
        // 4 outgoing unique modules: Accounts, Service, Notifier, Events
        let (incoming, outgoing) = connectivity
            .get("MyApp.Controller")
            .expect("Controller should be present");
        assert_eq!(
            *incoming, 1,
            "Controller should have 1 unique incoming module (Accounts)"
        );
        assert_eq!(
            *outgoing, 4,
            "Controller should have 4 unique outgoing modules (Accounts, Service, Notifier, Events)"
        );
    }

    #[test]
    fn test_get_module_connectivity_accounts_values() {
        let db = get_db();
        let connectivity = get_module_connectivity(&*db, "default", None, false)
            .expect("Query should succeed");

        // Accounts: 4 unique incoming modules (Controller, Service, Cache, self)
        // 3 unique outgoing modules: Repo, Controller, self
        let (incoming, outgoing) = connectivity
            .get("MyApp.Accounts")
            .expect("Accounts should be present");
        assert_eq!(
            *incoming, 4,
            "Accounts should have 4 unique incoming modules (Controller, Service, Cache, self)"
        );
        assert_eq!(
            *outgoing, 3,
            "Accounts should have 3 unique outgoing modules (Repo, Controller, self)"
        );
    }

    #[test]
    fn test_get_module_connectivity_service_values() {
        let db = get_db();
        let connectivity = get_module_connectivity(&*db, "default", None, false)
            .expect("Query should succeed");

        // Service: called by Controller, Repo (insert->get_context)
        // Calls: Accounts, Notifier, Logger
        let (incoming, outgoing) = connectivity
            .get("MyApp.Service")
            .expect("Service should be present");
        assert_eq!(*incoming, 2, "Service should have 2 incoming (Controller, Repo)");
        assert_eq!(
            *outgoing, 3,
            "Service should have 3 outgoing (Accounts, Notifier, Logger)"
        );
    }

    #[test]
    fn test_get_module_connectivity_repo_values() {
        let db = get_db();
        let connectivity = get_module_connectivity(&*db, "default", None, false)
            .expect("Query should succeed");

        // Repo: 3 unique incoming modules (Accounts, Logger, self)
        // 2 unique outgoing modules: Service, self
        let (incoming, outgoing) = connectivity
            .get("MyApp.Repo")
            .expect("Repo should be present");
        assert_eq!(
            *incoming, 3,
            "Repo should have 3 unique incoming modules (Accounts, Logger, self)"
        );
        assert_eq!(*outgoing, 2, "Repo should have 2 unique outgoing modules (Service, self)");
    }

    #[test]
    fn test_get_module_connectivity_notifier_values() {
        let db = get_db();
        let connectivity = get_module_connectivity(&*db, "default", None, false)
            .expect("Query should succeed");

        // Notifier: called by Service, Controller, Notifier (self), Cache (store->on_cache_update)
        // Calls: Notifier (self), Metrics
        let (incoming, outgoing) = connectivity
            .get("MyApp.Notifier")
            .expect("Notifier should be present");
        assert_eq!(
            *incoming, 4,
            "Notifier should have 4 incoming (Service, Controller, Notifier-self, Cache)"
        );
        assert_eq!(
            *outgoing, 2,
            "Notifier should have 2 outgoing (Notifier-self, Metrics)"
        );
    }

    #[test]
    fn test_get_module_connectivity_with_pattern() {
        let db = get_db();
        let connectivity =
            get_module_connectivity(&*db, "default", Some("MyApp.Controller"), false)
                .expect("Query should succeed");

        assert_eq!(connectivity.len(), 1, "Should match exactly 1 module");
        let (incoming, outgoing) = connectivity
            .get("MyApp.Controller")
            .expect("Controller should be present");
        assert_eq!(*incoming, 1, "Controller has 1 unique incoming module (Accounts)");
        assert_eq!(*outgoing, 4, "Controller has 4 unique outgoing modules");
    }

    #[test]
    fn test_get_module_connectivity_nonexistent_module() {
        let db = get_db();
        let connectivity =
            get_module_connectivity(&*db, "default", Some("NonExistent"), false)
                .expect("Query should succeed");

        assert!(
            connectivity.is_empty(),
            "Should return empty for non-existent module"
        );
    }

    #[test]
    fn test_get_module_connectivity_invalid_regex() {
        let db = get_db();
        let result = get_module_connectivity(&*db, "default", Some("[invalid"), true);

        assert!(result.is_err(), "Should reject invalid regex pattern");
    }

    // ===== Cross-function consistency tests =====

    #[test]
    fn test_function_counts_matches_connectivity_modules() {
        let db = get_db();
        let counts = get_function_counts(&*db, "default", None, false)
            .expect("Function counts query should succeed");
        let connectivity = get_module_connectivity(&*db, "default", None, false)
            .expect("Connectivity query should succeed");

        // Both queries should return the same set of modules
        assert_eq!(
            counts.len(),
            connectivity.len(),
            "Function counts and connectivity should have same module count"
        );

        for module in counts.keys() {
            assert!(
                connectivity.contains_key(module),
                "Module {} from function counts should exist in connectivity",
                module
            );
        }
    }

    #[test]
    fn test_all_modules_present_in_both_queries() {
        let db = get_db();
        let counts = get_function_counts(&*db, "default", None, false)
            .expect("Query should succeed");
        let connectivity = get_module_connectivity(&*db, "default", None, false)
            .expect("Query should succeed");

        let expected_modules = [
            "MyApp.Controller",
            "MyApp.Accounts",
            "MyApp.Service",
            "MyApp.Repo",
            "MyApp.Notifier",
            "MyApp.Logger",
            "MyApp.Events",
            "MyApp.Cache",
            "MyApp.Metrics",
        ];

        for module in expected_modules {
            assert!(
                counts.contains_key(module),
                "Module {} should be in function counts",
                module
            );
            assert!(
                connectivity.contains_key(module),
                "Module {} should be in connectivity",
                module
            );
        }
    }

    // ===== find_hotspots tests =====

    #[test]
    fn test_find_hotspots_returns_results() {
        let db = get_db();
        let hotspots = find_hotspots(
            &*db,
            HotspotKind::Incoming,
            None,
            "default",
            false,
            100,
            false,
            false,
        ).expect("Query should succeed");

        assert!(!hotspots.is_empty(), "Should return hotspots from fixture");
    }

    #[test]
    fn test_find_hotspots_verifies_fixture_values() {
        let db = get_db();
        let hotspots = find_hotspots(
            &*db,
            HotspotKind::Total,
            None,
            "default",
            false,
            100,
            false,
            false,
        ).expect("Query should succeed");

        // Verify we have functions with non-zero counts (the old buggy code returned all zeros)
        let non_zero_hotspots: Vec<_> = hotspots.iter().filter(|h| h.total > 0).collect();
        assert!(
            non_zero_hotspots.len() >= 5,
            "Should have at least 5 functions with non-zero call counts, got {}",
            non_zero_hotspots.len()
        );

        // Verify specific function from fixture: MyApp.Accounts.get_user should have calls
        let get_user = hotspots.iter().find(|h|
            h.module == "MyApp.Accounts" && h.function == "get_user"
        );
        assert!(get_user.is_some(), "Should find MyApp.Accounts.get_user");
        let get_user = get_user.unwrap();
        assert!(get_user.incoming > 0, "get_user should have incoming calls, got {}", get_user.incoming);
        assert!(get_user.outgoing > 0, "get_user should have outgoing calls, got {}", get_user.outgoing);

        // Verify Repo.query is a leaf node (called but doesn't call others)
        let repo_query = hotspots.iter().find(|h|
            h.module == "MyApp.Repo" && h.function == "query"
        );
        assert!(repo_query.is_some(), "Should find MyApp.Repo.query");
        let repo_query = repo_query.unwrap();
        assert!(repo_query.incoming > 0, "Repo.query should have incoming calls");
        assert_eq!(repo_query.outgoing, 0, "Repo.query should be a leaf node with no outgoing calls");
    }

    #[test]
    fn test_find_hotspots_has_valid_structure() {
        let db = get_db();
        let hotspots = find_hotspots(
            &*db,
            HotspotKind::Incoming,
            None,
            "default",
            false,
            100,
            false,
            false,
        ).expect("Query should succeed");

        // All hotspots should have valid structure
        for hotspot in &hotspots {
            assert!(!hotspot.module.is_empty(), "Module should not be empty");
            assert!(!hotspot.function.is_empty(), "Function should not be empty");
            assert!(hotspot.incoming >= 0, "Incoming should be non-negative");
            assert!(hotspot.outgoing >= 0, "Outgoing should be non-negative");
            assert!(hotspot.total >= 0, "Total should be non-negative");
            assert_eq!(
                hotspot.total,
                hotspot.incoming + hotspot.outgoing,
                "Total should equal incoming + outgoing"
            );
        }
    }

    #[test]
    fn test_find_hotspots_incoming_sort_order() {
        let db = get_db();
        let hotspots = find_hotspots(
            &*db,
            HotspotKind::Incoming,
            None,
            "default",
            false,
            100,
            false,
            false,
        ).expect("Query should succeed");

        // Should be sorted by incoming in descending order
        for i in 0..hotspots.len().saturating_sub(1) {
            assert!(
                hotspots[i].incoming >= hotspots[i + 1].incoming,
                "Hotspots should be sorted by incoming (descending)"
            );
        }
    }

    #[test]
    fn test_find_hotspots_outgoing_sort_order() {
        let db = get_db();
        let hotspots = find_hotspots(
            &*db,
            HotspotKind::Outgoing,
            None,
            "default",
            false,
            100,
            false,
            false,
        ).expect("Query should succeed");

        // Should be sorted by outgoing in descending order
        for i in 0..hotspots.len().saturating_sub(1) {
            assert!(
                hotspots[i].outgoing >= hotspots[i + 1].outgoing,
                "Hotspots should be sorted by outgoing (descending)"
            );
        }
    }

    #[test]
    fn test_find_hotspots_total_sort_order() {
        let db = get_db();
        let hotspots = find_hotspots(
            &*db,
            HotspotKind::Total,
            None,
            "default",
            false,
            100,
            false,
            false,
        ).expect("Query should succeed");

        // Should be sorted by total in descending order
        for i in 0..hotspots.len().saturating_sub(1) {
            assert!(
                hotspots[i].total >= hotspots[i + 1].total,
                "Hotspots should be sorted by total (descending)"
            );
        }
    }

    #[test]
    fn test_find_hotspots_ratio_sort_order() {
        let db = get_db();
        let hotspots = find_hotspots(
            &*db,
            HotspotKind::Ratio,
            None,
            "default",
            false,
            100,
            false,
            false,
        ).expect("Query should succeed");

        // Should be sorted by ratio in descending order
        for i in 0..hotspots.len().saturating_sub(1) {
            let ratio_cmp = hotspots[i].ratio.partial_cmp(&hotspots[i + 1].ratio)
                .unwrap_or(std::cmp::Ordering::Equal);
            assert!(
                ratio_cmp == std::cmp::Ordering::Greater || ratio_cmp == std::cmp::Ordering::Equal,
                "Hotspots should be sorted by ratio (descending)"
            );
        }
    }

    #[test]
    fn test_find_hotspots_respects_limit() {
        let db = get_db();
        let limit_5 = find_hotspots(
            &*db,
            HotspotKind::Incoming,
            None,
            "default",
            false,
            5,
            false,
            false,
        ).expect("Query should succeed");

        let limit_100 = find_hotspots(
            &*db,
            HotspotKind::Incoming,
            None,
            "default",
            false,
            100,
            false,
            false,
        ).expect("Query should succeed");

        assert!(limit_5.len() <= 5, "Should respect limit of 5");
        assert!(limit_5.len() <= limit_100.len(), "Smaller limit should return <= results");
    }

    #[test]
    fn test_find_hotspots_with_module_pattern() {
        let db = get_db();
        let hotspots = find_hotspots(
            &*db,
            HotspotKind::Incoming,
            Some("MyApp.Controller"),
            "default",
            false,
            100,
            false,
            false,
        ).expect("Query should succeed");

        // All results should match the module pattern
        for hotspot in &hotspots {
            assert_eq!(
                hotspot.module,
                "MyApp.Controller",
                "All hotspots should be from MyApp.Controller"
            );
        }
    }

    #[test]
    fn test_find_hotspots_with_regex_pattern() {
        let db = get_db();
        let hotspots = find_hotspots(
            &*db,
            HotspotKind::Incoming,
            Some("^MyApp\\.Accounts$"),
            "default",
            true, // use_regex = true
            100,
            false,
            false,
        ).expect("Query should succeed");

        // All results should match the regex pattern
        for hotspot in &hotspots {
            assert_eq!(
                hotspot.module,
                "MyApp.Accounts",
                "All hotspots should match regex pattern"
            );
        }
    }

    #[test]
    fn test_find_hotspots_with_invalid_regex() {
        let db = get_db();
        let result = find_hotspots(
            &*db,
            HotspotKind::Incoming,
            Some("[invalid"),
            "default",
            true, // use_regex = true
            100,
            false,
            false,
        );

        assert!(result.is_err(), "Should reject invalid regex pattern");
    }

    #[test]
    fn test_find_hotspots_require_outgoing_excludes_leaf_nodes() {
        let db = get_db();
        let with_leaves = find_hotspots(
            &*db,
            HotspotKind::Incoming,
            None,
            "default",
            false,
            100,
            false,
            false, // require_outgoing = false
        ).expect("Query should succeed");

        let no_leaves = find_hotspots(
            &*db,
            HotspotKind::Incoming,
            None,
            "default",
            false,
            100,
            false,
            true, // require_outgoing = true
        ).expect("Query should succeed");

        // Excluding leaf nodes should return same or fewer results
        assert!(no_leaves.len() <= with_leaves.len(),
            "Excluding leaf nodes should return <= results"
        );

        // All results in no_leaves should have outgoing > 0
        for hotspot in &no_leaves {
            assert!(
                hotspot.outgoing > 0,
                "All hotspots should have outgoing > 0 when require_outgoing=true"
            );
        }
    }

    #[test]
    fn test_find_hotspots_nonexistent_module_pattern_returns_empty() {
        let db = get_db();
        let hotspots = find_hotspots(
            &*db,
            HotspotKind::Incoming,
            Some("NonExistentModule"),
            "default",
            false,
            100,
            false,
            false,
        ).expect("Query should succeed");

        assert!(hotspots.is_empty(), "Should return empty for non-existent module");
    }

    #[test]
    fn test_find_hotspots_ratio_calculation() {
        let db = get_db();
        let hotspots = find_hotspots(
            &*db,
            HotspotKind::Ratio,
            None,
            "default",
            false,
            100,
            false,
            false,
        ).expect("Query should succeed");

        // Verify ratio calculation
        for hotspot in &hotspots {
            let expected_ratio = if hotspot.outgoing == 0 {
                if hotspot.incoming > 0 { 9999.0 } else { 0.0 }
            } else {
                hotspot.incoming as f64 / hotspot.outgoing as f64
            };

            assert!(
                (hotspot.ratio - expected_ratio).abs() < 0.0001,
                "Ratio should be incoming/outgoing. Got {}, expected {}",
                hotspot.ratio,
                expected_ratio
            );
        }
    }
}

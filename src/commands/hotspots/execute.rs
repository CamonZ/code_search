use std::error::Error;
use std::path::Path;

use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use super::{HotspotsCmd, HotspotKind};
use crate::commands::Execute;
use crate::db::{extract_i64, extract_string, open_db, run_query, Params};

#[derive(Error, Debug)]
enum HotspotsError {
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
}

/// Result of the hotspots command execution
#[derive(Debug, Default, Serialize)]
pub struct HotspotsResult {
    pub project: String,
    pub kind: String,
    pub module_filter: Option<String>,
    pub hotspots: Vec<Hotspot>,
}

impl Execute for HotspotsCmd {
    type Output = HotspotsResult;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        let mut result = HotspotsResult {
            project: self.project.clone(),
            kind: match self.kind {
                HotspotKind::Incoming => "incoming".to_string(),
                HotspotKind::Outgoing => "outgoing".to_string(),
                HotspotKind::Total => "total".to_string(),
            },
            module_filter: self.module.clone(),
            ..Default::default()
        };

        result.hotspots = find_hotspots(
            &db,
            self.kind,
            self.module.as_deref(),
            &self.project,
            self.regex,
            self.limit,
        )?;

        Ok(result)
    }
}

fn find_hotspots(
    db: &cozo::DbInstance,
    kind: HotspotKind,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Hotspot>, Box<dyn Error>> {
    // Build optional module filter
    let module_filter = match module_pattern {
        Some(_) if use_regex => ", regex_matches(module, $module_pattern)".to_string(),
        Some(_) => ", str_includes(module, $module_pattern)".to_string(),
        None => String::new(),
    };

    let order_by = match kind {
        HotspotKind::Incoming => "incoming",
        HotspotKind::Outgoing => "outgoing",
        HotspotKind::Total => "total",
    };

    // Query to find hotspots by counting incoming and outgoing calls
    // We need to combine:
    // 1. Functions as callers (outgoing)
    // 2. Functions as callees (incoming)
    let script = format!(
        r#"
        # Count outgoing calls per function (as caller)
        outgoing_counts[module, function, count(callee_function)] :=
            *calls{{project, caller_module, caller_function, callee_function}},
            project == $project,
            module = caller_module,
            function = caller_function

        # Count incoming calls per function (as callee)
        incoming_counts[module, function, count(caller_function)] :=
            *calls{{project, caller_function, callee_module, callee_function}},
            project == $project,
            module = callee_module,
            function = callee_function

        # Get all unique module+function combinations
        all_functions[module, function] := outgoing_counts[module, function, _]
        all_functions[module, function] := incoming_counts[module, function, _]

        # Combine counts with defaults of 0
        ?[module, function, incoming, outgoing, total] :=
            all_functions[module, function],
            incoming_counts[module, function, inc] or inc = 0,
            outgoing_counts[module, function, out] or out = 0,
            incoming = inc,
            outgoing = out,
            total = inc + out
            {module_filter}

        :order -{order_by}, module, function
        :limit {limit}
        "#,
    );

    let mut params = Params::new();
    params.insert("project".to_string(), DataValue::Str(project.into()));
    if let Some(pattern) = module_pattern {
        params.insert("module_pattern".to_string(), DataValue::Str(pattern.into()));
    }

    let rows = run_query(&db, &script, params).map_err(|e| HotspotsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 5 {
            let Some(module) = extract_string(&row[0]) else { continue };
            let Some(function) = extract_string(&row[1]) else { continue };
            let incoming = extract_i64(&row[2], 0);
            let outgoing = extract_i64(&row[3], 0);
            let total = extract_i64(&row[4], 0);

            results.push(Hotspot {
                module,
                function,
                incoming,
                outgoing,
                total,
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    const TEST_JSON: &str = r#"{
        "structs": {},
        "function_locations": {},
        "calls": [
            {"caller": {"module": "MyApp.Web", "function": "index", "file": "lib/web.ex", "line": 10, "column": 5}, "type": "remote", "callee": {"arity": 1, "function": "get_user", "module": "MyApp.Accounts"}},
            {"caller": {"module": "MyApp.Web", "function": "show", "file": "lib/web.ex", "line": 20, "column": 5}, "type": "remote", "callee": {"arity": 1, "function": "get_user", "module": "MyApp.Accounts"}},
            {"caller": {"module": "MyApp.Web", "function": "edit", "file": "lib/web.ex", "line": 30, "column": 5}, "type": "remote", "callee": {"arity": 1, "function": "get_user", "module": "MyApp.Accounts"}},
            {"caller": {"module": "MyApp.Accounts", "function": "get_user", "file": "lib/accounts.ex", "line": 15, "column": 5}, "type": "remote", "callee": {"arity": 2, "function": "get", "module": "MyApp.Repo"}},
            {"caller": {"module": "MyApp.Service", "function": "process", "file": "lib/service.ex", "line": 10, "column": 5}, "type": "remote", "callee": {"arity": 1, "function": "log", "module": "Logger"}},
            {"caller": {"module": "MyApp.Service", "function": "process", "file": "lib/service.ex", "line": 11, "column": 5}, "type": "remote", "callee": {"arity": 2, "function": "get", "module": "MyApp.Repo"}},
            {"caller": {"module": "MyApp.Service", "function": "process", "file": "lib/service.ex", "line": 12, "column": 5}, "type": "remote", "callee": {"arity": 1, "function": "notify", "module": "MyApp.Notifier"}}
        ],
        "type_signatures": {}
    }"#;

    crate::execute_test_fixture! {
        fixture_name: populated_db,
        json: TEST_JSON,
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_hotspots_incoming,
        fixture: populated_db,
        cmd: HotspotsCmd {
            kind: HotspotKind::Incoming,
            module: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 20,
        },
        assertions: |result| {
            assert_eq!(result.kind, "incoming");
            assert!(!result.hotspots.is_empty());
            assert_eq!(result.hotspots[0].function, "get_user");
            assert_eq!(result.hotspots[0].incoming, 3);
        },
    }

    crate::execute_test! {
        test_name: test_hotspots_outgoing,
        fixture: populated_db,
        cmd: HotspotsCmd {
            kind: HotspotKind::Outgoing,
            module: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 20,
        },
        assertions: |result| {
            assert_eq!(result.kind, "outgoing");
            assert!(!result.hotspots.is_empty());
            assert_eq!(result.hotspots[0].function, "process");
            assert_eq!(result.hotspots[0].outgoing, 3);
        },
    }

    crate::execute_test! {
        test_name: test_hotspots_total,
        fixture: populated_db,
        cmd: HotspotsCmd {
            kind: HotspotKind::Total,
            module: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 20,
        },
        assertions: |result| {
            assert_eq!(result.kind, "total");
            assert!(!result.hotspots.is_empty());
            assert_eq!(result.hotspots[0].function, "get_user");
            assert_eq!(result.hotspots[0].total, 4);
        },
    }

    // =========================================================================
    // Filter tests
    // =========================================================================

    crate::execute_all_match_test! {
        test_name: test_hotspots_with_module_filter,
        fixture: populated_db,
        cmd: HotspotsCmd {
            kind: HotspotKind::Incoming,
            module: Some("Accounts".to_string()),
            project: "test_project".to_string(),
            regex: false,
            limit: 20,
        },
        collection: hotspots,
        condition: |h| h.module.contains("Accounts"),
    }

    crate::execute_limit_test! {
        test_name: test_hotspots_with_limit,
        fixture: populated_db,
        cmd: HotspotsCmd {
            kind: HotspotKind::Incoming,
            module: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 2,
        },
        collection: hotspots,
        limit: 2,
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: HotspotsCmd,
        cmd: HotspotsCmd {
            kind: HotspotKind::Incoming,
            module: None,
            project: "test_project".to_string(),
            regex: false,
            limit: 20,
        },
    }
}

use std::error::Error;

use clap::ValueEnum;
use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_f64, extract_i64, extract_string, run_query, Params};

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
    /// Functions with highest ratio of incoming to outgoing calls (boundary modules)
    Ratio,
    /// Modules with most functions (god modules)
    Functions,
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

/// Get function count per module
pub fn get_function_counts(
    db: &cozo::DbInstance,
    project: &str,
    module_pattern: Option<&str>,
    use_regex: bool,
) -> Result<std::collections::HashMap<String, i64>, Box<dyn Error>> {
    // Build optional module filter
    let module_filter = match module_pattern {
        Some(_) if use_regex => ", regex_matches(module, $module_pattern)".to_string(),
        Some(_) => ", str_includes(module, $module_pattern)".to_string(),
        None => String::new(),
    };

    let script = format!(
        r#"
        func_counts[module, count(name)] :=
            *function_locations{{project, module, name}},
            project == $project
            {module_filter}

        ?[module, func_count] :=
            func_counts[module, func_count]

        :order -func_count
        "#,
    );

    let mut params = Params::new();
    params.insert("project".to_string(), DataValue::Str(project.into()));
    if let Some(pattern) = module_pattern {
        params.insert("module_pattern".to_string(), DataValue::Str(pattern.into()));
    }

    let rows = run_query(db, &script, params).map_err(|e| HotspotsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut counts = std::collections::HashMap::new();
    for row in rows.rows {
        if row.len() >= 2 {
            if let Some(module) = extract_string(&row[0]) {
                let count = extract_i64(&row[1], 0);
                counts.insert(module, count);
            }
        }
    }

    Ok(counts)
}

pub fn find_hotspots(
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
        HotspotKind::Ratio => "ratio",
        HotspotKind::Functions => "incoming", // Functions uses incoming count for sorting
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

        # Combine counts with defaults of 0 and calculate ratio
        ?[module, function, incoming, outgoing, total, ratio] :=
            all_functions[module, function],
            incoming_counts[module, function, inc] or inc = 0,
            outgoing_counts[module, function, out] or out = 0,
            incoming = inc,
            outgoing = out,
            total = inc + out,
            ratio = if(out == 0, inc * 1000.0, inc / out)
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

    let rows = run_query(db, &script, params).map_err(|e| HotspotsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in rows.rows {
        if row.len() >= 6 {
            let Some(module) = extract_string(&row[0]) else { continue };
            let Some(function) = extract_string(&row[1]) else { continue };
            let incoming = extract_i64(&row[2], 0);
            let outgoing = extract_i64(&row[3], 0);
            let total = extract_i64(&row[4], 0);
            let ratio = extract_f64(&row[5], 0.0);

            results.push(Hotspot {
                module,
                function,
                incoming,
                outgoing,
                total,
                ratio,
            });
        }
    }

    Ok(results)
}

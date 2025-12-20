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
    exclude_generated: bool,
    require_outgoing: bool,
) -> Result<Vec<Hotspot>, Box<dyn Error>> {
    // Build optional module filter
    let module_filter = match module_pattern {
        Some(_) if use_regex => ", regex_matches(module, $module_pattern)".to_string(),
        Some(_) => ", str_includes(module, $module_pattern)".to_string(),
        None => String::new(),
    };

    // Build optional generated filter
    let generated_filter = if exclude_generated {
        ", generated_by == \"\"".to_string()
    } else {
        String::new()
    };

    // Build optional outgoing filter (for boundaries - exclude leaf nodes)
    let outgoing_filter = if require_outgoing {
        ", outgoing > 0".to_string()
    } else {
        String::new()
    };

    let order_by = match kind {
        HotspotKind::Incoming => "incoming",
        HotspotKind::Outgoing => "outgoing",
        HotspotKind::Total => "total",
        HotspotKind::Ratio => "ratio",
    };

    // Query to find hotspots by counting incoming and outgoing calls
    // We need to combine:
    // 1. Functions as callers (outgoing) - count unique callees
    // 2. Functions as callees (incoming) - count unique callers
    // Note: caller_function may have arity suffix (e.g., "format/1") while callee_function doesn't ("format")
    // We use callee_function as canonical name and match callers via starts_with
    // Excludes recursive calls and deduplicates via intermediate relations
    let script = format!(
        r#"
        # Get canonical function names (callee_function format, no arity suffix)
        # A function's canonical name is how it appears as a callee
        # Join with function_locations to filter generated functions
        canonical[module, function] :=
            *calls{{project, callee_module, callee_function}},
            *function_locations{{project, module: callee_module, name: callee_function, generated_by}},
            project == $project,
            module = callee_module,
            function = callee_function
            {generated_filter}

        # Distinct outgoing calls: match caller to canonical name
        # caller_function is either "name" or "name/N", canonical_name is "name"
        # Match: caller equals canonical OR starts with "canonical/"
        distinct_outgoing[caller_module, canonical_name, callee_module, callee_function] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function}},
            canonical[caller_module, canonical_name],
            project == $project,
            (caller_function == canonical_name or starts_with(caller_function, concat(canonical_name, "/")))

        # Count unique outgoing calls per function
        outgoing_counts[module, function, count(callee_function)] :=
            distinct_outgoing[module, function, callee_module, callee_function]

        # Distinct incoming calls
        distinct_incoming[callee_module, callee_function, caller_module, caller_function] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function}},
            canonical[callee_module, callee_function],
            project == $project

        # Count unique incoming calls per function
        incoming_counts[module, function, count(caller_function)] :=
            distinct_incoming[module, function, caller_module, caller_function]

        # Final query - functions with both incoming and outgoing
        # Ratio = incoming / outgoing (high ratio = many callers, few dependencies = boundary)
        ?[module, function, incoming, outgoing, total, ratio] :=
            incoming_counts[module, function, incoming],
            outgoing_counts[module, function, outgoing],
            total = incoming + outgoing,
            ratio = if(outgoing == 0, 9999.0, incoming / outgoing)
            {module_filter}
            {outgoing_filter}

        # Functions with only incoming (no outgoing) - leaf nodes
        # Excluded when require_outgoing is set
        ?[module, function, incoming, outgoing, total, ratio] :=
            incoming_counts[module, function, incoming],
            not outgoing_counts[module, function, _],
            outgoing = 0,
            total = incoming,
            ratio = 9999.0
            {module_filter}
            {outgoing_filter}

        # Functions with only outgoing (no incoming)
        ?[module, function, incoming, outgoing, total, ratio] :=
            outgoing_counts[module, function, outgoing],
            not incoming_counts[module, function, _],
            incoming = 0,
            total = outgoing,
            ratio = 0.0
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

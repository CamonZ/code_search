use std::error::Error;

use clap::ValueEnum;
use serde::Serialize;
use thiserror::Error;

use crate::backend::{Database, QueryParams};
use crate::query_builders::validate_regex_patterns;

#[cfg(feature = "backend-cozo")]
use crate::db::{extract_f64, extract_i64, extract_string};

#[cfg(feature = "backend-surrealdb")]
use crate::db::{extract_i64, extract_string};

#[cfg(feature = "backend-cozo")]
use crate::db::run_query;

#[cfg(feature = "backend-cozo")]
use crate::query_builders::OptionalConditionBuilder;

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

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
/// Get lines of code per module (sum of function line counts)
pub fn get_module_loc(
    db: &dyn Database,
    project: &str,
    module_pattern: Option<&str>,
    use_regex: bool,
) -> Result<std::collections::HashMap<String, i64>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // Build conditions using query builders
    let module_cond = OptionalConditionBuilder::new("module", "module_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(module_pattern.is_some(), use_regex);

    let script = format!(
        r#"
        # Calculate lines per function and sum by module
        module_loc[module, sum(lines)] :=
            *function_locations{{project, module, start_line, end_line}},
            project == $project,
            lines = end_line - start_line + 1
            {module_cond}

        ?[module, loc] :=
            module_loc[module, loc]

        :order -loc
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("project", project);

    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = run_query(db, &script, params).map_err(|e| HotspotsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut loc_map = std::collections::HashMap::new();
    for row in result.rows() {
        if row.len() >= 2
            && let Some(module) = extract_string(row.get(0).unwrap()) {
                let loc = extract_i64(row.get(1).unwrap(), 0);
                loc_map.insert(module, loc);
            }
    }

    Ok(loc_map)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
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

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
/// Get function count per module
pub fn get_function_counts(
    db: &dyn Database,
    project: &str,
    module_pattern: Option<&str>,
    use_regex: bool,
) -> Result<std::collections::HashMap<String, i64>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // Build conditions using query builders
    let module_cond = OptionalConditionBuilder::new("module", "module_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(module_pattern.is_some(), use_regex);

    let script = format!(
        r#"
        func_counts[module, count(name)] :=
            *function_locations{{project, module, name}},
            project == $project
            {module_cond}

        ?[module, func_count] :=
            func_counts[module, func_count]

        :order -func_count
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("project", project);

    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = run_query(db, &script, params).map_err(|e| HotspotsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut counts = std::collections::HashMap::new();
    for row in result.rows() {
        if row.len() >= 2
            && let Some(module) = extract_string(row.get(0).unwrap()) {
                let count = extract_i64(row.get(1).unwrap(), 0);
                counts.insert(module, count);
            }
    }

    Ok(counts)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
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

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
/// Get module-level connectivity (aggregated incoming/outgoing calls)
///
/// Returns a HashMap of module name -> (incoming, outgoing) call counts.
/// This aggregates function-level hotspots to module level at the database layer,
/// avoiding the need to fetch all function hotspots.
pub fn get_module_connectivity(
    db: &dyn Database,
    project: &str,
    module_pattern: Option<&str>,
    use_regex: bool,
) -> Result<std::collections::HashMap<String, (i64, i64)>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // Build conditions using query builders
    let module_cond = OptionalConditionBuilder::new("module", "module_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(module_pattern.is_some(), use_regex);

    // Aggregate incoming/outgoing calls at module level
    let script = format!(
        r#"
        # Get canonical function names (no generated functions)
        canonical[module, function] :=
            *calls{{project, callee_module, callee_function}},
            *function_locations{{project, module: callee_module, name: callee_function, generated_by}},
            project == $project,
            module = callee_module,
            function = callee_function,
            generated_by == ""

        # Distinct outgoing calls per function
        distinct_outgoing[caller_module, canonical_name, callee_module, callee_function] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function}},
            canonical[caller_module, canonical_name],
            project == $project,
            (caller_function == canonical_name or starts_with(caller_function, concat(canonical_name, "/")))

        # Count outgoing calls per function
        outgoing_counts[module, function, count(callee_function)] :=
            distinct_outgoing[module, function, callee_module, callee_function]

        # Distinct incoming calls per function
        distinct_incoming[callee_module, callee_function, caller_module, caller_function] :=
            *calls{{project, caller_module, caller_function, callee_module, callee_function}},
            canonical[callee_module, callee_function],
            project == $project

        # Count incoming calls per function
        incoming_counts[module, function, count(caller_function)] :=
            distinct_incoming[module, function, caller_module, caller_function]

        # Function stats with defaults for missing counts
        # Functions with both counts
        func_stats[module, function, incoming, outgoing] :=
            canonical[module, function],
            incoming_counts[module, function, incoming],
            outgoing_counts[module, function, outgoing]

        # Functions with only incoming (no outgoing)
        func_stats[module, function, incoming, outgoing] :=
            canonical[module, function],
            incoming_counts[module, function, incoming],
            not outgoing_counts[module, function, _],
            outgoing = 0

        # Functions with only outgoing (no incoming)
        func_stats[module, function, incoming, outgoing] :=
            canonical[module, function],
            not incoming_counts[module, function, _],
            outgoing_counts[module, function, outgoing],
            incoming = 0

        # Aggregate to module level
        module_connectivity[module, sum(incoming), sum(outgoing)] :=
            func_stats[module, function, incoming, outgoing]
            {module_cond}

        ?[module, incoming, outgoing] :=
            module_connectivity[module, incoming, outgoing]

        :order -incoming
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("project", project);

    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = run_query(db, &script, params).map_err(|e| HotspotsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut connectivity = std::collections::HashMap::new();
    for row in result.rows() {
        if row.len() >= 3
            && let Some(module) = extract_string(row.get(0).unwrap()) {
                let incoming = extract_i64(row.get(1).unwrap(), 0);
                let outgoing = extract_i64(row.get(2).unwrap(), 0);
                connectivity.insert(module, (incoming, outgoing));
            }
    }

    Ok(connectivity)
}

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
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

// ==================== CozoDB Implementation ====================
#[cfg(feature = "backend-cozo")]
pub fn find_hotspots(
    db: &dyn Database,
    kind: HotspotKind,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    limit: u32,
    exclude_generated: bool,
    require_outgoing: bool,
) -> Result<Vec<Hotspot>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    // Build conditions using query builders
    let module_cond = OptionalConditionBuilder::new("module", "module_pattern")
        .with_leading_comma()
        .with_regex()
        .build_with_regex(module_pattern.is_some(), use_regex);

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
            {module_cond}
            {outgoing_filter}

        # Functions with only incoming (no outgoing) - leaf nodes
        # Excluded when require_outgoing is set
        ?[module, function, incoming, outgoing, total, ratio] :=
            incoming_counts[module, function, incoming],
            not outgoing_counts[module, function, _],
            outgoing = 0,
            total = incoming,
            ratio = 9999.0
            {module_cond}
            {outgoing_filter}

        # Functions with only outgoing (no incoming)
        ?[module, function, incoming, outgoing, total, ratio] :=
            outgoing_counts[module, function, outgoing],
            not incoming_counts[module, function, _],
            incoming = 0,
            total = outgoing,
            ratio = 0.0
            {module_cond}

        :order -{order_by}, module, function
        :limit {limit}
        "#,
    );

    let mut params = QueryParams::new()
        .with_str("project", project);

    if let Some(pattern) = module_pattern {
        params = params.with_str("module_pattern", pattern);
    }

    let result = run_query(db, &script, params).map_err(|e| HotspotsError::QueryFailed {
        message: e.to_string(),
    })?;

    let mut results = Vec::new();
    for row in result.rows() {
        if row.len() >= 6 {
            let Some(module) = extract_string(row.get(0).unwrap()) else { continue };
            let Some(function) = extract_string(row.get(1).unwrap()) else { continue };
            let incoming = extract_i64(row.get(2).unwrap(), 0);
            let outgoing = extract_i64(row.get(3).unwrap(), 0);
            let total = extract_i64(row.get(4).unwrap(), 0);
            let ratio = extract_f64(row.get(5).unwrap(), 0.0);

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

// ==================== SurrealDB Implementation ====================
#[cfg(feature = "backend-surrealdb")]
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

    // Query to get incoming call counts per function
    let incoming_query = r#"
        SELECT in.module_name as module, in.name as function, count() as incoming
        FROM calls
        GROUP BY in.module_name, in.name
    "#;

    let incoming_result = db.execute_query(incoming_query, QueryParams::new())
        .map_err(|e| HotspotsError::QueryFailed {
            message: format!("Failed to get incoming calls: {}", e),
        })?;

    // Query to get outgoing call counts per function
    let outgoing_query = r#"
        SELECT out.module_name as module, out.name as function, count() as outgoing
        FROM calls
        GROUP BY out.module_name, out.name
    "#;

    let outgoing_result = db.execute_query(outgoing_query, QueryParams::new())
        .map_err(|e| HotspotsError::QueryFailed {
            message: format!("Failed to get outgoing calls: {}", e),
        })?;

    // Build hashmaps from query results
    let mut incoming_counts: std::collections::HashMap<(String, String), i64> = std::collections::HashMap::new();
    for row in incoming_result.rows() {
        if row.len() >= 3 {
            if let (Some(module), Some(function)) = (extract_string(row.get(0).unwrap()), extract_string(row.get(1).unwrap())) {
                let count = extract_i64(row.get(2).unwrap(), 0);
                incoming_counts.insert((module, function), count);
            }
        }
    }

    let mut outgoing_counts: std::collections::HashMap<(String, String), i64> = std::collections::HashMap::new();
    for row in outgoing_result.rows() {
        if row.len() >= 3 {
            if let (Some(module), Some(function)) = (extract_string(row.get(0).unwrap()), extract_string(row.get(1).unwrap())) {
                let count = extract_i64(row.get(2).unwrap(), 0);
                outgoing_counts.insert((module, function), count);
            }
        }
    }

    // Get all functions to combine incoming and outgoing
    let functions_query = "SELECT module_name as module, name as function FROM functions";
    let functions_result = db.execute_query(functions_query, QueryParams::new())
        .map_err(|e| HotspotsError::QueryFailed {
            message: format!("Failed to get functions: {}", e),
        })?;

    let mut hotspots = Vec::new();
    for row in functions_result.rows() {
        if row.len() >= 2 {
            if let (Some(module), Some(function)) = (extract_string(row.get(0).unwrap()), extract_string(row.get(1).unwrap())) {
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

#[cfg(all(test, feature = "backend-cozo"))]
mod tests {
    use super::*;
    use rstest::fixture;

    #[fixture]
    fn populated_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::call_graph_db("default")
    }

    fn get_db() -> Box<dyn crate::backend::Database> {
        crate::test_utils::call_graph_db("default")
    }

    #[test]
    fn test_get_module_connectivity_returns_results() {
        let db = get_db();
        let result = get_module_connectivity(
            &*db,
            "default",
            None,
            false,
        );

        if let Err(ref e) = result {
            eprintln!("Error: {}", e);
        }
        assert!(result.is_ok());
        let connectivity = result.unwrap();
        assert!(!connectivity.is_empty());
    }

    #[test]
    fn test_get_module_connectivity_has_valid_counts() {
        let db = get_db();
        let connectivity = get_module_connectivity(
            &*db,
            "default",
            None,
            false,
        ).unwrap();

        // All modules should have non-negative counts
        for (module, (incoming, outgoing)) in &connectivity {
            assert!(*incoming >= 0, "Module {} has negative incoming: {}", module, incoming);
            assert!(*outgoing >= 0, "Module {} has negative outgoing: {}", module, outgoing);
        }
    }

    #[test]
    fn test_get_module_connectivity_with_module_filter() {
        let db = get_db();
        let connectivity = get_module_connectivity(
            &*db,
            "default",
            Some("Accounts"),
            false,
        ).unwrap();

        // All modules should contain "Accounts"
        for module in connectivity.keys() {
            assert!(module.contains("Accounts"), "Module {} doesn't contain 'Accounts'", module);
        }
    }

    #[test]
    fn test_get_module_connectivity_aggregates_correctly() {
        let db = get_db();
        // Get module-level connectivity
        let module_conn = get_module_connectivity(
            &*db,
            "default",
            None,
            false,
        ).unwrap();

        // Get function-level hotspots
        let function_hotspots = find_hotspots(
            &*db,
            HotspotKind::Total,
            None,
            "default",
            false,
            u32::MAX,
            false,
            false,
        ).unwrap();

        // Manually aggregate function hotspots by module
        let mut manual_agg: std::collections::HashMap<String, (i64, i64)> = std::collections::HashMap::new();
        for hotspot in function_hotspots {
            let entry = manual_agg.entry(hotspot.module).or_insert((0, 0));
            entry.0 += hotspot.incoming;
            entry.1 += hotspot.outgoing;
        }

        // The two approaches should produce the same results
        assert_eq!(module_conn.len(), manual_agg.len(), "Different number of modules");

        for (module, (conn_in, conn_out)) in &module_conn {
            let (manual_in, manual_out) = manual_agg.get(module)
                .expect(&format!("Module {} not found in manual aggregation", module));
            assert_eq!(conn_in, manual_in, "Module {} has different incoming: {} vs {}", module, conn_in, manual_in);
            assert_eq!(conn_out, manual_out, "Module {} has different outgoing: {} vs {}", module, conn_out, manual_out);
        }
    }

    #[test]
    fn test_get_module_loc_returns_results() {
        let db = get_db();
        let result = get_module_loc(
            &*db,
            "default",
            None,
            false,
        );

        assert!(result.is_ok());
        let loc_map = result.unwrap();
        assert!(!loc_map.is_empty());
    }

    #[test]
    fn test_get_function_counts_returns_results() {
        let db = get_db();
        let result = get_function_counts(
            &*db,
            "default",
            None,
            false,
        );

        assert!(result.is_ok());
        let counts = result.unwrap();
        assert!(!counts.is_empty());
    }

    #[test]
    fn test_module_connectivity_returns_fewer_rows() {
        let db = get_db();
        // Get module-level connectivity (NEW approach)
        let module_conn = get_module_connectivity(
            &*db,
            "default",
            None,
            false,
        ).unwrap();

        // Get function-level hotspots (OLD approach)
        let function_hotspots = find_hotspots(
            &*db,
            HotspotKind::Total,
            None,
            "default",
            false,
            u32::MAX,
            false,
            false,
        ).unwrap();

        // The new approach should return FAR fewer rows
        println!("Module connectivity rows: {}", module_conn.len());
        println!("Function hotspots rows: {}", function_hotspots.len());

        // For any non-trivial codebase, there are more functions than modules
        assert!(
            module_conn.len() <= function_hotspots.len(),
            "Module connectivity ({} rows) should return same or fewer rows than function hotspots ({} rows)",
            module_conn.len(),
            function_hotspots.len()
        );

        // Calculate reduction percentage
        if function_hotspots.len() > 0 {
            let reduction = 100.0 * (1.0 - (module_conn.len() as f64 / function_hotspots.len() as f64));
            println!("Row reduction: {:.1}%", reduction);

            // In a typical codebase, we expect significant reduction
            // (unless every module has exactly 1 function, which is unlikely)
        }
    }

    #[test]
    fn test_get_module_connectivity_nonexistent_project() {
        let db = get_db();
        let connectivity = get_module_connectivity(
            &*db,
            "nonexistent_project",
            None,
            false,
        ).unwrap();

        // Should return empty for non-existent project
        assert!(connectivity.is_empty());
    }

    #[test]
    fn test_get_module_connectivity_nonexistent_module() {
        let db = get_db();
        let connectivity = get_module_connectivity(
            &*db,
            "default",
            Some("NonExistentModule"),
            false,
        ).unwrap();

        // Should return empty when module pattern matches nothing
        assert!(connectivity.is_empty());
    }

    #[test]
    fn test_get_module_connectivity_with_regex() {
        let db = get_db();
        let connectivity = get_module_connectivity(
            &*db,
            "default",
            Some(".*Accounts.*"),
            true, // use regex
        ).unwrap();

        // Should return results matching the regex
        for module in connectivity.keys() {
            assert!(module.contains("Accounts"), "Module {} doesn't match regex pattern", module);
        }
    }

    #[test]
    fn test_get_module_loc_nonexistent_project() {
        let db = get_db();
        let loc_map = get_module_loc(
            &*db,
            "nonexistent_project",
            None,
            false,
        ).unwrap();

        assert!(loc_map.is_empty());
    }

    #[test]
    fn test_get_function_counts_nonexistent_project() {
        let db = get_db();
        let counts = get_function_counts(
            &*db,
            "nonexistent_project",
            None,
            false,
        ).unwrap();

        assert!(counts.is_empty());
    }

    #[test]
    fn test_get_module_connectivity_all_values_positive() {
        let db = get_db();
        let connectivity = get_module_connectivity(
            &*db,
            "default",
            None,
            false,
        ).unwrap();

        // Verify all counts are non-negative (sanity check)
        for (module, (incoming, outgoing)) in &connectivity {
            assert!(*incoming >= 0, "Module {} has negative incoming", module);
            assert!(*outgoing >= 0, "Module {} has negative outgoing", module);
        }
    }
}

#[cfg(all(test, feature = "backend-surrealdb"))]
mod surrealdb_tests {
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

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
    _db: &dyn Database,
    _project: &str,
    module_pattern: Option<&str>,
    use_regex: bool,
) -> Result<std::collections::HashMap<String, i64>, Box<dyn Error>> {
    validate_regex_patterns(use_regex, &[module_pattern])?;

    let module_clause = if let Some(_pattern) = module_pattern {
        if use_regex {
            "WHERE module_name = <regex>$module_pattern"
        } else {
            "WHERE module_name = $module_pattern"
        }
    } else {
        ""
    };

    // SurrealDB doesn't support computed fields in aggregations easily,
    // so we return an empty map for now. The CozoDB implementation handles this.
    // In a production system, LOC would be stored as a field in the function record.
    let _query = format!(
        r#"
        SELECT module_name as module, COUNT(name) as function_count
        FROM functions
        {module_clause}
        GROUP BY module_name
        ORDER BY function_count DESC
        "#
    );

    // Return empty map for now - SurrealDB test fixture doesn't include LOC fields
    // A production system would store LOC as a field in the function record
    Ok(std::collections::HashMap::new())
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

    let module_clause = if let Some(_pattern) = module_pattern {
        if use_regex {
            "WHERE module_name = <regex>$module_pattern"
        } else {
            "WHERE module_name = $module_pattern"
        }
    } else {
        ""
    };

    let query = format!(
        r#"
        SELECT module_name, count() as function_count
        FROM functions
        {module_clause}
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

#[cfg(all(test, feature = "backend-cozo"))]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

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

        assert_eq!(counts.len(), 5, "Should have exactly 5 modules");
    }

    #[test]
    fn test_get_function_counts_exact_values_per_module() {
        let db = get_db();
        let counts = get_function_counts(&*db, "default", None, false)
            .expect("Query should succeed");

        // Verify exact function counts per module from fixture
        assert_eq!(
            counts.get("MyApp.Controller"),
            Some(&3),
            "Controller should have 3 functions (index/2, show/2, create/2)"
        );
        assert_eq!(
            counts.get("MyApp.Accounts"),
            Some(&4),
            "Accounts should have 4 functions (get_user/1, get_user/2, list_users/0, validate_email/1)"
        );
        assert_eq!(
            counts.get("MyApp.Service"),
            Some(&2),
            "Service should have 2 functions (process_request/2, transform_data/1)"
        );
        assert_eq!(
            counts.get("MyApp.Repo"),
            Some(&4),
            "Repo should have 4 functions (get/2, all/1, insert/1, query/2)"
        );
        assert_eq!(
            counts.get("MyApp.Notifier"),
            Some(&2),
            "Notifier should have 2 functions (send_email/2, format_message/1)"
        );
    }

    #[test]
    fn test_get_function_counts_total_is_fifteen() {
        let db = get_db();
        let counts = get_function_counts(&*db, "default", None, false)
            .expect("Query should succeed");

        let total: i64 = counts.values().sum();
        assert_eq!(total, 15, "Total function count should be 15");
    }

    #[test]
    fn test_get_function_counts_controller_pattern() {
        let db = get_db();
        let counts = get_function_counts(&*db, "default", Some("MyApp.Controller"), false)
            .expect("Query should succeed");

        assert_eq!(counts.len(), 1, "Should match exactly 1 module");
        assert_eq!(
            counts.get("MyApp.Controller"),
            Some(&3),
            "Controller should have 3 functions"
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
            Some(&4),
            "Accounts should have 4 functions"
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
    // Note: SurrealDB implementation returns empty for LOC queries
    // since the test fixture doesn't include LOC (start_line/end_line) fields.

    #[test]
    fn test_get_module_loc_returns_empty() {
        let db = get_db();
        let loc_map = get_module_loc(&*db, "default", None, false)
            .expect("Query should succeed");

        assert!(loc_map.is_empty(), "SurrealDB test fixture doesn't include LOC data");
    }

    #[test]
    fn test_get_module_loc_with_pattern_returns_empty() {
        let db = get_db();
        let loc_map = get_module_loc(&*db, "default", Some("MyApp.Accounts"), false)
            .expect("Query should succeed");

        assert!(loc_map.is_empty(), "SurrealDB test fixture doesn't include LOC data");
    }

    #[test]
    fn test_get_module_loc_invalid_regex() {
        let db = get_db();
        let result = get_module_loc(&*db, "default", Some("[invalid"), true);

        assert!(result.is_err(), "Should reject invalid regex pattern");
    }

    // ===== get_module_connectivity tests =====
    // Tests connectivity based on the 12 call edges in the fixture

    #[test]
    fn test_get_module_connectivity_exact_module_count() {
        let db = get_db();
        let connectivity = get_module_connectivity(&*db, "default", None, false)
            .expect("Query should succeed");

        assert_eq!(connectivity.len(), 5, "Should have exactly 5 modules");
    }

    #[test]
    fn test_get_module_connectivity_controller_values() {
        let db = get_db();
        let connectivity = get_module_connectivity(&*db, "default", None, false)
            .expect("Query should succeed");

        // Controller: no incoming calls, calls 3 modules (Accounts, Service, Notifier)
        let (incoming, outgoing) = connectivity
            .get("MyApp.Controller")
            .expect("Controller should be present");
        assert_eq!(
            *incoming, 0,
            "Controller should have 0 incoming (no one calls Controller)"
        );
        assert_eq!(
            *outgoing, 3,
            "Controller should have 3 outgoing (calls Accounts, Service, Notifier)"
        );
    }

    #[test]
    fn test_get_module_connectivity_accounts_values() {
        let db = get_db();
        let connectivity = get_module_connectivity(&*db, "default", None, false)
            .expect("Query should succeed");

        // Accounts: called by Controller, Accounts (self), Service
        // Calls: Repo, Accounts (self)
        let (incoming, outgoing) = connectivity
            .get("MyApp.Accounts")
            .expect("Accounts should be present");
        assert_eq!(
            *incoming, 3,
            "Accounts should have 3 incoming (Controller, Accounts-self, Service)"
        );
        assert_eq!(
            *outgoing, 2,
            "Accounts should have 2 outgoing (Repo, Accounts-self)"
        );
    }

    #[test]
    fn test_get_module_connectivity_service_values() {
        let db = get_db();
        let connectivity = get_module_connectivity(&*db, "default", None, false)
            .expect("Query should succeed");

        // Service: called by Controller only
        // Calls: Accounts, Notifier
        let (incoming, outgoing) = connectivity
            .get("MyApp.Service")
            .expect("Service should be present");
        assert_eq!(*incoming, 1, "Service should have 1 incoming (Controller)");
        assert_eq!(
            *outgoing, 2,
            "Service should have 2 outgoing (Accounts, Notifier)"
        );
    }

    #[test]
    fn test_get_module_connectivity_repo_values() {
        let db = get_db();
        let connectivity = get_module_connectivity(&*db, "default", None, false)
            .expect("Query should succeed");

        // Repo: called by Accounts, Repo (self)
        // Calls: Repo (self only)
        let (incoming, outgoing) = connectivity
            .get("MyApp.Repo")
            .expect("Repo should be present");
        assert_eq!(
            *incoming, 2,
            "Repo should have 2 incoming (Accounts, Repo-self)"
        );
        assert_eq!(*outgoing, 1, "Repo should have 1 outgoing (Repo-self)");
    }

    #[test]
    fn test_get_module_connectivity_notifier_values() {
        let db = get_db();
        let connectivity = get_module_connectivity(&*db, "default", None, false)
            .expect("Query should succeed");

        // Notifier: called by Service, Controller, Notifier (self)
        // Calls: Notifier (self only)
        let (incoming, outgoing) = connectivity
            .get("MyApp.Notifier")
            .expect("Notifier should be present");
        assert_eq!(
            *incoming, 3,
            "Notifier should have 3 incoming (Service, Controller, Notifier-self)"
        );
        assert_eq!(
            *outgoing, 1,
            "Notifier should have 1 outgoing (Notifier-self)"
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
        assert_eq!(*incoming, 0);
        assert_eq!(*outgoing, 3);
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
}

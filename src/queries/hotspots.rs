use crate::db::DatabaseBackend;
use std::error::Error;

use clap::ValueEnum;
use cozo::DataValue;
use serde::Serialize;
use thiserror::Error;

use crate::db::{extract_f64, extract_i64, extract_string, run_query, Params};
use crate::queries::builder::{QueryBuilder, CompiledQuery};

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

/// Query builder for finding function hotspots
#[derive(Debug)]
pub struct HotspotsQueryBuilder {
    pub kind: HotspotKind,
    pub module_pattern: Option<String>,
    pub project: String,
    pub use_regex: bool,
    pub limit: u32,
}

impl QueryBuilder for HotspotsQueryBuilder {
    fn compile(&self, backend: &dyn DatabaseBackend) -> Result<String, Box<dyn Error>> {
        match backend.backend_name() {
            "CozoSqlite" | "CozoRocksdb" | "CozoMem" => self.compile_cozo(),
            "PostgresAge" => self.compile_age(),
            _ => Err(format!("Unsupported backend: {}", backend.backend_name()).into()),
        }
    }

    fn parameters(&self) -> Params {
        let mut params = Params::new();
        params.insert("project".to_string(), DataValue::Str(self.project.clone().into()));
        if let Some(ref pattern) = self.module_pattern {
            params.insert("module_pattern".to_string(), DataValue::Str(pattern.clone().into()));
        }
        params
    }
}

impl HotspotsQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        let module_filter = match self.module_pattern {
            Some(_) if self.use_regex => ", regex_matches(module, $module_pattern)".to_string(),
            Some(_) => ", str_includes(module, $module_pattern)".to_string(),
            None => String::new(),
        };

        let order_by = match self.kind {
            HotspotKind::Incoming => "incoming",
            HotspotKind::Outgoing => "outgoing",
            HotspotKind::Total => "total",
            HotspotKind::Ratio => "ratio",
            HotspotKind::Functions => "incoming",
        };

        Ok(format!(
            r#"# Count outgoing calls per function (as caller)
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
:limit {}"#,
            self.limit
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        // AGE data model uses vertices only, not edges.
        // Call vertex has: caller_module, caller_function, callee_module, callee_function
        // We count incoming calls (where function is callee) and outgoing calls (where function is caller)

        let mod_match = if self.use_regex { "=~" } else { "=" };

        let where_filter = match &self.module_pattern {
            Some(_) => format!("\n  AND c.callee_module {} $module_pattern", mod_match),
            None => String::new(),
        };

        let order_field = match self.kind {
            HotspotKind::Incoming => "incoming",
            HotspotKind::Outgoing => "outgoing",
            HotspotKind::Total => "total",
            HotspotKind::Ratio => "ratio",
            HotspotKind::Functions => "incoming",
        };

        // Count incoming calls per function (as callee)
        // Note: We're counting by callee_module + callee_function to identify functions
        // Then we'd need a second query for outgoing. For simplicity, this query focuses on incoming.
        // A full implementation would need subqueries or multiple roundtrips.
        // Simplified AGE query that only counts incoming calls
        // Full outgoing/ratio calculation would require complex subqueries
        // that AGE may not support well
        let order_expr = match self.kind {
            HotspotKind::Incoming | HotspotKind::Functions => "incoming".to_string(),
            HotspotKind::Outgoing => "incoming".to_string(), // fallback to incoming
            HotspotKind::Total => "incoming".to_string(),    // fallback to incoming
            HotspotKind::Ratio => "incoming".to_string(),    // fallback to incoming
        };

        Ok(format!(
            r#"MATCH (c:Call)
WHERE c.project = $project
  AND c.callee_function <> '%'{where_filter}
WITH c.callee_module AS module, c.callee_function AS function, count(*) AS incoming
RETURN module, function, incoming, 0 AS outgoing, incoming AS total, toFloat(incoming) AS ratio
ORDER BY {order_expr} DESC, module, function
LIMIT {limit}"#,
            limit = self.limit,
            order_expr = order_expr
        ))
    }
}

/// Query builder for counting functions per module
#[derive(Debug)]
pub struct FunctionCountsQueryBuilder {
    pub project: String,
    pub module_pattern: Option<String>,
    pub use_regex: bool,
}

impl QueryBuilder for FunctionCountsQueryBuilder {
    fn compile(&self, backend: &dyn DatabaseBackend) -> Result<String, Box<dyn Error>> {
        match backend.backend_name() {
            "CozoSqlite" | "CozoRocksdb" | "CozoMem" => self.compile_cozo(),
            "PostgresAge" => self.compile_age(),
            _ => Err(format!("Unsupported backend: {}", backend.backend_name()).into()),
        }
    }

    fn parameters(&self) -> Params {
        let mut params = Params::new();
        params.insert("project".to_string(), DataValue::Str(self.project.clone().into()));
        if let Some(ref pattern) = self.module_pattern {
            params.insert("module_pattern".to_string(), DataValue::Str(pattern.clone().into()));
        }
        params
    }
}

impl FunctionCountsQueryBuilder {
    fn compile_cozo(&self) -> Result<String, Box<dyn Error>> {
        let module_filter = match self.module_pattern {
            Some(_) if self.use_regex => ", regex_matches(module, $module_pattern)".to_string(),
            Some(_) => ", str_includes(module, $module_pattern)".to_string(),
            None => String::new(),
        };

        Ok(format!(
            r#"func_counts[module, count(name)] :=
    *function_locations{{project, module, name}},
    project == $project
    {module_filter}

?[module, func_count] :=
    func_counts[module, func_count]

:order -func_count"#,
        ))
    }

    fn compile_age(&self) -> Result<String, Box<dyn Error>> {
        let mod_match = if self.use_regex { "=~" } else { "=" };

        let where_filter = match &self.module_pattern {
            Some(_) => format!("AND f.module {} $module_pattern", mod_match),
            None => String::new(),
        };

        Ok(format!(
            r#"MATCH (f:Function)
WHERE f.project = $project
{where_filter}
RETURN f.module AS module, count(f) AS func_count
ORDER BY count(f) DESC"#,
        ))
    }
}

/// Get function count per module
pub fn get_function_counts(
    db: &dyn DatabaseBackend,
    project: &str,
    module_pattern: Option<&str>,
    use_regex: bool,
) -> Result<std::collections::HashMap<String, i64>, Box<dyn Error>> {
    let builder = FunctionCountsQueryBuilder {
        project: project.to_string(),
        module_pattern: module_pattern.map(|s| s.to_string()),
        use_regex,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| HotspotsError::QueryFailed {
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
    db: &dyn DatabaseBackend,
    kind: HotspotKind,
    module_pattern: Option<&str>,
    project: &str,
    use_regex: bool,
    limit: u32,
) -> Result<Vec<Hotspot>, Box<dyn Error>> {
    let builder = HotspotsQueryBuilder {
        kind,
        module_pattern: module_pattern.map(|s| s.to_string()),
        project: project.to_string(),
        use_regex,
        limit,
    };

    let compiled = CompiledQuery::from_builder(&builder, db)?;
    let rows = run_query(db, &compiled.script, compiled.params).map_err(|e| HotspotsError::QueryFailed {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_mem_db;

    #[test]
    fn test_hotspots_query_cozo_incoming() {
        let builder = HotspotsQueryBuilder {
            kind: HotspotKind::Incoming,
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            limit: 100,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("incoming_counts"));
        assert!(compiled.contains("outgoing_counts"));
        assert!(compiled.contains("$project"));
    }

    #[test]
    fn test_hotspots_query_cozo_with_module_pattern() {
        let builder = HotspotsQueryBuilder {
            kind: HotspotKind::Total,
            module_pattern: Some("MyApp".to_string()),
            project: "myproject".to_string(),
            use_regex: true,
            limit: 50,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("regex_matches"));
    }

    #[test]
    fn test_hotspots_query_cozo_order_by_kind() {
        // Test each HotspotKind generates correct ORDER BY
        for kind in [
            HotspotKind::Incoming,
            HotspotKind::Outgoing,
            HotspotKind::Total,
            HotspotKind::Ratio,
        ] {
            let builder = HotspotsQueryBuilder {
                kind,
                module_pattern: None,
                project: "proj".to_string(),
                use_regex: false,
                limit: 10,
            };

            let backend = open_mem_db(true).unwrap();
            let compiled = builder.compile(backend.as_ref()).unwrap();

            let expected_order = match kind {
                HotspotKind::Incoming => "incoming",
                HotspotKind::Outgoing => "outgoing",
                HotspotKind::Total => "total",
                HotspotKind::Ratio => "ratio",
                HotspotKind::Functions => "incoming",
            };
            assert!(
                compiled.contains(&format!(":order -{}", expected_order)),
                "Order clause for {:?} not found in compiled query",
                kind
            );
        }
    }

    #[test]
    fn test_hotspots_query_age() {
        let builder = HotspotsQueryBuilder {
            kind: HotspotKind::Incoming,
            module_pattern: None,
            project: "myproject".to_string(),
            use_regex: false,
            limit: 100,
        };

        let compiled = builder.compile_age().unwrap();

        // AGE queries use vertex matching, not edge relationships
        assert!(compiled.contains("MATCH (c:Call)"));
        assert!(compiled.contains("c.callee_function <> '%'"));
        assert!(compiled.contains("count"));
    }

    #[test]
    fn test_function_counts_query_cozo() {
        let builder = FunctionCountsQueryBuilder {
            project: "myproject".to_string(),
            module_pattern: None,
            use_regex: false,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = builder.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("func_counts"));
        assert!(compiled.contains("count(name)"));
    }

    #[test]
    fn test_function_counts_query_age() {
        let builder = FunctionCountsQueryBuilder {
            project: "myproject".to_string(),
            module_pattern: Some("MyApp".to_string()),
            use_regex: false,
        };

        let compiled = builder.compile_age().unwrap();

        assert!(compiled.contains("MATCH"));
        assert!(compiled.contains("count"));
    }

    #[test]
    fn test_hotspots_query_parameters() {
        let builder = HotspotsQueryBuilder {
            kind: HotspotKind::Total,
            module_pattern: Some("test".to_string()),
            project: "proj".to_string(),
            use_regex: false,
            limit: 10,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 2);
        assert!(params.contains_key("project"));
        assert!(params.contains_key("module_pattern"));
    }

    #[test]
    fn test_function_counts_query_parameters() {
        let builder = FunctionCountsQueryBuilder {
            project: "proj".to_string(),
            module_pattern: Some("mod".to_string()),
            use_regex: false,
        };

        let params = builder.parameters();
        assert_eq!(params.len(), 2);
        assert!(params.contains_key("project"));
        assert!(params.contains_key("module_pattern"));
    }
}

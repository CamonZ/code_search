//! Common query patterns for backend-agnostic query building.
//!
//! This module provides pre-built query builders for common patterns like
//! simple SELECT queries and recursive traversal queries.

use super::{compilers::get_compiler, QueryBuilder};
use crate::db::{DatabaseBackend, Params};
use std::error::Error;

/// A simple SELECT query builder.
///
/// Compiles to:
/// - **Cozo**: `?[fields] := *relation{fields}, filters`
/// - **AGE**: `MATCH (m:relation) WHERE filters RETURN m.fields`
#[derive(Debug, Clone)]
pub struct SelectQuery {
    /// The relation/table/node label name
    pub relation: &'static str,
    /// Fields to select
    pub fields: Vec<&'static str>,
    /// Filter expressions (field op param, param placeholder)
    pub filters: Vec<(String, String)>,
    /// Optional LIMIT clause
    pub limit: Option<usize>,
}

impl QueryBuilder for SelectQuery {
    fn compile(&self, backend: &dyn DatabaseBackend) -> Result<String, Box<dyn Error>> {
        match backend.backend_name() {
            "CozoSqlite" | "CozoRocksdb" | "CozoMem" => {
                // Cozo Datalog: ?[f1, f2] := *relation{f1, f2}, filter1, filter2, ... :limit N
                let fields = self.fields.join(", ");
                let field_bindings = self
                    .fields
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                let mut query =
                    format!("?[{}] := *{}{{{}}}", fields, self.relation, field_bindings);

                if !self.filters.is_empty() {
                    query.push_str(",\n    ");
                    let filter_exprs = self
                        .filters
                        .iter()
                        .map(|(expr, _)| expr.clone())
                        .collect::<Vec<_>>()
                        .join(",\n    ");
                    query.push_str(&filter_exprs);
                }

                if let Some(limit) = self.limit {
                    query.push_str(&format!("\n:limit {}", limit));
                }

                Ok(query)
            }
            "PostgresAge" => {
                // AGE Cypher: MATCH (m:relation) WHERE filters RETURN m.field1, m.field2, ... LIMIT N
                let return_fields = self
                    .fields
                    .iter()
                    .map(|f| format!("m.{}", f))
                    .collect::<Vec<_>>()
                    .join(", ");

                let mut query = format!("MATCH (m:{}) ", self.relation);

                if !self.filters.is_empty() {
                    query.push_str("WHERE ");
                    let filter_exprs = self
                        .filters
                        .iter()
                        .map(|(expr, _)| expr.clone())
                        .collect::<Vec<_>>()
                        .join(" AND ");
                    query.push_str(&filter_exprs);
                }

                query.push_str(&format!(" RETURN {}", return_fields));

                if let Some(limit) = self.limit {
                    query.push_str(&format!(" LIMIT {}", limit));
                }

                Ok(query)
            }
            _ => Err("Unsupported backend".into()),
        }
    }

    fn parameters(&self) -> Params {
        Params::new() // Parameters are set via filters
    }
}

/// A recursive query builder for graph traversal.
///
/// Supports depth-limited recursive queries for tracing call paths,
/// dependencies, etc.
///
/// Compiles to:
/// - **Cozo**: Recursive rule definition with depth tracking
/// - **AGE**: Path queries with edge relationships
#[derive(Debug, Clone)]
pub struct RecursiveQuery {
    /// The starting relation/node label
    pub start_relation: &'static str,
    /// The recursive step expression/relationship
    pub recursive_step: &'static str,
    /// Maximum recursion depth (if Some)
    pub depth_limit: Option<usize>,
    /// Fields to return
    pub return_fields: Vec<&'static str>,
}

impl QueryBuilder for RecursiveQuery {
    fn compile(&self, backend: &dyn DatabaseBackend) -> Result<String, Box<dyn Error>> {
        match backend.backend_name() {
            "CozoSqlite" | "CozoRocksdb" | "CozoMem" => {
                // Cozo native recursion syntax
                // Will be implemented when schema definitions are available (Ticket #47)
                Err("Recursive Cozo queries require schema definitions (Ticket #47+)".into())
            }
            "PostgresAge" => {
                // AGE path syntax: -[*1..depth]->
                // Will be implemented when schema definitions are available (Ticket #47+)
                Err("Recursive AGE queries require schema definitions (Ticket #47+)".into())
            }
            _ => Err("Unsupported backend".into()),
        }
    }

    fn parameters(&self) -> Params {
        Params::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_mem_db;

    #[test]
    fn test_select_query_cozo_compilation() {
        let query = SelectQuery {
            relation: "modules",
            fields: vec!["project", "name"],
            filters: vec![],
            limit: Some(10),
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = query.compile(backend.as_ref()).unwrap();

        // Verify Cozo syntax
        assert!(compiled.contains("?[project, name]"));
        assert!(compiled.contains("*modules"));
        assert!(compiled.contains(":limit 10"));
    }

    #[test]
    fn test_select_query_cozo_with_filters() {
        let query = SelectQuery {
            relation: "functions",
            fields: vec!["module", "name"],
            filters: vec![
                ("module == $mod".to_string(), "$mod".to_string()),
                ("arity > $min_arity".to_string(), "$min_arity".to_string()),
            ],
            limit: None,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = query.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("module == $mod"));
        assert!(compiled.contains("arity > $min_arity"));
    }

    #[test]
    fn test_select_query_no_limit() {
        let query = SelectQuery {
            relation: "calls",
            fields: vec!["caller", "callee"],
            filters: vec![],
            limit: None,
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = query.compile(backend.as_ref()).unwrap();

        assert!(!compiled.contains(":limit"));
    }

    #[test]
    fn test_select_query_multiple_fields() {
        let query = SelectQuery {
            relation: "locations",
            fields: vec!["file", "line", "column"],
            filters: vec![],
            limit: Some(5),
        };

        let backend = open_mem_db(true).unwrap();
        let compiled = query.compile(backend.as_ref()).unwrap();

        assert!(compiled.contains("file, line, column"));
        assert!(compiled.contains(":limit 5"));
    }

    #[test]
    fn test_recursive_query_cozo_not_implemented() {
        let query = RecursiveQuery {
            start_relation: "calls",
            recursive_step: "call_edge",
            depth_limit: Some(5),
            return_fields: vec!["caller", "callee"],
        };

        let backend = open_mem_db(true).unwrap();
        let result = query.compile(backend.as_ref());

        // Should fail with informative error until Ticket #47
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.err());
        assert!(err_msg.contains("Ticket #47"));
    }

    #[test]
    fn test_select_query_parameters_empty() {
        let query = SelectQuery {
            relation: "test",
            fields: vec!["x"],
            filters: vec![],
            limit: None,
        };

        let params = query.parameters();
        assert_eq!(params.len(), 0);
    }
}

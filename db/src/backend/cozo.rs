//! CozoDB backend implementation.
//!
//! This module provides the CozoDB-specific implementation of the Database trait,
//! wrapping the existing `DbInstance` and integrating with the generic trait interface.

use super::{Database, QueryParams, QueryResult, Row, Value, ValueType};
use cozo::{DataValue, DbInstance, NamedRows, Num, ScriptMutability};
use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

/// CozoDB database wrapper implementing the generic Database trait.
pub struct CozoDatabase {
    inner: DbInstance,
}

impl CozoDatabase {
    /// Opens a CozoDB database at the specified path.
    ///
    /// Creates a SQLite-backed CozoDB instance at the given filesystem path.
    pub fn open(path: &Path) -> Result<Self, Box<dyn Error>> {
        let inner = DbInstance::new("sqlite", path, "").map_err(|e| {
            format!("CozoDB open failed: {:?}", e)
        })?;
        Ok(Self { inner })
    }

    /// Opens an in-memory CozoDB database for testing.
    ///
    /// This is only available when building tests or with the `test-utils` feature.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn open_mem() -> Self {
        let inner = DbInstance::new("mem", "", "").expect("Failed to create in-memory DB");
        Self { inner }
    }

    /// Returns a reference to the inner DbInstance.
    ///
    /// This is mainly used for testing and for code that needs to work with DbInstance directly.
    #[cfg(all(any(test, feature = "test-utils"), feature = "backend-cozo"))]
    pub fn inner_ref(&self) -> &DbInstance {
        &self.inner
    }
}

impl Database for CozoDatabase {
    fn execute_query(
        &self,
        query: &str,
        params: QueryParams,
    ) -> Result<Box<dyn QueryResult>, Box<dyn Error>> {
        // Convert QueryParams to CozoDB format
        let cozo_params = convert_query_params(params);

        let rows = self
            .inner
            .run_script(query, cozo_params, ScriptMutability::Mutable)
            .map_err(|e| format!("Query failed: {:?}", e))?;

        Ok(Box::new(CozoQueryResult::new(rows)))
    }

    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self as &(dyn std::any::Any + Send + Sync)
    }
}

/// Converts QueryParams to CozoDB's BTreeMap<String, DataValue> format.
fn convert_query_params(params: QueryParams) -> BTreeMap<String, DataValue> {
    params
        .params()
        .iter()
        .map(|(k, v)| {
            let data_value = match v {
                ValueType::Str(s) => DataValue::Str(s.clone().into()),
                ValueType::Int(i) => DataValue::Num(Num::Int(*i)),
                ValueType::Float(f) => DataValue::Num(Num::Float(*f)),
                ValueType::Bool(b) => DataValue::Bool(*b),
                ValueType::StrArray(arr) => DataValue::List(
                    arr.iter()
                        .map(|s| DataValue::Str(s.clone().into()))
                        .collect(),
                ),
            };
            (k.clone(), data_value)
        })
        .collect()
}

/// Query result wrapper implementing the generic QueryResult trait.
pub struct CozoQueryResult {
    headers: Vec<String>,
    rows: Vec<Box<dyn Row>>,
}

impl CozoQueryResult {
    /// Creates a new query result from CozoDB's NamedRows.
    pub fn new(named_rows: NamedRows) -> Self {
        let headers = named_rows.headers;
        let rows: Vec<Box<dyn Row>> = named_rows
            .rows
            .into_iter()
            .map(|row_values| Box::new(CozoRow::new(row_values)) as Box<dyn Row>)
            .collect();

        Self { headers, rows }
    }
}

impl QueryResult for CozoQueryResult {
    fn headers(&self) -> &[String] {
        &self.headers
    }

    fn rows(&self) -> &[Box<dyn Row>] {
        &self.rows
    }

    fn into_rows(self: Box<Self>) -> Vec<Box<dyn Row>> {
        self.rows
    }
}

/// Row wrapper implementing the generic Row trait.
pub struct CozoRow {
    values: Vec<DataValue>,
}

impl CozoRow {
    /// Creates a new row from CozoDB DataValues.
    fn new(values: Vec<DataValue>) -> Self {
        Self { values }
    }
}

impl Row for CozoRow {
    fn get(&self, index: usize) -> Option<&dyn Value> {
        self.values.get(index).map(|v| v as &dyn Value)
    }

    fn len(&self) -> usize {
        self.values.len()
    }
}

/// Implements the Value trait for CozoDB's DataValue type.
impl Value for DataValue {
    fn as_str(&self) -> Option<&str> {
        match self {
            DataValue::Str(s) => Some(s),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            DataValue::Num(Num::Int(i)) => Some(*i),
            DataValue::Num(Num::Float(f)) => Some(*f as i64),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            DataValue::Num(Num::Int(i)) => Some(*i as f64),
            DataValue::Num(Num::Float(f)) => Some(*f),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            DataValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<Vec<&dyn Value>> {
        None // CozoDB doesn't need array extraction for graph traversal
    }

    fn as_thing_id(&self) -> Option<&dyn Value> {
        None // CozoDB doesn't have Thing type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_mem() {
        let _db = CozoDatabase::open_mem();
        // If we got here, open succeeded
    }

    #[test]
    fn test_execute_query_no_params() {
        let db = CozoDatabase::open_mem();
        let result = db
            .execute_query("?[x] := x = 1", QueryParams::new())
            .expect("Query should succeed");

        assert_eq!(result.headers(), &["x"]);
        assert_eq!(result.rows().len(), 1);
    }

    #[test]
    fn test_parameter_conversion() {
        let params = QueryParams::new()
            .with_str("name", "test")
            .with_int("count", 42)
            .with_float("value", 3.14)
            .with_bool("flag", true);

        let cozo_params = convert_query_params(params);

        assert_eq!(cozo_params.len(), 4);
        assert!(cozo_params.contains_key("name"));
        assert!(cozo_params.contains_key("count"));
        assert!(cozo_params.contains_key("value"));
        assert!(cozo_params.contains_key("flag"));
    }

    #[test]
    fn test_value_extraction() {
        let str_value = DataValue::Str("hello".to_string().into());
        assert_eq!(str_value.as_str(), Some("hello"));
        assert!(str_value.as_i64().is_none());

        let int_value = DataValue::Num(Num::Int(42));
        assert_eq!(int_value.as_i64(), Some(42));
        assert_eq!(int_value.as_f64(), Some(42.0));

        let float_value = DataValue::Num(Num::Float(3.14));
        assert_eq!(float_value.as_f64(), Some(3.14));

        let bool_value = DataValue::Bool(true);
        assert_eq!(bool_value.as_bool(), Some(true));
    }

    #[test]
    fn test_row_access() {
        let values = vec![
            DataValue::Str("test".to_string().into()),
            DataValue::Num(Num::Int(123)),
            DataValue::Bool(true),
        ];
        let row = CozoRow::new(values);

        assert_eq!(row.len(), 3);
        assert!(!row.is_empty());
        assert!(row.get(0).is_some());
        assert!(row.get(3).is_none());
    }

    #[test]
    fn test_query_result_structure() {
        let db = CozoDatabase::open_mem();
        let result = db
            .execute_query("?[x, y] := x = 1, y = 2", QueryParams::new())
            .expect("Query should succeed");

        assert_eq!(result.headers(), &["x", "y"]);
        assert_eq!(result.rows().len(), 1);

        let row = &result.rows()[0];
        assert_eq!(row.len(), 2);
        assert_eq!(row.get(0).and_then(|v| v.as_i64()), Some(1));
        assert_eq!(row.get(1).and_then(|v| v.as_i64()), Some(2));
    }

    #[test]
    fn test_query_with_parameters() {
        let db = CozoDatabase::open_mem();
        let params = QueryParams::new().with_int("val", 99);
        let result = db
            .execute_query("?[x] := x = $val", params)
            .expect("Query should succeed");

        assert_eq!(result.rows()[0].get(0).and_then(|v| v.as_i64()), Some(99));
    }
}

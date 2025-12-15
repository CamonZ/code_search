//! Type conversion utilities for PostgreSQL AGE backend.
//!
//! Handles conversion between:
//! - `DataValue` ↔ PostgreSQL types
//! - `Params` ↔ AGE parameter format
//! - AGE query results ↔ `QueryResult<DataValue>`

use std::error::Error;
use cozo::{DataValue, Num};
use serde_json::{Value as JsonValue};
use crate::db::backend::{Params, QueryResult};
use crate::db::schema::SchemaRelation;

/// Convert our Params type to format expected by apache-age crate.
///
/// Params is a BTreeMap<String, DataValue>. We need to convert it to the
/// parameter format expected by the apache-age client.execute method.
///
/// # Returns
/// A vector of PostgreSQL ToSql trait objects that can be used with execute()
pub fn convert_params_to_age(params: &Params) -> Result<Vec<String>, Box<dyn Error>> {
    // For now, we'll convert params to a vector of strings that can be
    // used as PostgreSQL parameters. The actual implementation depends on
    // how apache-age expects parameters.
    //
    // Most AGE queries use JSON parameter format like:
    // SELECT * FROM cypher('graph', $$ MATCH ... WHERE n.id = $id $$, jsonb_build_object('id', $1))

    let mut param_vec = Vec::new();

    for (key, value) in params.iter() {
        let param_str = match value {
            DataValue::Str(s) => s.to_string(),
            DataValue::Num(Num::Int(i)) => i.to_string(),
            DataValue::Num(Num::Float(f)) => f.to_string(),
            DataValue::Bool(b) => b.to_string(),
            DataValue::Null => "null".to_string(),
            _ => return Err(format!("Unsupported parameter type for key '{}': {:?}", key, value).into()),
        };
        param_vec.push(param_str);
    }

    Ok(param_vec)
}

/// Convert AGE query results (as JSON values) to our QueryResult<DataValue> type.
///
/// AGE's query_cypher method with serde_json::Value returns results as a Vec<Row>,
/// but we work with the JSON representation they contain.
#[allow(dead_code)]
pub fn convert_age_rows_to_query_result(
    json_rows: Vec<JsonValue>,
) -> Result<QueryResult<DataValue>, Box<dyn Error>> {
    if json_rows.is_empty() {
        return Ok(QueryResult {
            headers: vec![],
            rows: vec![],
        });
    }

    // Extract headers from first row (assuming object structure)
    let headers = if let Some(first_row) = json_rows.first() {
        if let JsonValue::Object(obj) = first_row {
            obj.keys().map(|k| k.to_string()).collect()
        } else {
            // If not an object, create generic column names
            vec!["col0".to_string()]
        }
    } else {
        vec![]
    };

    // Convert each row to Vec<DataValue>
    let mut result_rows = Vec::new();
    for row in json_rows {
        let mut row_values = Vec::new();

        match row {
            JsonValue::Object(obj) => {
                // Convert each field in the object to a DataValue
                for header in &headers {
                    if let Some(val) = obj.get(header) {
                        row_values.push(json_value_to_data_value(val)?);
                    } else {
                        row_values.push(DataValue::Null);
                    }
                }
            }
            JsonValue::Array(arr) => {
                // Convert array elements directly
                for val in arr {
                    row_values.push(json_value_to_data_value(&val)?);
                }
            }
            _ => {
                // Single value, wrap in DataValue
                row_values.push(json_value_to_data_value(&row)?);
            }
        }

        result_rows.push(row_values);
    }

    Ok(QueryResult {
        headers,
        rows: result_rows,
    })
}

/// Convert a JSON value to a DataValue.
fn json_value_to_data_value(val: &JsonValue) -> Result<DataValue, Box<dyn Error>> {
    match val {
        JsonValue::Null => Ok(DataValue::Null),
        JsonValue::Bool(b) => Ok(DataValue::Bool(*b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(DataValue::Num(Num::Int(i)))
            } else if let Some(f) = n.as_f64() {
                Ok(DataValue::Num(Num::Float(f)))
            } else {
                Err("Could not convert JSON number to DataValue".into())
            }
        }
        JsonValue::String(s) => Ok(DataValue::Str(s.clone().into())),
        JsonValue::Array(arr) => {
            let mut list = Vec::new();
            for item in arr {
                list.push(json_value_to_data_value(item)?);
            }
            Ok(DataValue::List(list.into()))
        }
        JsonValue::Object(obj) => {
            // For objects, convert to JSON string representation
            let json_str = serde_json::to_string(obj)?;
            Ok(DataValue::Str(json_str.into()))
        }
    }
}

/// Convert rows to JSON format for batch operations (UNWIND).
///
/// Takes a slice of rows (Vec<Vec<DataValue>>) and converts them to a JSON
/// array of objects suitable for UNWIND in Cypher queries.
///
/// # Example
/// ```ignore
/// [
///   { "project": "my_project", "name": "module1", "file": "module1.rs" },
///   { "project": "my_project", "name": "module2", "file": "module2.rs" }
/// ]
/// ```
pub fn convert_rows_to_json(
    relation: &SchemaRelation,
    rows: &[Vec<DataValue>],
) -> Result<String, Box<dyn Error>> {
    let mut json_rows = Vec::new();
    let fields = relation.all_fields().collect::<Vec<_>>();

    for row in rows {
        if row.len() != fields.len() {
            return Err(format!(
                "Row has {} values but relation {} expects {} fields",
                row.len(),
                relation.name,
                fields.len()
            )
            .into());
        }

        let mut obj = serde_json::Map::new();
        for (i, field) in fields.iter().enumerate() {
            let val = data_value_to_json(&row[i])?;
            obj.insert(field.name.to_string(), val);
        }

        json_rows.push(JsonValue::Object(obj));
    }

    let json_array = JsonValue::Array(json_rows);
    Ok(json_array.to_string())
}

/// Convert a DataValue to a JSON value.
fn data_value_to_json(val: &DataValue) -> Result<JsonValue, Box<dyn Error>> {
    match val {
        DataValue::Null => Ok(JsonValue::Null),
        DataValue::Bool(b) => Ok(JsonValue::Bool(*b)),
        DataValue::Num(Num::Int(i)) => Ok(JsonValue::Number(
            serde_json::Number::from(*i)
        )),
        DataValue::Num(Num::Float(f)) => {
            if f.is_finite() {
                Ok(JsonValue::Number(
                    serde_json::Number::from_f64(*f)
                        .ok_or("Could not convert float to JSON number")?
                ))
            } else {
                Ok(JsonValue::Null)
            }
        }
        DataValue::Str(s) => Ok(JsonValue::String(s.to_string())),
        DataValue::Bytes(_b) => {
            // Convert bytes to string representation (as hex)
            Ok(JsonValue::String("[bytes]".to_string()))
        }
        DataValue::List(l) => {
            let mut arr = Vec::new();
            for item in l.iter() {
                arr.push(data_value_to_json(item)?);
            }
            Ok(JsonValue::Array(arr))
        }
        _ => {
            // For other types (Set, Vec, Json, Uuid, etc.), convert to string
            Ok(JsonValue::String(format!("{:?}", val)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_convert_params_empty() {
        let params = Params::new();
        let result = convert_params_to_age(&params).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_convert_params_string() {
        let mut params = Params::new();
        params.insert("name".to_string(), DataValue::Str("test".into()));

        let result = convert_params_to_age(&params).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "test");
    }

    #[test]
    fn test_convert_params_int() {
        let mut params = Params::new();
        params.insert("id".to_string(), DataValue::Num(Num::Int(42)));

        let result = convert_params_to_age(&params).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "42");
    }

    #[test]
    fn test_json_value_to_data_value_null() {
        let val = json!(null);
        let result = json_value_to_data_value(&val).unwrap();
        assert!(matches!(result, DataValue::Null));
    }

    #[test]
    fn test_json_value_to_data_value_bool() {
        let val = json!(true);
        let result = json_value_to_data_value(&val).unwrap();
        assert!(matches!(result, DataValue::Bool(true)));
    }

    #[test]
    fn test_json_value_to_data_value_number_int() {
        let val = json!(42);
        let result = json_value_to_data_value(&val).unwrap();
        match result {
            DataValue::Num(Num::Int(i)) => assert_eq!(i, 42),
            _ => panic!("Expected Int"),
        }
    }

    #[test]
    fn test_json_value_to_data_value_string() {
        let val = json!("hello");
        let result = json_value_to_data_value(&val).unwrap();
        match result {
            DataValue::Str(s) => {
                let s_str: &str = &s;
                assert_eq!(s_str, "hello");
            }
            _ => panic!("Expected Str"),
        }
    }

    #[test]
    fn test_convert_age_rows_empty() {
        let rows = vec![];
        let result = convert_age_rows_to_query_result(rows).unwrap();
        assert_eq!(result.headers.len(), 0);
        assert_eq!(result.rows.len(), 0);
    }
}

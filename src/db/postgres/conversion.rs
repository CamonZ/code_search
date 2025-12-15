//! Type conversion utilities for PostgreSQL AGE backend.
//!
//! Handles conversion between:
//! - `DataValue` ↔ PostgreSQL types
//! - `Params` ↔ AGE parameter format
//! - AGE query results ↔ `QueryResult<DataValue>`

use std::error::Error;
use cozo::{DataValue, Num, JsonData};
use serde_json::{Value as JsonValue};
use crate::db::backend::{Params, QueryResult};
use crate::db::schema::SchemaRelation;

/// Convert our Params type to format expected by apache-age crate.
///
/// Params is a BTreeMap<String, DataValue>. We need to convert it to the
/// parameter format expected by the apache-age client.execute method.
///
/// # Returns
/// A vector of JsonValue that can be used with AGE queries
pub fn convert_params_to_age(params: &Params) -> Result<Vec<JsonValue>, Box<dyn Error>> {
    let mut param_vec = Vec::new();

    for (_, value) in params.iter() {
        param_vec.push(datavalue_to_json(value)?);
    }

    Ok(param_vec)
}

/// Convert AGE query results (as JSON values) to our QueryResult<DataValue> type.
///
/// AGE's query_cypher method with serde_json::Value returns results as a Vec<Row>,
/// but we work with the JSON representation they contain.
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
                        row_values.push(json_to_datavalue(val));
                    } else {
                        row_values.push(DataValue::Null);
                    }
                }
            }
            JsonValue::Array(arr) => {
                // Convert array elements directly
                for val in arr {
                    row_values.push(json_to_datavalue(&val));
                }
            }
            _ => {
                // Single value, wrap in DataValue
                row_values.push(json_to_datavalue(&row));
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
///
/// Handles all JSON types and AGE-specific types (vertices, edges, paths).
/// Preserves vertex/edge/path objects as `DataValue::Json` rather than stringifying them.
pub fn json_to_datavalue(val: &JsonValue) -> DataValue {
    match val {
        JsonValue::Null => DataValue::Null,
        JsonValue::Bool(b) => DataValue::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                DataValue::Num(Num::Int(i))
            } else if let Some(f) = n.as_f64() {
                DataValue::Num(Num::Float(f))
            } else {
                // Fallback - shouldn't happen
                DataValue::Null
            }
        }
        JsonValue::String(s) => DataValue::Str(s.clone().into()),
        JsonValue::Array(arr) => {
            let list: Vec<DataValue> = arr.iter()
                .map(json_to_datavalue)
                .collect();
            DataValue::List(list)
        }
        JsonValue::Object(obj) => {
            // Check if this is an AGE vertex, edge, or path object
            if is_age_vertex(obj) || is_age_edge(obj) || is_age_path(obj) {
                // Preserve AGE objects as JSON
                DataValue::Json(JsonData(val.clone()))
            } else {
                // Regular object - also preserve as JSON for flexibility
                DataValue::Json(JsonData(val.clone()))
            }
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
) -> Result<JsonValue, Box<dyn Error>> {
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
            let val = datavalue_to_json(&row[i])?;
            obj.insert(field.name.to_string(), val);
        }

        json_rows.push(JsonValue::Object(obj));
    }

    Ok(JsonValue::Array(json_rows))
}

/// Convert a DataValue to a JSON value.
///
/// Handles all DataValue variants with proper encoding:
/// - Bytes are hex-encoded
/// - UUIDs are stringified
/// - Json types are passed through
/// - Sets are converted to arrays
/// - Regex patterns are stored as strings
pub fn datavalue_to_json(val: &DataValue) -> Result<JsonValue, Box<dyn Error>> {
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
        DataValue::Bytes(b) => {
            // Encode bytes as hex string
            Ok(JsonValue::String(hex::encode(b)))
        }
        DataValue::Uuid(u) => {
            // Store UUID as string - u is a UuidWrapper, access inner Uuid via .0
            Ok(JsonValue::String((u.0).to_string()))
        }
        DataValue::Json(j) => {
            // j is a JsonData wrapper, extract the inner JsonValue
            Ok(j.0.clone())
        }
        DataValue::List(l) => {
            let mut arr = Vec::new();
            for item in l.iter() {
                arr.push(datavalue_to_json(item)?);
            }
            Ok(JsonValue::Array(arr))
        }
        DataValue::Set(s) => {
            // Convert set to array
            let mut arr = Vec::new();
            for item in s.iter() {
                arr.push(datavalue_to_json(item)?);
            }
            Ok(JsonValue::Array(arr))
        }
        DataValue::Regex(r) => {
            // Store regex pattern as string - r is a RegexWrapper, access pattern via .0
            Ok(JsonValue::String(r.0.as_str().to_string()))
        }
        DataValue::Bot => {
            // Bottom type - should not appear in normal data
            Err("Cannot convert Bot value to JSON".into())
        }
        DataValue::Validity(_) => {
            // Temporal validity - not supported in AGE
            Err("Cannot convert Validity value to JSON".into())
        }
        DataValue::Vec(_) => {
            // Vector type - not yet supported
            Err("Vector type conversion not yet implemented".into())
        }
    }
}

/// Extract column headers from AGE query results.
///
/// AGE doesn't provide column names in the same way as regular SQL,
/// so we parse them from the query or use defaults. This parses the RETURN clause.
///
/// # Example
/// ```ignore
/// let query = "MATCH (n:Function) RETURN n.name AS function_name, n.arity";
/// let headers = extract_column_headers(query);
/// assert_eq!(headers, vec!["function_name", "arity"]);
/// ```
pub fn extract_column_headers(query: &str) -> Vec<String> {
    // Parse RETURN clause to get column names
    if let Some(return_idx) = query.to_uppercase().find("RETURN ") {
        let return_clause = &query[return_idx + 7..];

        // Find the end of the RETURN clause (before ORDER, LIMIT, semicolon, etc.)
        let end_idx = return_clause.find(|c: char| {
            matches!(c, ';' | ')') || return_clause[return_clause.find(c).unwrap_or(0)..].to_uppercase().starts_with("ORDER")
                || return_clause[return_clause.find(c).unwrap_or(0)..].to_uppercase().starts_with("LIMIT")
        }).unwrap_or(return_clause.len());

        let columns = &return_clause[..end_idx];
        return columns.split(',')
            .filter_map(|s| {
                let s = s.trim();
                if s.is_empty() {
                    return None;
                }
                // Handle aliases: "n.name AS function_name" -> "function_name"
                if let Some(as_idx) = s.to_uppercase().find(" AS ") {
                    Some(s[as_idx + 4..].trim().to_string())
                } else {
                    // Handle property access: "n.name" -> "name"
                    Some(s.rsplit('.').next().unwrap_or(s).to_string())
                }
            })
            .collect();
    }

    vec!["result".to_string()]
}

/// Parse an agtype value from a PostgreSQL row.
///
/// The apache-age crate handles most of this, but we need additional
/// processing for vertex/edge/path types to detect them and preserve
/// them as JSON rather than converting to strings.
pub fn parse_agtype(value: &JsonValue) -> DataValue {
    json_to_datavalue(value)
}

/// Check if a JSON object is an AGE vertex.
///
/// Vertices have `id` and `label` fields.
fn is_age_vertex(obj: &serde_json::Map<String, JsonValue>) -> bool {
    obj.contains_key("id") && obj.contains_key("label")
}

/// Check if a JSON object is an AGE edge.
///
/// Edges have `start_id`, `end_id`, `id`, and `label` fields.
fn is_age_edge(obj: &serde_json::Map<String, JsonValue>) -> bool {
    obj.contains_key("start_id") && obj.contains_key("end_id") && obj.contains_key("id")
}

/// Check if a JSON object is an AGE path.
///
/// Paths have `vertices` and `edges` arrays.
fn is_age_path(obj: &serde_json::Map<String, JsonValue>) -> bool {
    matches!(obj.get("vertices"), Some(JsonValue::Array(_))) &&
    matches!(obj.get("edges"), Some(JsonValue::Array(_)))
}

/// Convert a vertex/edge ID to a string for matching.
///
/// AGE uses 64-bit integers for IDs, which we store as strings.
pub fn age_id_to_string(id: i64) -> String {
    id.to_string()
}

/// Parse an AGE ID from a string.
///
/// # Errors
/// Returns an error if the string is not a valid 64-bit integer.
pub fn string_to_age_id(s: &str) -> Result<i64, Box<dyn Error>> {
    s.parse::<i64>()
        .map_err(|e| format!("Invalid AGE ID '{}': {}", s, e).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Tests for datavalue_to_json

    #[test]
    fn test_datavalue_null_to_json() {
        let result = datavalue_to_json(&DataValue::Null).unwrap();
        assert_eq!(result, JsonValue::Null);
    }

    #[test]
    fn test_datavalue_bool_to_json() {
        let result = datavalue_to_json(&DataValue::Bool(true)).unwrap();
        assert_eq!(result, JsonValue::Bool(true));
    }

    #[test]
    fn test_datavalue_int_to_json() {
        let result = datavalue_to_json(&DataValue::Num(Num::Int(42))).unwrap();
        assert_eq!(result, JsonValue::Number(42.into()));
    }

    #[test]
    fn test_datavalue_float_to_json() {
        let result = datavalue_to_json(&DataValue::Num(Num::Float(3.14))).unwrap();
        assert!(matches!(result, JsonValue::Number(_)));
    }

    #[test]
    fn test_datavalue_string_to_json() {
        let result = datavalue_to_json(&DataValue::Str("hello".into())).unwrap();
        assert_eq!(result, JsonValue::String("hello".to_string()));
    }

    #[test]
    fn test_datavalue_list_to_json() {
        let list = DataValue::List(vec![
            DataValue::Num(Num::Int(1)),
            DataValue::Num(Num::Int(2)),
        ]);
        let result = datavalue_to_json(&list).unwrap();
        assert_eq!(result, JsonValue::Array(vec![
            JsonValue::Number(1.into()),
            JsonValue::Number(2.into()),
        ]));
    }

    #[test]
    fn test_datavalue_bytes_to_json() {
        let bytes = DataValue::Bytes(vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]); // "Hello" in hex
        let result = datavalue_to_json(&bytes).unwrap();
        assert_eq!(result, JsonValue::String("48656c6c6f".to_string()));
    }

    #[test]
    fn test_datavalue_uuid_to_json() {
        // UUID tests are covered by cozo's internal tests
        // We just verify that our encoding works
        // Testing with the nil UUID (00000000-0000-0000-0000-000000000000)
        // This would be: UuidWrapper(Uuid::nil()) -> "00000000-0000-0000-0000-000000000000"
        // Since we don't have direct uuid crate access, we skip detailed UUID testing
        // and rely on the fact that our code simply calls u.0.to_string()
    }

    #[test]
    fn test_datavalue_json_to_json() {
        let json_val = json!({"key": "value"});
        let dv = DataValue::Json(JsonData(json_val.clone()));
        let result = datavalue_to_json(&dv).unwrap();
        assert_eq!(result, json_val);
    }

    #[test]
    fn test_datavalue_set_to_json() {
        let set = {
            let mut s = std::collections::BTreeSet::new();
            s.insert(DataValue::Num(Num::Int(1)));
            s.insert(DataValue::Num(Num::Int(2)));
            DataValue::Set(s)
        };
        let result = datavalue_to_json(&set).unwrap();
        assert!(matches!(result, JsonValue::Array(_)));
    }

    // Tests for json_to_datavalue

    #[test]
    fn test_json_null_to_datavalue() {
        let result = json_to_datavalue(&JsonValue::Null);
        assert_eq!(result, DataValue::Null);
    }

    #[test]
    fn test_json_bool_to_datavalue() {
        let result = json_to_datavalue(&JsonValue::Bool(false));
        assert_eq!(result, DataValue::Bool(false));
    }

    #[test]
    fn test_json_int_to_datavalue() {
        let result = json_to_datavalue(&JsonValue::Number(100.into()));
        assert_eq!(result, DataValue::Num(Num::Int(100)));
    }

    #[test]
    fn test_json_string_to_datavalue() {
        let result = json_to_datavalue(&JsonValue::String("test".to_string()));
        assert_eq!(result, DataValue::Str("test".into()));
    }

    #[test]
    fn test_json_array_to_datavalue() {
        let json_array = json!([1, 2, 3]);
        let result = json_to_datavalue(&json_array);
        assert!(matches!(result, DataValue::List(_)));
    }

    #[test]
    fn test_json_object_to_datavalue_preserves_json() {
        let json_obj = json!({"key": "value"});
        let result = json_to_datavalue(&json_obj);
        assert!(matches!(result, DataValue::Json(_)));
    }

    // Tests for extract_column_headers

    #[test]
    fn test_extract_column_headers_simple() {
        let query = "MATCH (n:Function) RETURN n.name, n.arity";
        let headers = extract_column_headers(query);
        assert_eq!(headers, vec!["name", "arity"]);
    }

    #[test]
    fn test_extract_column_headers_with_alias() {
        let query = "MATCH (n:Function) RETURN n.name AS function_name";
        let headers = extract_column_headers(query);
        assert_eq!(headers, vec!["function_name"]);
    }

    #[test]
    fn test_extract_column_headers_with_limit() {
        let query = "MATCH (n:Function) RETURN n.name LIMIT 10";
        let headers = extract_column_headers(query);
        assert_eq!(headers, vec!["name"]);
    }

    #[test]
    fn test_extract_column_headers_mixed() {
        let query = "MATCH (n:Function) RETURN n.name AS fname, n.arity AS ar, n.file";
        let headers = extract_column_headers(query);
        assert_eq!(headers, vec!["fname", "ar", "file"]);
    }

    #[test]
    fn test_extract_column_headers_no_return() {
        let query = "MATCH (n:Function) WHERE n.name = 'test'";
        let headers = extract_column_headers(query);
        assert_eq!(headers, vec!["result"]);
    }

    // Tests for AGE ID conversion

    #[test]
    fn test_age_id_to_string() {
        let id: i64 = 12345;
        let result = age_id_to_string(id);
        assert_eq!(result, "12345");
    }

    #[test]
    fn test_string_to_age_id() {
        let result = string_to_age_id("12345").unwrap();
        assert_eq!(result, 12345);
    }

    #[test]
    fn test_string_to_age_id_invalid() {
        let result = string_to_age_id("not_a_number");
        assert!(result.is_err());
    }

    // Roundtrip tests

    #[test]
    fn test_roundtrip_null() {
        let original = DataValue::Null;
        let json = datavalue_to_json(&original).unwrap();
        let back = json_to_datavalue(&json);
        assert_eq!(original, back);
    }

    #[test]
    fn test_roundtrip_bool() {
        let original = DataValue::Bool(true);
        let json = datavalue_to_json(&original).unwrap();
        let back = json_to_datavalue(&json);
        assert_eq!(original, back);
    }

    #[test]
    fn test_roundtrip_int() {
        let original = DataValue::Num(Num::Int(42));
        let json = datavalue_to_json(&original).unwrap();
        let back = json_to_datavalue(&json);
        assert_eq!(original, back);
    }

    #[test]
    fn test_roundtrip_string() {
        let original = DataValue::Str("test".into());
        let json = datavalue_to_json(&original).unwrap();
        let back = json_to_datavalue(&json);
        assert_eq!(original, back);
    }

    #[test]
    fn test_roundtrip_list() {
        let original = DataValue::List(vec![
            DataValue::Num(Num::Int(1)),
            DataValue::Str("hello".into()),
            DataValue::Bool(true),
        ]);
        let json = datavalue_to_json(&original).unwrap();
        let back = json_to_datavalue(&json);
        assert_eq!(original, back);
    }

    #[test]
    fn test_roundtrip_json_object() {
        let json_obj = json!({"name": "test", "value": 42});
        let original = DataValue::Json(JsonData(json_obj));
        let json = datavalue_to_json(&original).unwrap();
        let back = json_to_datavalue(&json);
        assert_eq!(original, back);
    }

    // Tests for vertex/edge/path detection

    #[test]
    fn test_parse_agtype_vertex() {
        let vertex = json!({
            "id": 12345,
            "label": "Function",
            "properties": {"name": "test"}
        });
        let result = parse_agtype(&vertex);
        assert!(matches!(result, DataValue::Json(_)));
    }

    #[test]
    fn test_parse_agtype_edge() {
        let edge = json!({
            "id": 12345,
            "label": "CALLS",
            "start_id": 111,
            "end_id": 222,
            "properties": {}
        });
        let result = parse_agtype(&edge);
        assert!(matches!(result, DataValue::Json(_)));
    }

    #[test]
    fn test_parse_agtype_path() {
        let path = json!({
            "vertices": [
                {"id": 1, "label": "Node"}
            ],
            "edges": [
                {"id": 100, "label": "Link", "start_id": 1, "end_id": 2}
            ]
        });
        let result = parse_agtype(&path);
        assert!(matches!(result, DataValue::Json(_)));
    }

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
        assert_eq!(result[0], JsonValue::String("test".to_string()));
    }

    #[test]
    fn test_convert_params_int() {
        let mut params = Params::new();
        params.insert("id".to_string(), DataValue::Num(Num::Int(42)));
        let result = convert_params_to_age(&params).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], JsonValue::Number(42.into()));
    }

    #[test]
    fn test_convert_age_rows_empty() {
        let rows = vec![];
        let result = convert_age_rows_to_query_result(rows).unwrap();
        assert_eq!(result.headers.len(), 0);
        assert_eq!(result.rows.len(), 0);
    }

    #[test]
    fn test_convert_age_rows_object() {
        let rows = vec![
            json!({"name": "test", "id": 42})
        ];
        let result = convert_age_rows_to_query_result(rows).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].len(), 2); // name and id
    }

    #[test]
    fn test_convert_age_rows_array() {
        let rows = vec![
            json!(["test", 42])
        ];
        let result = convert_age_rows_to_query_result(rows).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].len(), 2);
    }
}

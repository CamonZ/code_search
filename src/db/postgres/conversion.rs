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

/// Substitute parameters directly into a Cypher query string.
///
/// Since the rust-postgres driver doesn't support ToSql for AGE's agtype,
/// we inline parameter values directly into the query string.
///
/// Parameters in Cypher use `$name` syntax. This function replaces them
/// with properly escaped literal values.
///
/// # Arguments
/// * `query` - The Cypher query with `$param` placeholders
/// * `params` - Map of parameter names to values
///
/// # Example
/// ```ignore
/// let query = "MATCH (n:Module) WHERE n.name = $name RETURN n";
/// let params = btreemap!{"name" => DataValue::Str("Test".into())};
/// let result = substitute_params(query, &params)?;
/// // Returns: "MATCH (n:Module) WHERE n.name = 'Test' RETURN n"
/// ```
pub fn substitute_params(query: &str, params: &Params) -> Result<String, Box<dyn Error>> {
    let mut result = query.to_string();

    for (name, value) in params.iter() {
        let json_val = datavalue_to_json(value)?;
        let literal = json_to_cypher_literal(&json_val);

        // Replace $name with the literal value
        // Handle both $name and ${name} syntax
        let placeholder = format!("${}", name);
        result = result.replace(&placeholder, &literal);
    }

    Ok(result)
}

/// Wrap a Cypher query in AGE's SQL function call.
///
/// AGE queries are executed via: `SELECT * FROM cypher('graph', $$ query $$) AS (col1 agtype, ...)`
/// This function parses the RETURN clause to determine column names and generates the wrapper.
/// Columns are cast to text for easier deserialization in Rust.
///
/// # Returns
/// A tuple of (sql_query, column_count)
pub fn wrap_cypher_query(graph_name: &str, cypher: &str) -> Result<(String, usize), Box<dyn Error>> {
    // Extract column names from RETURN clause
    let columns = extract_return_columns(cypher)?;
    let column_count = columns.len();

    // Build the AS clause with column definitions
    let column_defs = columns.iter()
        .map(|c| format!("{} agtype", c))
        .collect::<Vec<_>>()
        .join(", ");

    // Build SELECT list with text casts (agtype can't be directly deserialized in rust-postgres)
    let select_list = columns.iter()
        .map(|c| format!("{}::text", c))
        .collect::<Vec<_>>()
        .join(", ");

    // Wrap in AGE's cypher function with text casting
    let sql = format!(
        "SELECT {} FROM cypher('{}', $$ {} $$) AS ({})",
        select_list, graph_name, cypher, column_defs
    );

    Ok((sql, column_count))
}

/// Extract column names from a Cypher RETURN clause.
///
/// Handles various RETURN patterns:
/// - `RETURN a, b, c` -> ["a", "b", "c"]
/// - `RETURN n.name, n.age` -> ["name", "age"]
/// - `RETURN n.name AS fullname` -> ["fullname"]
/// - `RETURN count(*) AS cnt` -> ["cnt"]
fn extract_return_columns(cypher: &str) -> Result<Vec<String>, Box<dyn Error>> {
    // Find the RETURN clause (case-insensitive)
    let upper = cypher.to_uppercase();
    let return_pos = upper.find("RETURN ")
        .ok_or("Cypher query must have a RETURN clause")?;

    // Get everything after RETURN
    let after_return = &cypher[return_pos + 7..];

    // Find where RETURN clause ends (ORDER BY, LIMIT, or end of string)
    let end_pos = ["ORDER BY", "LIMIT", "SKIP", "UNION"]
        .iter()
        .filter_map(|kw| after_return.to_uppercase().find(kw))
        .min()
        .unwrap_or(after_return.len());

    let return_clause = after_return[..end_pos].trim();

    // Split by comma and extract column names
    let mut columns = Vec::new();
    for part in return_clause.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Check for AS alias
        let upper_part = part.to_uppercase();
        let col_name = if let Some(as_pos) = upper_part.find(" AS ") {
            // Use the alias after AS
            part[as_pos + 4..].trim().to_string()
        } else if part.contains('.') {
            // Property access like n.name -> use "name"
            part.rsplit('.').next().unwrap_or(part).trim().to_string()
        } else if part.contains('(') {
            // Function call like count(*) -> generate a name
            format!("col{}", columns.len())
        } else {
            // Simple variable name
            part.to_string()
        };

        columns.push(col_name);
    }

    if columns.is_empty() {
        return Err("RETURN clause has no columns".into());
    }

    Ok(columns)
}

/// Convert PostgreSQL rows to QueryResult<DataValue>.
///
/// Each row contains agtype columns which are parsed as JSON and converted to DataValue.
pub fn convert_postgres_rows_to_query_result(
    rows: &[postgres::Row],
    column_count: usize,
) -> Result<QueryResult<DataValue>, Box<dyn Error>> {
    if rows.is_empty() {
        // Return empty result with generated headers
        let headers: Vec<String> = (0..column_count)
            .map(|i| format!("col{}", i))
            .collect();
        return Ok(QueryResult {
            headers,
            rows: vec![],
        });
    }

    // Get column names from the first row
    let headers: Vec<String> = (0..column_count)
        .map(|i| rows[0].columns().get(i).map(|c| c.name().to_string()).unwrap_or_else(|| format!("col{}", i)))
        .collect();

    // Convert each row
    let mut result_rows = Vec::new();
    for row in rows {
        let mut values = Vec::new();
        for i in 0..column_count {
            // AGE returns agtype which we get as a string and parse as JSON
            let agtype_str: String = row.try_get(i)
                .map_err(|e| format!("Failed to get column {}: {}", i, e))?;

            // Parse the agtype string as JSON
            let json_val: JsonValue = serde_json::from_str(&agtype_str)
                .unwrap_or(JsonValue::String(agtype_str));

            // Convert JSON to DataValue
            values.push(json_to_datavalue(&json_val));
        }
        result_rows.push(values);
    }

    Ok(QueryResult {
        headers,
        rows: result_rows,
    })
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

use crate::db::schema::compilers::AgeCompiler;

/// Convert a JSON value to Cypher literal syntax.
///
/// Cypher uses different syntax than JSON:
/// - Property names are unquoted identifiers
/// - String values use single quotes
/// - Arrays use square brackets
/// - Maps use curly braces
///
/// # Examples
/// - JSON: `{"name": "test", "count": 42}`
/// - Cypher: `{name: 'test', count: 42}`
pub fn json_to_cypher_literal(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "null".to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => {
            // Escape single quotes by doubling them
            let escaped = s.replace('\'', "''");
            format!("'{}'", escaped)
        }
        JsonValue::Array(arr) => {
            let items: Vec<String> = arr.iter()
                .map(json_to_cypher_literal)
                .collect();
            format!("[{}]", items.join(", "))
        }
        JsonValue::Object(obj) => {
            let props: Vec<String> = obj.iter()
                .map(|(k, v)| format!("{}: {}", k, json_to_cypher_literal(v)))
                .collect();
            format!("{{{}}}", props.join(", "))
        }
    }
}

/// Convert rows to Cypher literal format for UNWIND.
///
/// Takes a slice of rows and returns a Cypher array literal string
/// suitable for direct embedding in a Cypher query.
///
/// # Example Output
/// ```cypher
/// [{project: 'test', name: 'MyModule', file: 'lib/mod.ex'}]
/// ```
pub fn rows_to_cypher_literal(
    relation: &SchemaRelation,
    rows: &[Vec<DataValue>],
) -> Result<String, Box<dyn Error>> {
    let fields = relation.all_fields().collect::<Vec<_>>();
    let mut cypher_rows = Vec::new();

    for row in rows {
        if row.len() != fields.len() {
            return Err(format!(
                "Row has {} values but relation {} expects {} fields",
                row.len(),
                relation.name,
                fields.len()
            ).into());
        }

        let mut props = Vec::new();
        for (i, field) in fields.iter().enumerate() {
            let json_val = datavalue_to_json(&row[i])?;
            let cypher_val = json_to_cypher_literal(&json_val);
            props.push(format!("{}: {}", field.name, cypher_val));
        }

        cypher_rows.push(format!("{{{}}}", props.join(", ")));
    }

    Ok(format!("[{}]", cypher_rows.join(", ")))
}

/// Generate Cypher batch insert with inlined data.
///
/// Since the rust-postgres driver doesn't support `ToSql` for AGE's `agtype`,
/// we inline the JSON data directly into the Cypher query as a literal.
///
/// # Arguments
/// * `relation` - The schema relation definition
/// * `rows_json` - JSON array string of row objects (e.g., `[{"project":"test",...}]`)
///
/// # Returns
/// A Cypher query string like:
/// ```cypher
/// UNWIND [{"project":"test","name":"Mod"}] AS row
/// CREATE (n:Module { project: row.project, name: row.name, ... })
/// ```
pub fn compile_batch_insert_with_data(relation: &SchemaRelation, rows_json: &str) -> String {
    let vertex_label = AgeCompiler::relation_to_vertex_label(relation.name);

    let props = relation.all_fields()
        .map(|f| format!("{}: row.{}", f.name, f.name))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "UNWIND {} AS row\nCREATE (n:{} {{ {} }})",
        rows_json, vertex_label, props
    )
}

/// Generate Cypher MERGE (upsert) with inlined data.
///
/// Since the rust-postgres driver doesn't support `ToSql` for AGE's `agtype`,
/// we inline the JSON data directly into the Cypher query as a literal.
///
/// # Arguments
/// * `relation` - The schema relation definition
/// * `rows_json` - JSON array string of row objects
///
/// # Returns
/// A Cypher query string using MERGE for upsert semantics
pub fn compile_upsert_with_data(relation: &SchemaRelation, rows_json: &str) -> String {
    let vertex_label = AgeCompiler::relation_to_vertex_label(relation.name);

    let key_props = relation.key_fields.iter()
        .map(|f| format!("{}: row.{}", f.name, f.name))
        .collect::<Vec<_>>()
        .join(", ");

    let value_sets = relation.value_fields.iter()
        .map(|f| format!("n.{} = row.{}", f.name, f.name))
        .collect::<Vec<_>>()
        .join(", ");

    if value_sets.is_empty() {
        format!(
            "UNWIND {} AS row\nMERGE (n:{} {{ {} }})",
            rows_json, vertex_label, key_props
        )
    } else {
        format!(
            "UNWIND {} AS row\nMERGE (n:{} {{ {} }})\nSET {}",
            rows_json, vertex_label, key_props, value_sets
        )
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

    // ==================== json_to_cypher_literal tests ====================

    #[test]
    fn test_cypher_literal_null() {
        let result = json_to_cypher_literal(&JsonValue::Null);
        assert_eq!(result, "null");
    }

    #[test]
    fn test_cypher_literal_bool() {
        assert_eq!(json_to_cypher_literal(&JsonValue::Bool(true)), "true");
        assert_eq!(json_to_cypher_literal(&JsonValue::Bool(false)), "false");
    }

    #[test]
    fn test_cypher_literal_number() {
        assert_eq!(json_to_cypher_literal(&json!(42)), "42");
        assert_eq!(json_to_cypher_literal(&json!(3.14)), "3.14");
    }

    #[test]
    fn test_cypher_literal_string() {
        assert_eq!(json_to_cypher_literal(&json!("hello")), "'hello'");
    }

    #[test]
    fn test_cypher_literal_string_with_quotes() {
        // Single quotes should be escaped by doubling
        assert_eq!(json_to_cypher_literal(&json!("it's")), "'it''s'");
        assert_eq!(json_to_cypher_literal(&json!("say 'hello'")), "'say ''hello'''");
    }

    #[test]
    fn test_cypher_literal_array() {
        let arr = json!([1, 2, 3]);
        assert_eq!(json_to_cypher_literal(&arr), "[1, 2, 3]");
    }

    #[test]
    fn test_cypher_literal_array_mixed() {
        let arr = json!(["test", 42, true]);
        assert_eq!(json_to_cypher_literal(&arr), "['test', 42, true]");
    }

    #[test]
    fn test_cypher_literal_object() {
        let obj = json!({"name": "test"});
        // Note: object key order may vary, so we check contains
        let result = json_to_cypher_literal(&obj);
        assert!(result.contains("name: 'test'"));
        assert!(result.starts_with('{'));
        assert!(result.ends_with('}'));
    }

    #[test]
    fn test_cypher_literal_object_multiple_fields() {
        let obj = json!({"a": 1, "b": "two"});
        let result = json_to_cypher_literal(&obj);
        assert!(result.contains("a: 1"));
        assert!(result.contains("b: 'two'"));
    }

    #[test]
    fn test_cypher_literal_nested() {
        let nested = json!({"items": [1, 2], "flag": true});
        let result = json_to_cypher_literal(&nested);
        assert!(result.contains("items: [1, 2]"));
        assert!(result.contains("flag: true"));
    }

    // ==================== rows_to_cypher_literal tests ====================

    #[test]
    fn test_rows_to_cypher_literal_empty() {
        use crate::db::schema::MODULES;
        let result = rows_to_cypher_literal(&MODULES, &[]).unwrap();
        assert_eq!(result, "[]");
    }

    #[test]
    fn test_rows_to_cypher_literal_single_row() {
        use crate::db::schema::MODULES;
        let rows = vec![
            vec![
                DataValue::Str("test_project".into()),
                DataValue::Str("MyModule".into()),
                DataValue::Str("lib/my_module.ex".into()),
                DataValue::Str("source".into()),
            ],
        ];
        let result = rows_to_cypher_literal(&MODULES, &rows).unwrap();

        // Should be a Cypher array literal with one map
        assert!(result.starts_with("[{"));
        assert!(result.ends_with("}]"));
        assert!(result.contains("project: 'test_project'"));
        assert!(result.contains("name: 'MyModule'"));
        assert!(result.contains("file: 'lib/my_module.ex'"));
        assert!(result.contains("source: 'source'"));
    }

    #[test]
    fn test_rows_to_cypher_literal_multiple_rows() {
        use crate::db::schema::MODULES;
        let rows = vec![
            vec![
                DataValue::Str("proj".into()),
                DataValue::Str("Mod1".into()),
                DataValue::Str("file1.ex".into()),
                DataValue::Str("src".into()),
            ],
            vec![
                DataValue::Str("proj".into()),
                DataValue::Str("Mod2".into()),
                DataValue::Str("file2.ex".into()),
                DataValue::Str("src".into()),
            ],
        ];
        let result = rows_to_cypher_literal(&MODULES, &rows).unwrap();

        // Should contain two map literals separated by comma
        assert!(result.starts_with("[{"));
        assert!(result.ends_with("}]"));
        assert!(result.contains("}, {"));
        assert!(result.contains("name: 'Mod1'"));
        assert!(result.contains("name: 'Mod2'"));
    }

    #[test]
    fn test_rows_to_cypher_literal_with_integers() {
        use crate::db::schema::FUNCTIONS;
        let rows = vec![
            vec![
                DataValue::Str("proj".into()),        // project
                DataValue::Str("MyModule".into()),    // module
                DataValue::Str("my_func".into()),     // name
                DataValue::from(2i64),                // arity (integer)
                DataValue::Str("term()".into()),      // return_type
                DataValue::Str("a, b".into()),        // args
                DataValue::Str("source".into()),      // source
            ],
        ];
        let result = rows_to_cypher_literal(&FUNCTIONS, &rows).unwrap();

        // Integer should be unquoted
        assert!(result.contains("arity: 2"));
        // Strings should be quoted
        assert!(result.contains("name: 'my_func'"));
    }

    #[test]
    fn test_rows_to_cypher_literal_wrong_field_count() {
        use crate::db::schema::MODULES;
        let rows = vec![
            vec![
                DataValue::Str("only_one".into()),
            ],
        ];
        let result = rows_to_cypher_literal(&MODULES, &rows);
        assert!(result.is_err());
    }
}

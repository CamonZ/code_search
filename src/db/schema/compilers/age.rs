//! AGE (Apache Graph Extension for PostgreSQL) compiler.
//!
//! Generates schema documentation, index creation statements, and validation queries
//! from backend-agnostic schema definitions.
//!
//! Unlike Cozo, AGE schemas are implicit (vertices/edges created on insert), so
//! "compilation" means generating documentation, indexes, and validation queries.

use crate::db::schema::definition::SchemaRelation;
use std::collections::HashSet;

/// Compiler for generating AGE documentation and queries from schema definitions.
pub struct AgeCompiler;

impl AgeCompiler {
    /// Generate vertex definition (documentation) from a relation.
    ///
    /// Produces human-readable documentation showing vertex label and properties:
    /// ```text
    /// Vertex: :Module
    /// Properties:
    ///   project: String (default: None)
    ///   name: String (default: None)
    ///   file: String (default: Some(""))
    ///   source: String (default: Some("unknown"))
    /// ```
    pub fn vertex_definition(relation: &SchemaRelation) -> String {
        let vertex_label = Self::relation_to_vertex_label(relation.name);
        let all_fields = relation.key_fields.iter().chain(relation.value_fields.iter());

        let properties = all_fields
            .map(|f| {
                let default_str = match f.default {
                    Some(val) => format!("Some(\"{}\")", val),
                    None => "None".to_string(),
                };
                format!("  {}: {} (default: {})", f.name, f.data_type.age_type(), default_str)
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!("Vertex: :{}\nProperties:\n{}\n", vertex_label, properties)
    }

    /// Generate edge definition (documentation) from an edge specification.
    ///
    /// Produces human-readable documentation showing edge type and properties:
    /// ```text
    /// Edge: :CALLS Function -> Function
    /// Properties:
    ///   (none)
    /// ```
    pub fn edge_definition(edge_type: &str, from_label: &str, to_label: &str, properties: &[&str]) -> String {
        let props = if properties.is_empty() {
            "  (none)".to_string()
        } else {
            properties
                .iter()
                .map(|p| format!("  {}", p))
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            "Edge: :{} {} -> {}\nProperties:\n{}\n",
            edge_type, from_label, to_label, props
        )
    }

    /// Generate index creation statement for a relation.
    ///
    /// Produces valid Cypher CREATE INDEX statement:
    /// ```cypher
    /// CREATE INDEX IF NOT EXISTS idx_modules_keys ON Module(n.project, n.name)
    /// ```
    pub fn create_index(relation: &SchemaRelation) -> String {
        let vertex_label = Self::relation_to_vertex_label(relation.name);

        if relation.key_fields.is_empty() {
            return String::new();
        }

        let key_properties = relation
            .key_fields
            .iter()
            .map(|f| format!("n.{}", f.name))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "CREATE INDEX IF NOT EXISTS idx_{}_keys ON {}({})",
            relation.name, vertex_label, key_properties
        )
    }

    /// Generate initialization query to verify graph exists.
    ///
    /// Produces a query to check if a graph with the given name exists:
    /// ```sql
    /// SELECT * FROM ag_graph WHERE name = 'code_search'
    /// ```
    pub fn init_graph_query(graph_name: &str) -> String {
        format!("SELECT * FROM ag_graph WHERE name = '{}'", graph_name)
    }

    /// Generate schema validation query for a relation.
    ///
    /// Produces a Cypher query to count vertices of a given type:
    /// ```cypher
    /// MATCH (n:Module) RETURN count(*) as count
    /// ```
    pub fn validate_schema_query(relation: &SchemaRelation) -> String {
        let vertex_label = Self::relation_to_vertex_label(relation.name);
        format!("MATCH (n:{}) RETURN count(*) as count", vertex_label)
    }

    /// Map relation name to AGE vertex label.
    ///
    /// Converts plural/snake_case relation names to singular/PascalCase vertex labels:
    /// - modules -> Module
    /// - functions -> Function
    /// - calls -> Call
    /// - specs -> Spec
    /// - types -> Type
    /// - struct_fields -> StructField
    /// - function_locations -> FunctionLocation
    pub fn relation_to_vertex_label(relation_name: &str) -> String {
        let singular = match relation_name {
            "modules" => "Module",
            "functions" => "Function",
            "calls" => "Call",
            "specs" => "Spec",
            "types" => "Type",
            "struct_fields" => "StructField",
            "function_locations" => "FunctionLocation",
            _ => relation_name,
        };
        singular.to_string()
    }

    /// Generate Cypher batch insert using UNWIND.
    ///
    /// Produces output in the format:
    /// ```cypher
    /// UNWIND $rows AS row
    /// CREATE (n:Label { prop1: row.prop1, prop2: row.prop2, ... })
    /// ```
    ///
    /// The caller passes rows as the `$rows` parameter in a format like:
    /// `[{project: "proj", name: "MyApp"}, {project: "proj", name: "Other"}]`
    ///
    /// # Arguments
    /// * `relation` - The schema relation definition
    ///
    /// # Example
    /// ```ignore
    /// let script = AgeCompiler::compile_batch_insert(&MODULES);
    /// // Returns:
    /// // UNWIND $rows AS row
    /// // CREATE (n:Module { project: row.project, name: row.name, file: row.file, source: row.source })
    /// ```
    pub fn compile_batch_insert(relation: &SchemaRelation) -> String {
        let vertex_label = Self::relation_to_vertex_label(relation.name);

        let props = relation.all_fields()
            .map(|f| format!("{}: row.{}", f.name, f.name))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "UNWIND $rows AS row\nCREATE (n:{} {{ {} }})",
            vertex_label, props
        )
    }

    /// Generate Cypher DELETE statement by project.
    ///
    /// Produces output in the format:
    /// ```cypher
    /// MATCH (n:Label)
    /// WHERE n.project = $project
    /// DETACH DELETE n
    /// ```
    ///
    /// Uses DETACH DELETE to also remove any edges connected to the vertices.
    ///
    /// # Arguments
    /// * `relation` - The schema relation definition
    ///
    /// # Example
    /// ```ignore
    /// let script = AgeCompiler::compile_delete_by_project(&MODULES);
    /// // Returns:
    /// // MATCH (n:Module)
    /// // WHERE n.project = $project
    /// // DETACH DELETE n
    /// ```
    pub fn compile_delete_by_project(relation: &SchemaRelation) -> String {
        let vertex_label = Self::relation_to_vertex_label(relation.name);
        format!(
            "MATCH (n:{})\nWHERE n.project = $project\nDETACH DELETE n",
            vertex_label
        )
    }

    /// Generate Cypher MERGE statement for upsert.
    ///
    /// Produces output in the format:
    /// ```cypher
    /// UNWIND $rows AS row
    /// MERGE (n:Label { key1: row.key1, key2: row.key2 })
    /// SET n.val1 = row.val1, n.val2 = row.val2
    /// ```
    ///
    /// MERGE creates the vertex if it doesn't exist (based on key fields),
    /// then SET updates the value fields.
    ///
    /// # Arguments
    /// * `relation` - The schema relation definition
    ///
    /// # Example
    /// ```ignore
    /// let script = AgeCompiler::compile_upsert(&MODULES);
    /// // Returns:
    /// // UNWIND $rows AS row
    /// // MERGE (n:Module { project: row.project, name: row.name })
    /// // SET n.file = row.file, n.source = row.source
    /// ```
    pub fn compile_upsert(relation: &SchemaRelation) -> String {
        let vertex_label = Self::relation_to_vertex_label(relation.name);

        let key_props = relation.key_fields.iter()
            .map(|f| format!("{}: row.{}", f.name, f.name))
            .collect::<Vec<_>>()
            .join(", ");

        let value_sets = relation.value_fields.iter()
            .map(|f| format!("n.{} = row.{}", f.name, f.name))
            .collect::<Vec<_>>()
            .join(", ");

        if value_sets.is_empty() {
            // No value fields, just MERGE
            format!(
                "UNWIND $rows AS row\nMERGE (n:{} {{ {} }})",
                vertex_label, key_props
            )
        } else {
            format!(
                "UNWIND $rows AS row\nMERGE (n:{} {{ {} }})\nSET {}",
                vertex_label, key_props, value_sets
            )
        }
    }

    /// Generate comprehensive schema documentation.
    ///
    /// Produces full documentation including vertices and edges sections.
    /// Each vertex documents properties with types and defaults.
    /// Each edge documents source vertex, target vertex, and any properties.
    pub fn schema_documentation(relations: &[&SchemaRelation]) -> String {
        let mut doc = String::from("AGE Schema Documentation\n=========================\n\n");

        // Vertices section
        doc.push_str("Vertices\n--------\n\n");
        for relation in relations {
            doc.push_str(&Self::vertex_definition(relation));
            doc.push('\n');
        }

        // Edges section (from relationships)
        doc.push_str("Edges\n-----\n\n");
        let mut edges_documented: HashSet<(&str, &str, &str)> = HashSet::new();

        for relation in relations {
            for rel in relation.relationships {
                let edge_key = (rel.edge_type, rel.name, rel.target);
                if edges_documented.insert(edge_key) {
                    let from = Self::relation_to_vertex_label(relation.name);
                    let to = Self::relation_to_vertex_label(rel.target);
                    doc.push_str(&Self::edge_definition(rel.edge_type, &from, &to, &[]));
                    doc.push('\n');
                }
            }
        }

        doc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::relations::*;

    #[test]
    fn test_vertex_definition_modules() {
        let def = AgeCompiler::vertex_definition(&MODULES);
        assert!(def.contains("Vertex: :Module"));
        assert!(def.contains("project: String"));
        assert!(def.contains("name: String"));
        assert!(def.contains("file: String"));
        assert!(def.contains("source: String"));
    }

    #[test]
    fn test_vertex_definition_functions() {
        let def = AgeCompiler::vertex_definition(&FUNCTIONS);
        assert!(def.contains("Vertex: :Function"));
        assert!(def.contains("project: String"));
        assert!(def.contains("module: String"));
        assert!(def.contains("name: String"));
        assert!(def.contains("arity: Integer"));
        assert!(def.contains("return_type: String"));
        assert!(def.contains("args: String"));
        assert!(def.contains("source: String"));
    }

    #[test]
    fn test_vertex_definition_calls() {
        let def = AgeCompiler::vertex_definition(&CALLS);
        assert!(def.contains("Vertex: :Call"));
        assert!(def.contains("caller_module: String"));
        assert!(def.contains("caller_function: String"));
        assert!(def.contains("callee_module: String"));
        assert!(def.contains("callee_function: String"));
        assert!(def.contains("callee_arity: Integer"));
    }

    #[test]
    fn test_vertex_definition_struct_fields() {
        let def = AgeCompiler::vertex_definition(&STRUCT_FIELDS);
        assert!(def.contains("Vertex: :StructField"));
        assert!(def.contains("project: String"));
        assert!(def.contains("module: String"));
        assert!(def.contains("field: String"));
    }

    #[test]
    fn test_vertex_definition_function_locations() {
        let def = AgeCompiler::vertex_definition(&FUNCTION_LOCATIONS);
        assert!(def.contains("Vertex: :FunctionLocation"));
        assert!(def.contains("project: String"));
        assert!(def.contains("complexity: Integer"));
    }

    #[test]
    fn test_vertex_definition_specs() {
        let def = AgeCompiler::vertex_definition(&SPECS);
        assert!(def.contains("Vertex: :Spec"));
        assert!(def.contains("project: String"));
        assert!(def.contains("module: String"));
        assert!(def.contains("name: String"));
        assert!(def.contains("arity: Integer"));
    }

    #[test]
    fn test_vertex_definition_types() {
        let def = AgeCompiler::vertex_definition(&TYPES);
        assert!(def.contains("Vertex: :Type"));
        assert!(def.contains("project: String"));
        assert!(def.contains("module: String"));
        assert!(def.contains("name: String"));
    }

    #[test]
    fn test_vertex_definition_includes_defaults() {
        let def = AgeCompiler::vertex_definition(&MODULES);
        // Should include default value information
        assert!(def.contains("default:"));
        assert!(def.contains("Some"));
        assert!(def.contains("None"));
    }

    #[test]
    fn test_edge_definition_basic() {
        let edge = AgeCompiler::edge_definition("CALLS", "Function", "Function", &[]);
        assert!(edge.contains("Edge: :CALLS"));
        assert!(edge.contains("Function -> Function"));
        assert!(edge.contains("(none)"));
    }

    #[test]
    fn test_edge_definition_with_properties() {
        let edge = AgeCompiler::edge_definition("LOCATED_IN", "Function", "Module", &["file_id", "line"]);
        assert!(edge.contains("Edge: :LOCATED_IN"));
        assert!(edge.contains("Function -> Module"));
        assert!(edge.contains("file_id"));
        assert!(edge.contains("line"));
    }

    #[test]
    fn test_create_index_modules() {
        let index = AgeCompiler::create_index(&MODULES);
        assert!(index.contains("CREATE INDEX"));
        assert!(index.contains("idx_modules_keys"));
        assert!(index.contains("Module"));
        assert!(index.contains("n.project"));
        assert!(index.contains("n.name"));
    }

    #[test]
    fn test_create_index_functions() {
        let index = AgeCompiler::create_index(&FUNCTIONS);
        assert!(index.contains("CREATE INDEX"));
        assert!(index.contains("idx_functions_keys"));
        assert!(index.contains("Function"));
        // Should include all 4 key fields
        assert!(index.contains("n.project"));
        assert!(index.contains("n.module"));
        assert!(index.contains("n.name"));
        assert!(index.contains("n.arity"));
    }

    #[test]
    fn test_create_index_calls() {
        let index = AgeCompiler::create_index(&CALLS);
        assert!(index.contains("CREATE INDEX"));
        assert!(index.contains("idx_calls_keys"));
        assert!(index.contains("Call"));
        // Should include all 9 key fields
        assert!(index.contains("n.project"));
        assert!(index.contains("n.caller_module"));
        assert!(index.contains("n.caller_function"));
        assert!(index.contains("n.callee_module"));
        assert!(index.contains("n.callee_function"));
        assert!(index.contains("n.callee_arity"));
        assert!(index.contains("n.file"));
        assert!(index.contains("n.line"));
        assert!(index.contains("n.column"));
    }

    #[test]
    fn test_create_index_struct_fields() {
        let index = AgeCompiler::create_index(&STRUCT_FIELDS);
        assert!(index.contains("CREATE INDEX"));
        assert!(index.contains("idx_struct_fields_keys"));
        assert!(index.contains("StructField"));
    }

    #[test]
    fn test_create_index_function_locations() {
        let index = AgeCompiler::create_index(&FUNCTION_LOCATIONS);
        assert!(index.contains("CREATE INDEX"));
        assert!(index.contains("idx_function_locations_keys"));
        assert!(index.contains("FunctionLocation"));
    }

    #[test]
    fn test_create_index_specs() {
        let index = AgeCompiler::create_index(&SPECS);
        assert!(index.contains("CREATE INDEX"));
        assert!(index.contains("idx_specs_keys"));
        assert!(index.contains("Spec"));
    }

    #[test]
    fn test_create_index_types() {
        let index = AgeCompiler::create_index(&TYPES);
        assert!(index.contains("CREATE INDEX"));
        assert!(index.contains("idx_types_keys"));
        assert!(index.contains("Type"));
    }

    #[test]
    fn test_init_graph_query() {
        let query = AgeCompiler::init_graph_query("code_search");
        assert!(query.contains("SELECT * FROM ag_graph"));
        assert!(query.contains("name = 'code_search'"));
    }

    #[test]
    fn test_init_graph_query_different_name() {
        let query = AgeCompiler::init_graph_query("my_graph");
        assert!(query.contains("name = 'my_graph'"));
    }

    #[test]
    fn test_validate_schema_query_modules() {
        let query = AgeCompiler::validate_schema_query(&MODULES);
        assert!(query.contains("MATCH"));
        assert!(query.contains("(n:Module)"));
        assert!(query.contains("count(*)"));
    }

    #[test]
    fn test_validate_schema_query_functions() {
        let query = AgeCompiler::validate_schema_query(&FUNCTIONS);
        assert!(query.contains("MATCH"));
        assert!(query.contains("(n:Function)"));
        assert!(query.contains("count(*)"));
    }

    #[test]
    fn test_validate_schema_query_calls() {
        let query = AgeCompiler::validate_schema_query(&CALLS);
        assert!(query.contains("MATCH"));
        assert!(query.contains("(n:Call)"));
        assert!(query.contains("count(*)"));
    }

    #[test]
    fn test_validate_schema_query_struct_fields() {
        let query = AgeCompiler::validate_schema_query(&STRUCT_FIELDS);
        assert!(query.contains("(n:StructField)"));
    }

    #[test]
    fn test_validate_schema_query_function_locations() {
        let query = AgeCompiler::validate_schema_query(&FUNCTION_LOCATIONS);
        assert!(query.contains("(n:FunctionLocation)"));
    }

    #[test]
    fn test_validate_schema_query_specs() {
        let query = AgeCompiler::validate_schema_query(&SPECS);
        assert!(query.contains("(n:Spec)"));
    }

    #[test]
    fn test_validate_schema_query_types() {
        let query = AgeCompiler::validate_schema_query(&TYPES);
        assert!(query.contains("(n:Type)"));
    }

    #[test]
    fn test_relation_to_vertex_label_all_standard() {
        assert_eq!(AgeCompiler::relation_to_vertex_label("modules"), "Module");
        assert_eq!(AgeCompiler::relation_to_vertex_label("functions"), "Function");
        assert_eq!(AgeCompiler::relation_to_vertex_label("calls"), "Call");
        assert_eq!(AgeCompiler::relation_to_vertex_label("specs"), "Spec");
        assert_eq!(AgeCompiler::relation_to_vertex_label("types"), "Type");
        assert_eq!(
            AgeCompiler::relation_to_vertex_label("struct_fields"),
            "StructField"
        );
        assert_eq!(
            AgeCompiler::relation_to_vertex_label("function_locations"),
            "FunctionLocation"
        );
    }

    #[test]
    fn test_relation_to_vertex_label_unknown() {
        // Unknown relations pass through unchanged
        assert_eq!(AgeCompiler::relation_to_vertex_label("unknown"), "unknown");
    }

    #[test]
    fn test_schema_documentation_contains_vertices() {
        let doc = AgeCompiler::schema_documentation(&ALL_RELATIONS);
        assert!(doc.contains("AGE Schema Documentation"));
        assert!(doc.contains("Vertices"));
        assert!(doc.contains("Vertex: :Module"));
        assert!(doc.contains("Vertex: :Function"));
        assert!(doc.contains("Vertex: :Call"));
        assert!(doc.contains("Vertex: :Spec"));
        assert!(doc.contains("Vertex: :Type"));
        assert!(doc.contains("Vertex: :StructField"));
        assert!(doc.contains("Vertex: :FunctionLocation"));
    }

    #[test]
    fn test_schema_documentation_contains_edges() {
        let doc = AgeCompiler::schema_documentation(&ALL_RELATIONS);
        assert!(doc.contains("Edges"));
        // Should document the relationships from the schema
        assert!(doc.contains("Edge: :LOCATED_IN"));
        assert!(doc.contains("Edge: :CALLS"));
    }

    #[test]
    fn test_schema_documentation_edge_direction() {
        let doc = AgeCompiler::schema_documentation(&ALL_RELATIONS);
        // LOCATED_IN goes from Function to Module
        assert!(doc.contains("Edge: :LOCATED_IN Function -> Module"));
        // CALLS goes from Call (the relation representing call edges) to Function
        // Note: In AGE, call edges are from functions to functions, but in our schema,
        // CALLS is a separate relation documenting each call, so edges go from Call to Function
        assert!(doc.contains("Edge: :CALLS Call -> Function"));
    }

    #[test]
    fn test_schema_documentation_all_relations() {
        let doc = AgeCompiler::schema_documentation(&ALL_RELATIONS);
        // Should document all 7 relations as vertices
        assert!(doc.contains("Vertex: :Module"));
        assert!(doc.contains("Vertex: :Function"));
        assert!(doc.contains("Vertex: :Call"));
        assert!(doc.contains("Vertex: :Spec"));
        assert!(doc.contains("Vertex: :Type"));
        assert!(doc.contains("Vertex: :StructField"));
        assert!(doc.contains("Vertex: :FunctionLocation"));
    }

    #[test]
    fn test_schema_documentation_includes_properties() {
        let doc = AgeCompiler::schema_documentation(&ALL_RELATIONS);
        // Spot check for properties in the documentation
        assert!(doc.contains("Properties:"));
        assert!(doc.contains("project: String"));
        assert!(doc.contains("name: String"));
    }

    #[test]
    fn test_create_index_all_relations() {
        for relation in ALL_RELATIONS {
            let index = AgeCompiler::create_index(relation);
            if !relation.key_fields.is_empty() {
                assert!(!index.is_empty(), "Should generate index for {}", relation.name);
                assert!(index.contains("CREATE INDEX"), "Should contain CREATE INDEX for {}", relation.name);
                assert!(
                    index.contains(&format!("idx_{}_keys", relation.name)),
                    "Should contain correct index name for {}",
                    relation.name
                );
            }
        }
    }

    #[test]
    fn test_validate_schema_query_all_relations() {
        for relation in ALL_RELATIONS {
            let query = AgeCompiler::validate_schema_query(relation);
            assert!(query.contains("MATCH"), "Should contain MATCH for {}", relation.name);
            assert!(query.contains("count(*)"), "Should contain count(*) for {}", relation.name);
            assert!(query.contains("RETURN"), "Should contain RETURN for {}", relation.name);
        }
    }

    #[test]
    fn test_vertex_definition_all_relations() {
        for relation in ALL_RELATIONS {
            let def = AgeCompiler::vertex_definition(relation);
            assert!(def.contains("Vertex: :"), "Should contain vertex label for {}", relation.name);
            assert!(def.contains("Properties:"), "Should contain properties section for {}", relation.name);
            // Should have at least one property
            assert!(def.contains("default:"), "Should contain default values for {}", relation.name);
        }
    }

    #[test]
    fn test_vertex_definition_field_count_modules() {
        let def = AgeCompiler::vertex_definition(&MODULES);
        // MODULES has 4 fields total: 2 key + 2 value
        let property_lines: Vec<&str> = def
            .lines()
            .filter(|line| line.starts_with("  ") && line.contains(":"))
            .collect();
        assert_eq!(property_lines.len(), 4, "Should have 4 properties for MODULES");
    }

    #[test]
    fn test_vertex_definition_field_count_functions() {
        let def = AgeCompiler::vertex_definition(&FUNCTIONS);
        // FUNCTIONS has 7 fields total: 4 key + 3 value
        let property_lines: Vec<&str> = def
            .lines()
            .filter(|line| line.starts_with("  ") && line.contains(":"))
            .collect();
        assert_eq!(property_lines.len(), 7, "Should have 7 properties for FUNCTIONS");
    }

    #[test]
    fn test_vertex_definition_field_count_calls() {
        let def = AgeCompiler::vertex_definition(&CALLS);
        // CALLS has 12 fields total: 9 key + 3 value
        let property_lines: Vec<&str> = def
            .lines()
            .filter(|line| line.starts_with("  ") && line.contains(":"))
            .collect();
        assert_eq!(property_lines.len(), 12, "Should have 12 properties for CALLS");
    }

    #[test]
    fn test_create_index_format() {
        let index = AgeCompiler::create_index(&MODULES);
        // Should be a single line (no newlines)
        assert!(!index.contains('\n'));
        // Should be valid CREATE INDEX syntax
        assert!(index.starts_with("CREATE INDEX"));
    }

    #[test]
    fn test_vertex_definition_format_header() {
        let def = AgeCompiler::vertex_definition(&MODULES);
        // First line should be Vertex: :Label
        let first_line = def.lines().next().unwrap();
        assert!(first_line.starts_with("Vertex: :"));
    }

    #[test]
    fn test_vertex_definition_format_properties_section() {
        let def = AgeCompiler::vertex_definition(&MODULES);
        // Should have Properties: as second line
        let second_line = def.lines().nth(1).unwrap();
        assert_eq!(second_line, "Properties:");
    }

    #[test]
    fn test_edge_definition_format() {
        let edge = AgeCompiler::edge_definition("CALLS", "Function", "Function", &[]);
        let first_line = edge.lines().next().unwrap();
        assert!(first_line.starts_with("Edge: :"));
        assert!(first_line.contains("->"));
    }

    #[test]
    fn test_init_graph_query_format() {
        let query = AgeCompiler::init_graph_query("test_graph");
        // Should be a single line SQL query
        assert!(!query.contains('\n'));
        assert!(query.starts_with("SELECT"));
    }

    #[test]
    fn test_validate_schema_query_format() {
        let query = AgeCompiler::validate_schema_query(&MODULES);
        // Should be a single line Cypher query
        assert!(!query.contains('\n'));
        assert!(query.starts_with("MATCH"));
        assert!(query.ends_with("count"));
    }

    // ==================== compile_batch_insert tests ====================

    #[test]
    fn test_compile_batch_insert_modules() {
        let script = AgeCompiler::compile_batch_insert(&MODULES);

        assert!(script.contains("UNWIND $rows AS row"));
        assert!(script.contains("CREATE (n:Module"));
        assert!(script.contains("project: row.project"));
        assert!(script.contains("name: row.name"));
        assert!(script.contains("file: row.file"));
        assert!(script.contains("source: row.source"));
    }

    #[test]
    fn test_compile_batch_insert_functions() {
        let script = AgeCompiler::compile_batch_insert(&FUNCTIONS);

        assert!(script.contains("UNWIND $rows AS row"));
        assert!(script.contains("CREATE (n:Function"));
        assert!(script.contains("module: row.module"));
        assert!(script.contains("arity: row.arity"));
    }

    #[test]
    fn test_compile_batch_insert_calls() {
        let script = AgeCompiler::compile_batch_insert(&CALLS);

        assert!(script.contains("CREATE (n:Call"));
        assert!(script.contains("caller_module: row.caller_module"));
        assert!(script.contains("callee_function: row.callee_function"));
    }

    #[test]
    fn test_compile_batch_insert_all_relations() {
        for relation in ALL_RELATIONS {
            let script = AgeCompiler::compile_batch_insert(relation);
            assert!(script.contains("UNWIND"), "Should contain UNWIND for {}", relation.name);
            assert!(script.contains("CREATE"), "Should contain CREATE for {}", relation.name);
        }
    }

    // ==================== compile_delete_by_project tests ====================

    #[test]
    fn test_compile_delete_by_project_modules() {
        let script = AgeCompiler::compile_delete_by_project(&MODULES);

        assert!(script.contains("MATCH (n:Module)"));
        assert!(script.contains("WHERE n.project = $project"));
        assert!(script.contains("DETACH DELETE n"));
    }

    #[test]
    fn test_compile_delete_by_project_functions() {
        let script = AgeCompiler::compile_delete_by_project(&FUNCTIONS);

        assert!(script.contains("MATCH (n:Function)"));
        assert!(script.contains("DETACH DELETE n"));
    }

    #[test]
    fn test_compile_delete_by_project_calls() {
        let script = AgeCompiler::compile_delete_by_project(&CALLS);

        assert!(script.contains("MATCH (n:Call)"));
        assert!(script.contains("DETACH DELETE n"));
    }

    #[test]
    fn test_compile_delete_by_project_all_relations() {
        for relation in ALL_RELATIONS {
            let script = AgeCompiler::compile_delete_by_project(relation);
            assert!(script.contains("MATCH"), "Should contain MATCH for {}", relation.name);
            assert!(script.contains("$project"), "Should contain $project for {}", relation.name);
            assert!(script.contains("DETACH DELETE"), "Should contain DETACH DELETE for {}", relation.name);
        }
    }

    // ==================== compile_upsert tests ====================

    #[test]
    fn test_compile_upsert_modules() {
        let script = AgeCompiler::compile_upsert(&MODULES);

        assert!(script.contains("UNWIND $rows AS row"));
        assert!(script.contains("MERGE (n:Module"));
        // Key fields in MERGE
        assert!(script.contains("project: row.project"));
        assert!(script.contains("name: row.name"));
        // Value fields in SET
        assert!(script.contains("SET n.file = row.file"));
        assert!(script.contains("n.source = row.source"));
    }

    #[test]
    fn test_compile_upsert_functions() {
        let script = AgeCompiler::compile_upsert(&FUNCTIONS);

        assert!(script.contains("MERGE (n:Function"));
        // Key fields: project, module, name, arity
        assert!(script.contains("arity: row.arity"));
        // Value fields in SET
        assert!(script.contains("n.return_type = row.return_type"));
    }

    #[test]
    fn test_compile_upsert_key_value_separation() {
        // MODULES has 2 key fields (project, name) and 2 value fields (file, source)
        let script = AgeCompiler::compile_upsert(&MODULES);

        // MERGE should only have key fields
        let merge_part = script.lines()
            .find(|l| l.contains("MERGE"))
            .unwrap();
        assert!(merge_part.contains("project: row.project"));
        assert!(merge_part.contains("name: row.name"));
        assert!(!merge_part.contains("file:"));
        assert!(!merge_part.contains("source:"));

        // SET should only have value fields
        let set_part = script.lines()
            .find(|l| l.contains("SET"))
            .unwrap();
        assert!(set_part.contains("n.file"));
        assert!(set_part.contains("n.source"));
    }

    #[test]
    fn test_compile_upsert_all_relations() {
        for relation in ALL_RELATIONS {
            let script = AgeCompiler::compile_upsert(relation);
            assert!(script.contains("UNWIND"), "Should contain UNWIND for {}", relation.name);
            assert!(script.contains("MERGE"), "Should contain MERGE for {}", relation.name);
        }
    }
}

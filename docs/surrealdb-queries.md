# SurrealDB Query Patterns

This document covers SurrealQL syntax, query builders, and result parsing patterns used in `db/src/queries/`.

## Query Module Structure

Each query module in `db/src/queries/` exports a single `find_*` or `*_query` function that:

1. Validates regex patterns (if applicable) via `validate_regex_patterns()`
2. Builds a SurrealQL query string with parameters
3. Executes via `db.execute_query()` with bound parameters
4. Extracts results into typed Rust structs

Parameters are bound using SurrealDB's parameter binding to prevent injection.

## Query Categories

### Data Import
- `import` - Import JSON call graph data into database relations

### Basic Lookups
- `location` - Find function definition locations by name
- `function` - Get function signatures with type information
- `search` - Full-text search across functions, specs, and types
- `file` - List all functions defined in a module/file

### Call Graph Traversal
- `calls_from` / `calls_to` - Find outgoing/incoming calls (wrappers around `calls`)
- `calls` - Unified call query with `CallDirection::From` / `CallDirection::To`
- `trace` / `reverse_trace` - Forward/backward recursive call tracing
- `path` - Find call path between two functions

### Dependency Analysis
- `depends_on` / `depended_by` - Module dependency queries (wrappers around `dependencies`)
- `dependencies` - Unified dependency query with `DependencyDirection`

### Code Quality
- `unused` - Find functions that are never called
- `hotspots` - Find most-called functions (high fan-in)

### Type System
- `specs` - Query @spec and @callback definitions
- `types` - Query @type, @typep, and @opaque definitions
- `structs` - Query struct definitions with field info

## SurrealQL Syntax Patterns

### Basic SELECT with Parameters

```sql
SELECT name, source
FROM modules
WHERE name = $pattern
ORDER BY name
LIMIT $limit
```

Parameters are bound via `QueryParams`:

```rust
let params = QueryParams::new()
    .with_str("pattern", pattern)
    .with_int("limit", limit as i64);
```

### Regex Matching

SurrealDB supports regex via type casting. Use `<regex>$pattern` to create a regex from a string parameter:

```sql
-- Regex match using type casting
WHERE name = <regex>$pattern

-- Regex match using string::matches() function
WHERE string::matches(module_name, $module_pattern)
```

**Important:** SurrealDB does not honor `ORDER BY` when using regex `WHERE` clauses. Always sort results in Rust after the query.

### Edge Table Queries (calls)

The `calls` table is a graph edge connecting `functions` records. Access connected record properties via dot notation:

```sql
SELECT
    in.name as caller_name,
    in.module_name as caller_module,
    in.arity as caller_arity,
    out.module_name as callee_module,
    out.name as callee_function,
    out.arity as callee_arity,
    line as callee_line
FROM calls
WHERE in.module_name = $module_pattern
ORDER BY in.module_name, in.name
LIMIT $limit
```

**SurrealDB quirk:** Combining `in.module_name = X AND in.name = Y` in a WHERE clause may return 0 rows. Use `type::string(in.name) = Y` as a workaround for the second condition.

### Graph Traversal (trace queries)

Use SurrealDB's graph traversal operators for recursive call chain tracing:

```sql
-- Forward trace: follow calls from starting function
SELECT * FROM (
    SELECT VALUE id FROM functions
    WHERE module_name = $module AND name = $function
).{1..5+path+inclusive}->calls->functions.*
LIMIT $limit;

-- Reverse trace: find callers of starting function
SELECT * FROM (
    SELECT VALUE id FROM functions
    WHERE module_name = $module AND name = $function
).{1..5+path+inclusive}<-calls<-functions.*
LIMIT $limit;
```

Key syntax elements:
- `{1..N}` - Limits traversal depth
- `+path` - Returns full paths (not just endpoints)
- `+inclusive` - Includes the starting node
- `->calls->` - Follows outgoing edges
- `<-calls<-` - Follows incoming edges
- `functions.*` - Fetches full function records (not just IDs)

### Clause ID Traversal

Access related record properties through record links:

```sql
SELECT
    caller_clause_id.start_line as clause_start,
    caller_clause_id.end_line as clause_end
FROM calls
WHERE in = functions:[$caller_module, $caller_name, $caller_arity]
  AND out = functions:[$callee_module, $callee_name, $callee_arity]
```

### Composite Record IDs

SurrealDB supports array-based record IDs for composite keys:

```sql
-- Reference a function by composite key
functions:[$module, $name, $arity]
```

### Module Dependency Queries

Filter out self-references when querying module dependencies:

```sql
SELECT in, out, line FROM calls
WHERE in.module_name = $module_pattern
  AND in.module_name != out.module_name
LIMIT $limit;
```

## Query Builders

### ConditionBuilder

Builds WHERE clause conditions that switch between exact and regex matching:

```rust
let builder = ConditionBuilder::new("module", "module_pattern");
builder.build(false); // "module == $module_pattern"
builder.build(true);  // "regex_matches(module, $module_pattern)"
```

Supports `.with_leading_comma()` for mid-query conditions.

### OptionalConditionBuilder

Handles optional parameters (e.g., function pattern, arity):

```rust
let builder = OptionalConditionBuilder::new("arity", "arity")
    .with_leading_comma()
    .when_none("true");

builder.build(true);  // ", arity == $arity"
builder.build(false); // ", true"
```

Supports `.with_regex()` for optional regex-aware fields.

### Regex Validation

All query functions validate regex patterns before execution using `validate_regex_patterns()`:

```rust
validate_regex_patterns(use_regex, &[Some(module_pattern), function_pattern])?;
```

This validates patterns using the same Rust `regex` crate that SurrealDB uses internally, providing clear error messages at the CLI boundary rather than cryptic database errors.

## Result Parsing Patterns

### Column Order

SurrealDB returns columns in **alphabetical order by alias name**, not in SELECT clause order. Always account for this when extracting by index:

```rust
// SELECT call_type, callee_arity, callee_function, ...
// Returns in alphabetical order: call_type=0, callee_arity=1, callee_function=2, ...
let call_type_str = extract_string_or(row.get(0).unwrap(), "");
let callee_arity = extract_i64(row.get(1).unwrap(), 0);
```

### Header-Based Layout

For resilience to column reordering, use `CallRowLayout::from_headers()`:

```rust
let layout = CallRowLayout::from_headers(result.headers())?;
let call = extract_call_from_row_trait(row, &layout);
```

### Type-Safe Extraction Helpers

Located in `db/src/db.rs`:

- `extract_string(value)` - Returns `Option<String>`, None if not a string
- `extract_i64(value, default)` - Returns i64, falls back to default
- `extract_string_or(value, default)` - Returns String, falls back to default
- `extract_bool(value, default)` - Returns bool, falls back to default
- `extract_f64(value, default)` - Returns f64, falls back to default

### Record Reference Extraction

For graph edges, extract function references from SurrealDB record IDs:

```rust
// From Thing ID format (composite record ID)
let id = value.as_thing_id()?;
let parts = id.as_array()?;
let module = parts.get(0)?.as_str()?;
let name = parts.get(1)?.as_str()?;
let arity = parts.get(2)?.as_i64()?;

// From full object (via .* query)
let module = value.get("module_name")?.as_str()?;
let name = value.get("name")?.as_str()?;
let arity = value.get("arity")?.as_i64()?;
```

## Performance Notes

- All queries are indexed by module/function names
- Most lookups are O(log n) with O(m) result iteration where m is result count
- Trace queries may be O(n * depth) in worst case for highly connected graphs
- SurrealDB uses the same Rust `regex` crate, so validation and execution produce identical results

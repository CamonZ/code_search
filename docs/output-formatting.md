# Output Formatting

Commands return a result type implementing `Outputable`. Three formats are available: `Table` (default), `Json`, `Toon`.

`Outputable` requires `Serialize` + `to_table()` method. JSON and Toon formats are derived automatically from `Serialize`.

## Table Format

Human-readable, optimized for terminal display. Hand-crafted in each command's `to_table()` method to show the most relevant information clearly.

## JSON Format

Standard JSON via `serde_json::to_string_pretty()`. Uses the struct's `#[derive(Serialize)]` implementation. Nested structures serialize as nested objects/arrays. Use `#[serde(skip_serializing_if = "...")]` to omit empty collections.

## Toon Format

Token-efficient serialization via the `toon` crate. Automatically derived from the same Serialize implementation as JSON. Key design principles:

- Designed for LLM consumption (minimal tokens while preserving structure)
- Arrays show count in brackets: `callers[2]:` means 2 items follow
- Objects omit braces, use indentation for nesting
- Inline notation for simple objects: `targets[1]{arity,function,line}: 2,get,15`
- Empty collections still show: `modules[0]:` indicates empty array
- Whitespace-sensitive (indentation conveys hierarchy)

## Consistency Rules

When refactoring output, ensure all three formats remain consistent:

1. The struct hierarchy should make sense for both JSON and toon
2. Test fixtures exist in `db/src/fixtures/output/<command>/` for JSON and toon
3. Output tests verify round-trip consistency between formats

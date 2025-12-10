# import - Examples

## Basic Import

```bash
code_search --format toon import --file extracted_trace.json
```

Output:
```
calls_imported: 1513
functions_imported: 260
locations_imported: 1617
modules_imported: 92
schemas_created[7]: functions,calls,modules,specs,struct_fields,types,function_locations
specs_imported: 260
structs_imported: 164
types_imported: 27
```

## Import with Project Namespace

Useful for comparing multiple codebases or versions:

```bash
code_search --format toon import --file phoenix_v1.json --project phoenix_v1
code_search --format toon import --file phoenix_v2.json --project phoenix_v2
```

## Clear and Re-import

Replace all existing data:

```bash
code_search --format toon import --file call_graph.json --clear
```

## Import to Specific Database

```bash
code_search --db /path/to/my.db --format toon import --file call_graph.json
```

## Expected JSON Structure

The import expects a JSON file with this structure:

```json
{
  "function_locations": {
    "Module.Name": {
      "function/arity:line": {
        "source_file": "lib/path/file.ex",
        "line": 10,
        "start_line": 10,
        "end_line": 25,
        "kind": "def",
        "pattern": "arg1, arg2",
        "guard": "when is_binary(arg1)"
      }
    }
  },
  "calls": [
    {
      "caller": {"module": "A", "function": "foo", "file": "...", "line": 10},
      "callee": {"module": "B", "function": "bar", "arity": 2},
      "type": "remote"
    }
  ],
  "specs": { ... },
  "types": { ... },
  "structs": { ... }
}
```

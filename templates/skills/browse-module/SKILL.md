---
name: browse-module
description: Browse all definitions in a module or file. Use to get a complete overview of functions, specs, types, and structs.
---

# browse-module

Browse all definitions in a module or file.

## Purpose

Get a complete overview of all functions, specs, types, and structs in a specific module or file. Use this to understand what a module provides and explore its complete interface.

## Usage

```bash
code_search --format toon browse-module <MODULE_OR_FILE> [OPTIONS]
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<MODULE_OR_FILE>` | Module name, pattern, or file path to browse. Can be: module name ("MyApp.Accounts"), file path ("lib/accounts.ex"), or pattern with --regex |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-k, --kind <KIND>` | Filter by definition type: `functions`, `specs`, `types`, `structs` | all types |
| `-n, --name <NAME>` | Filter by definition name (substring or regex with --regex) | none |
| `-r, --regex` | Treat patterns as regular expressions | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
module: MyApp.User
file: lib/my_app/user.ex
functions[N]{arity,line,name,type}:
  1,15,create_user,def
  2,28,update_user,def
structs[N]{fields[N]{default,field,inferred_type,required},name}:
  MyApp.User,name,true,binary(),true
```

## When to Use

- Understanding what a module provides
- Exploring module interfaces and APIs
- Finding all functions in a specific module
- Getting an overview of module structure
- Filtering to specific definition types (functions only, types only, etc.)

## See Also

- `function` - Get detailed function signatures
- `search` - Find modules or functions by pattern
- `location` - Find where functions are defined

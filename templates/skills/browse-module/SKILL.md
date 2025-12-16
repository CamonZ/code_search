---
name: browse-module
description: Browse all definitions in a module or file. Use to get a complete overview of functions, structs, and definitions.
---

# browse-module

Browse all definitions in a module or file.

## Purpose

Get a complete overview of all functions, structs, and definitions in a specific module or file. Use this to understand what a module provides and explore its complete interface.

## Usage

```bash
code_search --format toon browse-module --module <MODULE> [OPTIONS]
```

## Required Options

| Option | Description |
|--------|-------------|
| `-m, --module <MODULE>` | Module name to browse |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-r, --regex` | Treat module as regex | false |
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

## See Also

- [examples.md](examples.md) for detailed usage examples
- `function` - Get detailed function signatures
- `search` - Find modules or functions by pattern
- `location` - Find where functions are defined

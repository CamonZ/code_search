---
name: struct-modules
description: Show which modules work with a given struct type. Use this to understand the scope of impact when changing a struct definition and find all places that use specific data structures.
---

# struct-modules

Show which modules work with a given struct type.

## Purpose

Find all modules that use, create, or manipulate a specific struct type. Use this to understand the scope of impact when changing a struct definition.

## Usage

```bash
code_search --format toon struct-modules --struct <STRUCT_NAME> [OPTIONS]
```

## Required Options

| Option | Description |
|--------|-------------|
| `-s, --struct <STRUCT_NAME>` | Struct name to analyze |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-r, --regex` | Treat struct name as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
modules[N]{name,usage_count}:
  MyApp.UserController,5
  MyApp.UserService,3
  MyApp.UserView,2
```

## When to Use

- Understanding struct usage across the codebase
- Planning struct refactoring and changes
- Finding all places that work with specific data structures
- Impact analysis for struct modifications

## See Also

- [examples.md](examples.md) for detailed usage examples
- `struct-usage` - Find specific usage of struct types
- `accepts` - Find functions accepting specific types
- `returns` - Find functions returning specific types

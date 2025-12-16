---
name: struct-usage
description: Find functions that accept or return a specific struct type. Use this to understand how structs flow through the system and plan struct refactoring.
---

# struct-usage

Find functions that accept or return a specific struct type.

## Purpose

Find all functions that work with a specific struct type, either as input parameters or return values. Use this to understand how structs flow through the system.

## Usage

```bash
code_search --format toon struct-usage --struct <STRUCT_NAME> [OPTIONS]
```

## Required Options

| Option | Description |
|--------|-------------|
| `-s, --struct <STRUCT_NAME>` | Struct name to analyze |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-m, --module <PATTERN>` | Module pattern to filter | all |
| `-r, --regex` | Treat patterns as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
items[N]{entries[N]{line,name,usage_type},file,name}:
  lib/user.ex,MyApp.User,create_user/1,15,create_user,returns
  lib/user.ex,MyApp.User,update_user/2,28,update_user,accepts
total_items: 2
```

## When to Use

- Understanding struct data flow through the system
- Finding all functions that work with specific structs
- Planning struct refactoring and API changes
- Analyzing struct coupling and dependencies

## See Also

- [examples.md](examples.md) for detailed usage examples
- `struct-modules` - Find modules using specific structs
- `accepts` - Find functions accepting any type
- `returns` - Find functions returning any type

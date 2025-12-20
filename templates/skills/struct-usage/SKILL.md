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
code_search --format toon struct-usage <PATTERN> [MODULE] [OPTIONS]
```

## Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `<PATTERN>` | Type pattern to search for in both inputs and returns | required |
| `[MODULE]` | Module filter pattern | all modules |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `--by-module` | Aggregate results by module (show counts instead of function details) | false |
| `-r, --regex` | Treat patterns as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Examples

```bash
code_search struct-usage "User.t"             # Find functions using User.t
code_search struct-usage "Changeset.t"        # Find functions using Changeset.t
code_search struct-usage "User.t" MyApp       # Filter to module MyApp
code_search struct-usage "User.t" --by-module # Summarize by module
code_search struct-usage -r ".*\.t"           # Regex pattern matching
```

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

- `accepts` - Find functions accepting any type
- `returns` - Find functions returning any type

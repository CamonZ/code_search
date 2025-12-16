---
name: returns
description: Find functions that return specific type patterns. Use this to understand what types are produced by the system and trace data flow from function outputs.
---

# returns

Find functions that return specific type patterns.

## Purpose

Find functions that return values of certain types. Use this to understand what types are produced by the system and trace data flow from function outputs.

## Usage

```bash
code_search --format toon returns --type <TYPE_PATTERN> [OPTIONS]
```

## Required Options

| Option | Description |
|--------|-------------|
| `-t, --type <TYPE_PATTERN>` | Type pattern to search for |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-m, --module <PATTERN>` | Module pattern to filter | all |
| `-r, --regex` | Treat patterns as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
items[N]{entries[N]{line,name,return_type},file,name}:
  lib/user.ex,MyApp.User,get_user/1,15,get_user
  lib/repo.ex,MyApp.Repo,find_user/2,23,find_user
total_items: 2
```

## When to Use

- Finding functions that produce specific types
- Understanding system outputs and data flow
- Planning type refactoring and changes
- Tracing where certain data structures originate

## See Also

- [examples.md](examples.md) for detailed usage examples
- `accepts` - Find functions accepting specific types
- `struct-usage` - Find usage of struct types
- `function` - Get detailed function signatures

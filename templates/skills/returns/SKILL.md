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
code_search --format toon returns <PATTERN> [MODULE] [OPTIONS]
```

## Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `<PATTERN>` | Type pattern to search for in return types | required |
| `[MODULE]` | Module filter pattern | all modules |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-r, --regex` | Treat patterns as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Examples

```bash
code_search returns "User.t"              # Find functions returning User.t
code_search returns "nil"                 # Find functions returning nil
code_search returns "{:error" MyApp       # Filter to module MyApp
code_search returns -r "list\(.*\)"       # Regex pattern matching
```

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

- `accepts` - Find functions accepting specific types
- `struct-usage` - Find usage of struct types
- `function` - Get detailed function signatures

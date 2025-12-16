---
name: accepts
description: Find functions that accept specific type patterns in their parameters. Use for understanding data flow, planning type changes, and finding functions that work with specific entities.
---

# accepts

Find functions that accept specific type patterns.

## Purpose

Find functions that take certain types as input parameters. Use this to understand what functions work with specific data types and trace data flow into function calls.

## Usage

```bash
code_search --format toon accepts --type <TYPE_PATTERN> [OPTIONS]
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
items[N]{entries[N]{args,line,name},file,name}:
  lib/email.ex,MyApp.Email,send_welcome/1,15,send_welcome
  lib/order.ex,MyApp.Order,process_for_user/2,23,process_for_user
total_items: 2
```

## When to Use

- Finding functions that process specific entities
- Understanding data flow into function calls
- Planning type refactoring and changes
- Finding functions that validate specific inputs

## See Also

- `returns` - Find functions returning specific types
- `struct-usage` - Find usage of struct types
- `function` - Get detailed function signatures

---
name: large-functions
description: Find functions that exceed recommended size limits. Use this to identify functions that are too long and likely need refactoring for better maintainability.
---

# large-functions

Find functions that exceed recommended size limits.

## Purpose

Identify functions that are too long or complex based on line count. Use this to find functions that likely violate the single responsibility principle and need refactoring.

## Usage

```bash
code_search --format toon large-functions [MODULE] [OPTIONS]
```

## Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `[MODULE]` | Module filter pattern (substring or regex with -r) | all modules |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `--min-lines <N>` | Minimum lines to be considered large | 50 |
| `--include-generated` | Include macro-generated functions (excluded by default) | false |
| `-r, --regex` | Treat patterns as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Examples

```bash
code_search large-functions                     # Find functions with 50+ lines
code_search large-functions MyApp.Web           # Filter to MyApp.Web namespace
code_search large-functions --min-lines 100     # Find functions with 100+ lines
code_search large-functions --include-generated # Include macro-generated functions
code_search large-functions -l 20               # Show top 20 largest functions
```

## Output Fields (toon format)

```
items[N]{entries[N]{line_count,line,name},file,name}:
  lib/my_app/user.ex,MyApp.User,create_user/3,45,create_user,156
  lib/my_app/order.ex,MyApp.Order,process_order/2,23,process_order,134
total_items: 2
```

## When to Use

- Finding functions that are too long to understand easily
- Prioritizing refactoring based on function size
- Improving code readability and maintainability
- Measuring adherence to coding standards

## See Also

- `complexity` - Find complex functions by logic complexity
- `god-modules` - Find large modules
- `unused` - Find unused functions

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
code_search --format toon large-functions [OPTIONS]
```

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-m, --module <PATTERN>` | Module pattern to filter | all |
| `-r, --regex` | Treat patterns as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

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

- [examples.md](examples.md) for detailed usage examples
- `complexity` - Find complex functions by logic complexity
- `god-modules` - Find large modules
- `unused` - Find unused functions

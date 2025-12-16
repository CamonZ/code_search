---
name: god-modules
description: Find modules with high function count and connectivity (god modules). Use this to identify modules violating single responsibility principle that should be split or refactored.
---

# god-modules

Find modules with high function count and connectivity (god modules).

## Purpose

Identify modules that violate single responsibility principle by having too many functions and/or too many connections. Use this to find modules that should be split or refactored.

## Usage

```bash
code_search --format toon god-modules [OPTIONS]
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
items[N]{entries[N]{connectivity,function_count,line,name},file,name}:
  lib/my_app/user.ex,MyApp.User,manage_profile/1,45,manage_profile,156
  lib/my_app/user.ex,MyApp.User,validate_user/2,23,validate_user,89
total_items: 2
```

## When to Use

- Finding modules that do too many things
- Planning module refactoring and splitting
- Identifying architectural problems
- Measuring code organization quality

## See Also

- [examples.md](examples.md) for detailed usage examples
- `hotspots` - Find high-connectivity functions
- `duplicate-hotspots` - Find modules with duplication
- `large-functions` - Find individual large functions

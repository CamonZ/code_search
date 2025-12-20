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
code_search --format toon god-modules [MODULE] [OPTIONS]
```

## Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `[MODULE]` | Module filter pattern (substring or regex with -r) | all modules |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `--min-functions <N>` | Minimum function count to be considered a god module | 20 |
| `--min-loc <N>` | Minimum lines of code to be considered a god module | 0 |
| `--min-total <N>` | Minimum total connectivity (incoming + outgoing) | 10 |
| `-r, --regex` | Treat patterns as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Examples

```bash
code_search god-modules                         # Find all god modules
code_search god-modules MyApp.Core              # Filter to MyApp.Core namespace
code_search god-modules --min-functions 30      # With minimum 30 functions
code_search god-modules --min-loc 500           # With minimum 500 lines of code
code_search god-modules --min-total 15          # With minimum 15 total connectivity
code_search god-modules -l 20                   # Show top 20 god modules
```

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

- `hotspots` - Find high-connectivity functions
- `large-functions` - Find individual large functions

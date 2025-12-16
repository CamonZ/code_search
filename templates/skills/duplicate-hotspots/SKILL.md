---
name: duplicate-hotspots
description: Find modules with the most duplicated functions. Use this to prioritize refactoring efforts and focus on modules with the highest duplication.
---

# duplicate-hotspots

Find modules with the most duplicated functions.

## Purpose

Identify modules that contain the highest amount of code duplication. Use this to prioritize refactoring efforts and focus on modules with the most duplication.

## Usage

```bash
code_search --format toon duplicate-hotspots [OPTIONS]
```

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-m, --module <PATTERN>` | Module pattern to filter | all |
| `-x, --exact` | Use exact source matching | false |
| `-r, --regex` | Treat patterns as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
modules[N]{duplicate_count,name,top_duplicates[N]{copy_count,name,arity}}:
  MyApp.User,5,validate_email,1,3
  MyApp.Order,3,calculate_total,2,2
total_duplicates: 8
total_modules: 2
```

## When to Use

- Prioritizing refactoring efforts by duplication volume
- Identifying modules with maintenance issues
- Planning consolidation strategies
- Measuring code quality at module level

## See Also

- [examples.md](examples.md) for detailed usage examples
- `duplicates` - Find individual duplicate functions
- `complexity` - Find complex functions
- `god-modules` - Find large modules

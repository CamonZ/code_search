---
name: duplicates
description: Find functions with identical or near-identical implementations. Use this to identify code duplication for refactoring, consolidation, and improving maintainability.
---

# duplicates

Find functions with identical or near-identical implementations.

## Purpose

Identify code duplication to find opportunities for refactoring and consolidation. Use this to reduce maintenance burden and improve code quality.

## Usage

```bash
code_search --format toon duplicates [OPTIONS]
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
duplicates[N]{duplicate_functions[N]{arity,file,line,module,name},hash}:
  lib/user.ex,MyApp.User,create_user,1,45,abc123
  lib/order.ex,MyApp.Order,validate_input,2,23,def456
```

## When to Use

- Finding code duplication for refactoring
- Consolidating similar functions
- Improving maintainability
- Reducing bug propagation risks

## See Also

- [examples.md](examples.md) for detailed usage examples
- `duplicate-hotspots` - Find modules with most duplicates
- `complexity` - Find complex functions
- `unused` - Find unused functions

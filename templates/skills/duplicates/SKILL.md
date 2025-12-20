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
code_search --format toon duplicates [MODULE] [OPTIONS]
```

## Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `[MODULE]` | Module filter pattern (substring or regex with -r) | all modules |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `--exact` | Use exact source matching instead of AST matching | false |
| `--by-module` | Aggregate results by module (show which modules have most duplicates) | false |
| `--exclude-generated` | Exclude macro-generated functions | false |
| `-r, --regex` | Treat patterns as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Examples

```bash
code_search duplicates                       # Find all duplicate functions
code_search duplicates MyApp                 # Filter to specific module
code_search duplicates --by-module           # Rank modules by duplication
code_search duplicates --exact               # Use exact source matching
code_search duplicates --exclude-generated   # Exclude macro-generated functions
```

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

- `complexity` - Find complex functions
- `unused` - Find unused functions

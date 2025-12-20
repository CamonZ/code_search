---
name: complexity
description: Display complexity metrics for functions. Use this to identify potentially problematic or hard-to-maintain code and find functions that may need refactoring.
---

# complexity

Display complexity metrics for functions.

## Purpose

Analyze function complexity to identify potentially problematic or hard-to-maintain code. Use this to find functions that may need refactoring or closer review.

## Usage

```bash
code_search --format toon complexity [MODULE] [OPTIONS]
```

## Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `[MODULE]` | Module filter pattern (substring match by default, regex with --regex) | all modules |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `--min <N>` | Minimum complexity threshold | 1 |
| `--min-depth <N>` | Minimum nesting depth threshold | 0 |
| `--exclude-generated` | Exclude macro-generated functions | false |
| `-r, --regex` | Treat patterns as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Examples

```bash
code_search complexity                      # Show all functions with complexity >= 1
code_search complexity MyApp.Accounts       # Filter to MyApp.Accounts module
code_search complexity --min 10             # Show functions with complexity >= 10
code_search complexity --min-depth 3        # Show functions with nesting depth >= 3
code_search complexity --exclude-generated  # Exclude macro-generated functions
code_search complexity -l 20                # Show top 20 most complex functions
```

## Output Fields (toon format)

```
items[N]{entries[N]{complexity,entries,line,name},file,name}:
  lib/my_app/user.ex,MyApp.User,validate_params/1,15,validate_params,42
  lib/my_app/order.ex,MyApp.Order,process_payment/2,23,process_payment,67
```

## When to Use

- Finding functions that may be too complex
- Prioritizing refactoring efforts
- Identifying code that needs review
- Measuring code quality metrics

## See Also

- `large-functions` - Find functions by line count
- `hotspots` - Find high-connectivity functions
- `duplicates` - Find similar function implementations

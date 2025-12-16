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
code_search --format toon complexity [OPTIONS]
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

- [examples.md](examples.md) for detailed usage examples
- `large-functions` - Find functions by line count
- `hotspots` - Find high-connectivity functions
- `duplicates` - Find similar function implementations

---
name: many-clauses
description: Find functions with many pattern-matched heads (multiple clauses). Use this to identify complex pattern matching that may need refactoring into separate functions.
---

# many-clauses

Find functions with many pattern-matched heads (multiple clauses).

## Purpose

Identify functions with many pattern matching clauses that may be complex or hard to maintain. Use this to find functions that might benefit from refactoring into separate functions or using different patterns.

## Usage

```bash
code_search --format toon many-clauses [OPTIONS]
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
items[N]{entries[N]{clause_count,line,name},file,name}:
  lib/parser.ex,MyApp.Parser,parse_input/1,15,parse_input,8
  lib/validator.ex,MyApp.Validator,validate_data/2,23,validate_data,6
total_items: 2
```

## When to Use

- Finding functions with complex pattern matching
- Identifying potential refactoring candidates
- Understanding code complexity patterns
- Improving code readability

## See Also

- [examples.md](examples.md) for detailed usage examples
- `complexity` - Find functions by logic complexity
- `large-functions` - Find functions by line count
- `duplicates` - Find duplicated implementations

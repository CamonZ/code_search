---
name: boundaries
description: Find boundary modules with high fan-in but low fan-out. Use to identify architectural boundaries and stable interfaces.
---

# boundaries

Find boundary modules with high fan-in but low fan-out.

## Purpose

Identify modules that act as architectural boundaries - modules that are heavily used by others but don't depend on many external modules. Use this to understand system architecture and identify stable interfaces.

## Usage

```bash
code_search --format toon boundaries [OPTIONS]
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
items[N]{entries[N]{fan_in,fan_out,line,name},file,name}:
  lib/my_app/repo.ex,MyApp.Repo,get_user/1,45,get_user,156
total_items: 2
```

## When to Use

- Understanding system architecture and layering
- Identifying stable APIs and interfaces
- Finding modules that define contracts
- Planning refactoring and dependency management

## See Also

- [examples.md](examples.md) for detailed usage examples
- `hotspots` - Find high-connectivity functions
- `depends-on` - See module dependencies
- `depended-by` - See modules that depend on others

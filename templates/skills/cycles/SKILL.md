---
name: cycles
description: Detect circular dependencies between modules. Use this to find dependency cycles that cause compilation issues, tight coupling, and maintenance problems.
---

# cycles

Detect circular dependencies between modules.

## Purpose

Find circular dependencies in the module graph that can cause compilation issues, tight coupling, and maintenance problems. Use this to identify and break problematic dependency cycles.

## Usage

```bash
code_search --format toon cycles [OPTIONS]
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
cycles[N]{cycle_length,modules[N]{name}}:
  3,MyApp.User,MyApp.Order,MyApp.User
  2,MyApp.Config,MyApp.Utils,MyApp.Config
```

## When to Use

- Finding circular dependencies that cause compilation issues
- Identifying tight coupling between modules
- Planning dependency injection and interface extraction
- Improving build times and reducing complexity

## See Also

- [examples.md](examples.md) for detailed usage examples
- `depends-on` - See module dependencies
- `clusters` - Analyze module connectivity patterns

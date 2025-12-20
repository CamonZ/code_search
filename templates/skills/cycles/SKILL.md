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
code_search --format toon cycles [MODULE] [OPTIONS]
```

## Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `[MODULE]` | Module filter pattern (substring match by default, regex with --regex) | all modules |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `--max-length <N>` | Maximum cycle length to find | none |
| `--involving <MODULE>` | Only show cycles involving this module (substring match) | none |
| `-r, --regex` | Treat patterns as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Examples

```bash
code_search cycles                            # Find all cycles
code_search cycles MyApp.Core                 # Filter to MyApp.Core namespace
code_search cycles --max-length 3             # Only show cycles of length <= 3
code_search cycles --involving MyApp.Accounts # Only cycles involving Accounts
```

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

- `depends-on` - See module dependencies
- `clusters` - Analyze module connectivity patterns

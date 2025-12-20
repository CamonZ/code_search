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
code_search --format toon boundaries [MODULE] [OPTIONS]
```

## Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `[MODULE]` | Module filter pattern (substring or regex with -r) | all modules |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `--min-incoming <N>` | Minimum incoming calls to be considered a boundary module | 1 |
| `--min-ratio <N>` | Minimum ratio (incoming/outgoing) to be considered a boundary | 2.0 |
| `-r, --regex` | Treat patterns as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Examples

```bash
code_search boundaries                          # Find all boundary modules
code_search boundaries MyApp.Web                # Filter to MyApp.Web namespace
code_search boundaries --min-incoming 5         # With minimum 5 incoming calls
code_search boundaries --min-ratio 2.0          # With minimum 2.0 ratio
code_search boundaries -l 20                    # Show top 20 boundary modules
```

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

- `hotspots` - Find high-connectivity functions
- `depends-on` - See module dependencies
- `depended-by` - See modules that depend on others

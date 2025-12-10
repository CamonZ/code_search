# hotspots

Find functions with the most incoming/outgoing calls.

## Purpose

Identify the most connected functions in the codebase. These are often critical code paths, utility functions, or potential refactoring targets.

## Usage

```bash
code_search --format toon hotspots [OPTIONS]
```

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-k, --kind <KIND>` | Type: `incoming`, `outgoing`, `total` | `incoming` |
| `-m, --module <MODULE>` | Filter to module pattern | all |
| `-r, --regex` | Treat module as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 20 |
| `--project <NAME>` | Project to search in | `default` |

## Hotspot Types

| Kind | Description |
|------|-------------|
| `incoming` | Most called functions (high fan-in) |
| `outgoing` | Functions calling many others (high fan-out) |
| `total` | Highest combined connections |

## Output Fields (toon format)

```
hotspots[N]{function,incoming,module,outgoing,total}:
  web_path,20,Mix.Phoenix,0,20
  expand_alias,16,Phoenix.Router,0,16
```

## When to Use

- Finding critical utility functions
- Identifying potential bottlenecks
- Finding refactoring candidates (high coupling)
- Understanding codebase structure

## See Also

- [examples.md](examples.md) for detailed usage examples
- `unused` - Find functions with zero connections

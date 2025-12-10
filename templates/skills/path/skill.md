# path

Find a call path between two functions.

## Purpose

Discover if and how one function can reach another through the call graph. Returns the shortest path(s) connecting the source to the target.

## Usage

```bash
code_search --format toon path --from-module <MOD> --from-function <FN> --to-module <MOD> --to-function <FN> [OPTIONS]
```

## Required Options

| Option | Description |
|--------|-------------|
| `--from-module <MODULE>` | Source module name |
| `--from-function <NAME>` | Source function name |
| `--to-module <MODULE>` | Target module name |
| `--to-function <NAME>` | Target function name |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `--from-arity <N>` | Source function arity | all |
| `--to-arity <N>` | Target function arity | all |
| `--depth <N>` | Max search depth (1-20) | 10 |
| `-l, --limit <N>` | Max paths to return (1-100) | 10 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
paths[N]{path}: [["Module.A", "func1", 1], ["Module.B", "func2", 2], ...]
```

## When to Use

- Verifying if two functions are connected
- Finding how execution flows between components
- Understanding indirect dependencies
- Debugging: "how does X end up calling Y?"

## See Also

- [examples.md](examples.md) for detailed usage examples
- `trace` - Forward traversal from a starting point
- `reverse-trace` - Backward traversal to a target

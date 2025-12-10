# unused

Find functions that are never called.

## Purpose

Identify dead code - functions that exist but are never called from anywhere in the codebase. Helps with code cleanup and maintenance.

## Usage

```bash
code_search --format toon unused [OPTIONS]
```

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-m, --module <MODULE>` | Filter to specific module | all |
| `-p, --private-only` | Only show private functions (defp) | false |
| `-P, --public-only` | Only show public functions (def) | false |
| `-x, --exclude-generated` | Exclude __struct__, __using__, etc. | false |
| `-r, --regex` | Treat module as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
functions[N]{arity,file,kind,line,module,name,project}:
  1,lib/my_module.ex,def,42,MyApp.Utils,unused_helper,default
```

## When to Use

- Code cleanup: finding dead code to remove
- Finding orphan private functions
- Identifying potential entry points (public but uncalled)
- Code review: checking for forgotten implementations

## See Also

- [examples.md](examples.md) for detailed usage examples
- `hotspots` - Find most-used functions (opposite of unused)

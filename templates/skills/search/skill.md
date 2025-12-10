# search

Search for modules or functions by name pattern.

## Purpose

Find modules or functions matching a pattern. Use this as a starting point to discover what's in the codebase before drilling down with other commands.

## Usage

```bash
code_search --format toon search --pattern <PATTERN> [OPTIONS]
```

## Required Options

| Option | Description |
|--------|-------------|
| `-p, --pattern <PATTERN>` | Search pattern (substring match, or regex with `-r`) |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-k, --kind <KIND>` | What to search: `modules` or `functions` | `modules` |
| `-r, --regex` | Treat pattern as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

- **modules**: `modules[N]{name,project}: ...`
- **functions**: `functions[N]{args,module,name,project,return_type}: ...`

## When to Use

- Discovering module names in an unfamiliar codebase
- Finding functions by naming convention (e.g., `get_`, `handle_`)
- Initial exploration before using `calls-to` or `calls-from`

## See Also

- [examples.md](examples.md) for detailed usage examples
- `location` - Find where functions are defined
- `specs` - See function type specifications

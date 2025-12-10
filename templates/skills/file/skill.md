# file

Show all functions defined in a file.

## Purpose

List all functions in a source file with their line ranges, kinds, and patterns. Useful for exploring file contents without opening the file.

## Usage

```bash
code_search --format toon file --file <PATH> [OPTIONS]
```

## Required Options

| Option | Description |
|--------|-------------|
| `-f, --file <PATH>` | File path pattern (substring match or regex) |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-r, --regex` | Treat path as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
files{<path>}[N]{arity,end_line,guard,kind,module,name,pattern,start_line}:
  0,462,"",defmacro,Phoenix.Channel,__using__,opts,450
  3,569,is_atom(status),def,Phoenix.Channel,reply,"socket_ref, status",567
```

## When to Use

- Quick overview of a file's contents
- Finding functions by file path
- Understanding file organization
- Navigating to specific functions

## See Also

- [examples.md](examples.md) for detailed usage examples
- `location` - Find where a specific function is defined
- `search` - Find functions by name

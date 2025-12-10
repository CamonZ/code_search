# specs

Show @spec and @callback definitions.

## Purpose

Display full @spec and @callback definitions for functions in a module. Shows the complete type specification including all clauses.

## Usage

```bash
code_search --format toon specs <MODULE> [OPTIONS]
```

## Required Arguments

| Argument | Description |
|----------|-------------|
| `<MODULE>` | Module name (positional argument) |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-f, --function <NAME>` | Filter to specific function | all |
| `-k, --kind <KIND>` | Filter by kind: `spec` or `callback` | all |
| `-r, --regex` | Treat names as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
specs[N]{arity,definition,kind,line,module,name,project}:
  2,"@spec render(Plug.Conn.t(), ...) :: Plug.Conn.t()",spec,869,Phoenix.Controller,render,default
```

## When to Use

- Understanding complete type specifications
- Finding callback definitions for behaviours
- Reviewing API contracts
- Documentation reference

## See Also

- [examples.md](examples.md) for detailed usage examples
- `function` - Simplified signature view
- `types` - See @type definitions

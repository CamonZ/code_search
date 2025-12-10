# types

Show @type, @typep, and @opaque definitions.

## Purpose

Display custom type definitions in a module. Shows the full type definition including parameters.

## Usage

```bash
code_search --format toon types <MODULE> [OPTIONS]
```

## Required Arguments

| Argument | Description |
|----------|-------------|
| `<MODULE>` | Module name (positional argument) |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-n, --name <NAME>` | Filter by type name | all |
| `-k, --kind <KIND>` | Filter by kind: `type`, `typep`, `opaque` | all |
| `-r, --regex` | Treat names as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
types[N]{definition,kind,line,module,name,params,project}:
  "@type t() :: %{__struct__: Phoenix.Socket, ...}",type,273,Phoenix.Socket,t,"[]",default
```

## When to Use

- Understanding custom types in a module
- Finding type definitions for documentation
- Exploring the type system of a codebase

## See Also

- [examples.md](examples.md) for detailed usage examples
- `specs` - See @spec definitions that use these types
- `struct` - See struct field definitions

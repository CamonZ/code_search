# function

Show function signature (args, return type).

## Purpose

Display the type signature of a function, including argument types and return type. This information comes from @spec definitions in the source code.

## Usage

```bash
code_search --format toon function --module <MODULE> --function <NAME> [OPTIONS]
```

## Required Options

| Option | Description |
|--------|-------------|
| `-m, --module <MODULE>` | Module name |
| `-f, --function <NAME>` | Function name |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-a, --arity <N>` | Filter by arity | all |
| `-r, --regex` | Treat names as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
functions[N]{args,arity,module,name,project,return_type}:
  "Plug.Conn.t(), Keyword.t() | map() | binary() | atom()",2,Phoenix.Controller,render,default,"Plug.Conn.t()"
```

## When to Use

- Understanding function input/output types
- Quick reference for API signatures
- Checking type compatibility before calling

## See Also

- [examples.md](examples.md) for detailed usage examples
- `specs` - See full @spec definitions with all clauses
- `location` - See where functions are defined

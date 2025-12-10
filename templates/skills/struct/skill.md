# struct

Show struct fields, defaults, and types.

## Purpose

Display the fields of a struct including default values, whether fields are required, and inferred types.

## Usage

```bash
code_search --format toon struct --module <MODULE> [OPTIONS]
```

## Required Options

| Option | Description |
|--------|-------------|
| `-m, --module <MODULE>` | Module name containing the struct |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-r, --regex` | Treat module as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
structs[N]{default,field,inferred_type,module,project,required}:
  "%{}",assigns,"",Phoenix.Socket,default,false
  nil,channel,"",Phoenix.Socket,default,false
```

## When to Use

- Understanding struct shape and fields
- Finding default values for struct fields
- Checking which fields are required vs optional
- Exploring data structures in unfamiliar code

## See Also

- [examples.md](examples.md) for detailed usage examples
- `types` - See @type definitions for the struct

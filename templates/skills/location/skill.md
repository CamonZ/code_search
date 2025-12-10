# location

Find where a function is defined (file, line range, pattern, guard).

## Purpose

Locate function definitions with clause-level detail. Shows file path, line numbers, function patterns, and guards for each clause of a function.

## Usage

```bash
code_search --format toon location --function <NAME> [OPTIONS]
```

## Required Options

| Option | Description |
|--------|-------------|
| `-f, --function <NAME>` | Function name to find |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-m, --module <MODULE>` | Filter to specific module | all |
| `-a, --arity <N>` | Filter by arity | all |
| `-r, --regex` | Treat names as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
locations[N]{arity,end_line,file,guard,kind,module,name,pattern,project,start_line}:
  2,873,lib/phoenix/controller.ex,"is_binary(template) or is_atom(template)",def,Phoenix.Controller,render,"conn, template",default,872
```

## When to Use

- Finding source file for a function
- Understanding function clause patterns and guards
- Navigating to specific function definitions
- Seeing all clauses of a multi-clause function

## See Also

- [examples.md](examples.md) for detailed usage examples
- `function` - See type signature (args, return type)
- `specs` - See @spec definitions

# reverse-trace

Trace call chains backwards - who calls the callers of a target.

## Purpose

Follow the call graph backward from a target function to find all the ways execution can reach it. Shows the chain of callers up to a specified depth.

## Usage

```bash
code_search --format toon reverse-trace --module <MODULE> --function <NAME> [OPTIONS]
```

## Required Options

| Option | Description |
|--------|-------------|
| `-m, --module <MODULE>` | Target module name |
| `-f, --function <NAME>` | Target function name |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-a, --arity <N>` | Filter by arity | all |
| `--depth <N>` | Max traversal depth (1-20) | 5 |
| `-r, --regex` | Treat names as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
calls[N]{callee_arity,callee_function,callee_module,caller_function,caller_kind,caller_module,depth,file,line,project}:
  3,render,Phoenix.Controller,render/2,def,Phoenix.Controller,1,lib/controller.ex,877,default
  3,render,Phoenix.Controller,render/6,defp,Phoenix.Endpoint.RenderErrors,1,lib/render_errors.ex,124,default
```

## When to Use

- Finding all entry points that lead to a function
- Impact analysis: what code paths reach this function
- Understanding how a deep function gets called
- Tracing execution paths backward from errors

## See Also

- [examples.md](examples.md) for detailed usage examples
- `trace` - Forward traversal
- `calls-to` - Single-level callers
- `path` - Find specific path between two functions

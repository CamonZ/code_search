# trace

Trace call chains from a starting function (forward traversal).

## Purpose

Follow the call graph forward from a starting point to see what functions get called, and what those functions call, up to a specified depth. Useful for understanding execution flow.

## Usage

```bash
code_search --format toon trace --module <MODULE> --function <NAME> [OPTIONS]
```

## Required Options

| Option | Description |
|--------|-------------|
| `-m, --module <MODULE>` | Starting module name |
| `-f, --function <NAME>` | Starting function name |

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
  2,helper,MyModule,main/0,def,MyModule,1,lib/my_module.ex,10,default
  1,format,String,helper/2,defp,MyModule,2,lib/my_module.ex,25,default
```

## When to Use

- Understanding what code runs when a function is called
- Tracing execution paths forward
- Finding all transitive dependencies of a function
- Exploring unfamiliar code flow

## See Also

- [examples.md](examples.md) for detailed usage examples
- `reverse-trace` - Trace callers backward
- `calls-from` - Single-level forward calls
- `path` - Find specific path between two functions

# calls-to

Show what calls a module/function (incoming edges).

## Purpose

Find all callers of a given module or function. Use this to understand who depends on a piece of code and assess the impact of changes.

## Usage

```bash
code_search --format toon calls-to --module <MODULE> [OPTIONS]
```

## Required Options

| Option | Description |
|--------|-------------|
| `-m, --module <MODULE>` | Target module name |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-f, --function <NAME>` | Filter to specific function | all |
| `-a, --arity <N>` | Filter by arity | all |
| `-r, --regex` | Treat names as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
calls[N]{call_type,callee_arity,callee_function,callee_module,caller_function,caller_kind,caller_module,file,line,project}:
  remote,4,copy_from,Mix.Phoenix,copy_new_files/3,defp,Mix.Tasks.Phx.Gen.Auth,lib/mix/tasks/phx.gen.auth.ex,611,default
```

## When to Use

- Impact analysis: who will be affected if this function changes
- Finding all call sites for a function
- Understanding how widely used a function is
- Tracing execution flow backward

## See Also

- [examples.md](examples.md) for detailed usage examples
- `calls-from` - Find callees (forward direction)
- `reverse-trace` - Multi-level backward traversal
- `depended-by` - Module-level dependents

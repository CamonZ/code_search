# calls-from

Show what a module/function calls (outgoing edges).

## Purpose

Find all functions that a given module or function calls. Use this to understand what dependencies a piece of code has and trace execution flow forward.

## Usage

```bash
code_search --format toon calls-from --module <MODULE> [OPTIONS]
```

## Required Options

| Option | Description |
|--------|-------------|
| `-m, --module <MODULE>` | Source module name |

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
  remote,2,accepts,Phoenix.Controller,put_formats/2,defp,Phoenix.Endpoint.RenderErrors,lib/endpoint/render_errors.ex,163,default
```

## When to Use

- Understanding what a function depends on
- Tracing execution flow forward from an entry point
- Finding external dependencies of a module
- Impact analysis: what will be affected if a callee changes

## See Also

- [examples.md](examples.md) for detailed usage examples
- `calls-to` - Find callers (reverse direction)
- `trace` - Multi-level forward traversal
- `depends-on` - Module-level dependencies

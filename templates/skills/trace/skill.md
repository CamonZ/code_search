# trace - Examples

## Basic Forward Trace

```bash
code_search --format toon trace Phoenix.Endpoint.RenderErrors "__catch__" --depth 3
```

Output:
```
calls[N]{callee_arity,callee_function,callee_module,caller_function,caller_kind,caller_module,depth,file,line,project}:
  5,instrument_render_and_send,Phoenix.Endpoint.RenderErrors,__catch__/5,def,Phoenix.Endpoint.RenderErrors,1,lib/endpoint/render_errors.ex,62,default
  6,render,Phoenix.Endpoint.RenderErrors,instrument_render_and_send/5,defp,Phoenix.Endpoint.RenderErrors,2,lib/endpoint/render_errors.ex,85,default
  3,render,Phoenix.Controller,render/6,defp,Phoenix.Endpoint.RenderErrors,3,lib/endpoint/render_errors.ex,124,default
  ...
```

## Deeper Traversal

```bash
code_search --format toon trace MyApp.Web index --depth 10
```

## Trace with Arity Filter

```bash
code_search --format toon trace Phoenix.Controller render --arity 2 --depth 3
```

## Understanding Depth

- `depth: 1` - Direct calls from the starting function
- `depth: 2` - Calls from those callees
- `depth: 3` - And so on...

Each level shows what gets called at that depth in the call chain.

## Use Case: Understanding Error Handling

```bash
code_search --format toon trace Phoenix.Endpoint.RenderErrors "__catch__" --depth 5
```

This reveals the full error handling pipeline: catch → instrument → render → controller.

## Options Reference

| Argument/Option | Description | Default |
|-----------------|-------------|---------|
| `<MODULE>` | Starting module name (exact match or pattern with --regex) | required |
| `<FUNCTION>` | Starting function name (exact match or pattern with --regex) | required |
| `-a, --arity <N>` | Function arity (optional) | all arities |
| `--depth <N>` | Maximum depth to traverse (1-20) | 5 |
| `-r, --regex` | Treat patterns as regular expressions | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

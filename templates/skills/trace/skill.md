# trace - Examples

## Basic Forward Trace

```bash
code_search --format toon trace --module Phoenix.Endpoint.RenderErrors --function "__catch__" --depth 3
```

Output:
```
calls[N]{callee_arity,callee_function,callee_module,caller_function,caller_kind,caller_module,depth,file,line,project}:
  5,instrument_render_and_send,Phoenix.Endpoint.RenderErrors,__catch__/5,def,Phoenix.Endpoint.RenderErrors,1,lib/endpoint/render_errors.ex,62,default
  6,render,Phoenix.Endpoint.RenderErrors,instrument_render_and_send/5,defp,Phoenix.Endpoint.RenderErrors,2,lib/endpoint/render_errors.ex,85,default
  3,render,Phoenix.Controller,render/6,defp,Phoenix.Endpoint.RenderErrors,3,lib/endpoint/render_errors.ex,124,default
  ...
depth: 3
function_pattern: __catch__
module_pattern: Phoenix.Endpoint.RenderErrors
```

## Deeper Traversal

```bash
code_search --format toon trace --module MyApp.Web --function index --depth 10
```

## Trace with Arity Filter

```bash
code_search --format toon trace --module Phoenix.Controller --function render --arity 2 --depth 3
```

## Understanding Depth

- `depth: 1` - Direct calls from the starting function
- `depth: 2` - Calls from those callees
- `depth: 3` - And so on...

Each level shows what gets called at that depth in the call chain.

## Use Case: Understanding Error Handling

```bash
code_search --format toon trace --module Phoenix.Endpoint.RenderErrors --function "__catch__" --depth 5
```

This reveals the full error handling pipeline: catch → instrument → render → controller.

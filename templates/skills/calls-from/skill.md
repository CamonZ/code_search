---
name: calls-from
description: Show what a module/function calls (outgoing edges). Use to understand dependencies and trace execution flow forward.
---

# calls-from - Examples

## All Calls from a Module

```bash
code_search --format toon calls-from Phoenix.Endpoint.RenderErrors
```

Output:
```
calls[22]{call_type,callee_arity,callee_function,callee_module,caller_function,caller_kind,caller_module,file,line,project}:
  remote,2,accepts,Phoenix.Controller,put_formats/2,defp,Phoenix.Endpoint.RenderErrors,lib/endpoint/render_errors.ex,163,default
  remote,1,get_format,Phoenix.Controller,put_formats/2,defp,Phoenix.Endpoint.RenderErrors,lib/endpoint/render_errors.ex,166,default
  remote,3,render,Phoenix.Controller,render/6,defp,Phoenix.Endpoint.RenderErrors,lib/endpoint/render_errors.ex,124,default
  local,5,instrument_render_and_send,Phoenix.Endpoint.RenderErrors,__catch__/5,def,Phoenix.Endpoint.RenderErrors,lib/endpoint/render_errors.ex,62,default
  ...
```

## Calls from a Specific Function

```bash
code_search --format toon calls-from Phoenix.Controller render
```

## With Specific Arity

```bash
code_search --format toon calls-from Phoenix.Controller render 3
```

## Understanding Call Types

- `remote` - Cross-module call (e.g., `OtherModule.function()`)
- `local` - Same-module call (e.g., `helper_function()`)

## Tracing Error Handling Flow

```bash
code_search --format toon calls-from Phoenix.Endpoint.RenderErrors "__catch__"
```

This shows what happens when an error is caught - the error handling chain.

## Options Reference

| Argument/Option | Description | Default |
|-----------------|-------------|---------|
| `<MODULE>` | Module name (exact match or pattern with --regex) | required |
| `[FUNCTION]` | Function name (optional, shows all module calls if not specified) | none |
| `[ARITY]` | Function arity (optional) | all arities |
| `-r, --regex` | Treat patterns as regular expressions | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

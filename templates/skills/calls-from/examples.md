# calls-from - Examples

## All Calls from a Module

```bash
code_search --format toon calls-from --module Phoenix.Endpoint.RenderErrors
```

Output:
```
calls[22]{call_type,callee_arity,callee_function,callee_module,caller_function,caller_kind,caller_module,file,line,project}:
  remote,2,accepts,Phoenix.Controller,put_formats/2,defp,Phoenix.Endpoint.RenderErrors,lib/endpoint/render_errors.ex,163,default
  remote,1,get_format,Phoenix.Controller,put_formats/2,defp,Phoenix.Endpoint.RenderErrors,lib/endpoint/render_errors.ex,166,default
  remote,3,render,Phoenix.Controller,render/6,defp,Phoenix.Endpoint.RenderErrors,lib/endpoint/render_errors.ex,124,default
  local,5,instrument_render_and_send,Phoenix.Endpoint.RenderErrors,__catch__/5,def,Phoenix.Endpoint.RenderErrors,lib/endpoint/render_errors.ex,62,default
  ...
function_pattern: ""
module_pattern: Phoenix.Endpoint.RenderErrors
```

## Calls from a Specific Function

```bash
code_search --format toon calls-from --module Phoenix.Controller --function render
```

## With Specific Arity

```bash
code_search --format toon calls-from --module Phoenix.Controller --function render --arity 3
```

## Understanding Call Types

- `remote` - Cross-module call (e.g., `OtherModule.function()`)
- `local` - Same-module call (e.g., `helper_function()`)

## Tracing Error Handling Flow

```bash
code_search --format toon calls-from --module Phoenix.Endpoint.RenderErrors --function "__catch__"
```

This shows what happens when an error is caught - the error handling chain.

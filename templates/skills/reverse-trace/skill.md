# reverse-trace - Examples

## Find All Paths to a Function

```bash
code_search --format toon reverse-trace Phoenix.Controller render --depth 3
```

Output:
```
calls[18]{callee_arity,callee_function,callee_module,caller_function,caller_kind,caller_module,depth,file,line,project}:
  3,render,Phoenix.Controller,render/2,def,Phoenix.Controller,1,lib/controller.ex,873,default
  3,render,Phoenix.Controller,render/2,def,Phoenix.Controller,1,lib/controller.ex,877,default
  4,render,Phoenix.Controller,render/3,def,Phoenix.Controller,1,lib/controller.ex,966,default
  3,render,Phoenix.Controller,render/4,def,Phoenix.Controller,1,lib/controller.ex,975,default
  3,render,Phoenix.Controller,render/6,defp,Phoenix.Endpoint.RenderErrors,1,lib/render_errors.ex,124,default
    3,render,Phoenix.Controller,render/2,def,Phoenix.Controller,2,...
    5,instrument_render_and_send,Phoenix.Endpoint.RenderErrors,render/6,defp,...,2,...
      5,__catch__,Phoenix.Endpoint.RenderErrors,instrument_render_and_send/5,def,...,3,...
```

## Deeper Trace

```bash
code_search --format toon reverse-trace Ecto.Repo insert --depth 10
```

## Understanding the Output

Results are grouped by depth:
- `depth: 1` - Direct callers of the target
- `depth: 2` - Callers of those callers
- `depth: 3` - And so on...

This reveals the call chain: `__catch__ → instrument_render_and_send → render/6 → Controller.render`

## Use Case: Finding Entry Points

```bash
code_search --format toon reverse-trace MyApp.Repo get --depth 8
```

Traces backward to find all controller actions or API endpoints that eventually call `Repo.get`.

## Options Reference

| Argument/Option | Description | Default |
|-----------------|-------------|---------|
| `<MODULE>` | Target module name (exact match or pattern with --regex) | required |
| `<FUNCTION>` | Target function name (exact match or pattern with --regex) | required |
| `-a, --arity <N>` | Function arity (optional) | all arities |
| `--depth <N>` | Maximum depth to traverse (1-20) | 5 |
| `-r, --regex` | Treat patterns as regular expressions | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

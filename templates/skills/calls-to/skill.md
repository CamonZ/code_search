---
name: calls-to
description: Show what calls a module/function (incoming edges). Use to find callers and assess impact of changes.
---

# calls-to - Examples

## Find All Callers of a Function

```bash
code_search --format toon calls-to Mix.Phoenix copy_from
```

Output:
```
calls[14]{call_type,callee_arity,callee_function,callee_module,caller_function,caller_kind,caller_module,file,line,project}:
  remote,4,copy_from,Mix.Phoenix,copy_new_files/3,defp,Mix.Tasks.Phx.Gen.Auth,lib/mix/tasks/phx.gen.auth.ex,611,default
  remote,4,copy_from,Mix.Phoenix,run/1,def,Mix.Tasks.Phx.Gen.Channel,lib/mix/tasks/phx.gen.channel.ex,54,default
  remote,4,copy_from,Mix.Phoenix,copy_new_files/3,def,Mix.Tasks.Phx.Gen.Html,lib/mix/tasks/phx.gen.html.ex,216,default
  ...
```

## Find All Callers of a Module

```bash
code_search --format toon calls-to Phoenix.Controller
```

## Find Callers with Specific Arity

```bash
code_search --format toon calls-to Phoenix.Controller render 3
```

## Find Internal Recursive Calls

```bash
code_search --format toon calls-to Phoenix.Channel reply
```

Output shows `Phoenix.Channel.reply/2` calling itself (clause delegation):
```
calls[1]{...}:
  local,2,reply,Phoenix.Channel,reply/2,def,Phoenix.Channel,lib/phoenix/channel.ex,675,default
```

## Understanding the Output

Each call shows:
- `caller_kind`: `def`, `defp`, `defmacro`, `defmacrop`
- `caller_function`: Function making the call (with arity)
- `file:line`: Exact location of the call site
- `call_type`: `remote` (cross-module) or `local` (same-module)

## Options Reference

| Argument/Option | Description | Default |
|-----------------|-------------|---------|
| `<MODULE>` | Module name (exact match or pattern with --regex) | required |
| `[FUNCTION]` | Function name (optional, shows all module callers if not specified) | none |
| `[ARITY]` | Function arity (optional) | all arities |
| `-r, --regex` | Treat patterns as regular expressions | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

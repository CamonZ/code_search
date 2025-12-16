# location - Examples

## Find All Definitions of a Function

```bash
code_search --format toon location --function render
```

Output:
```
locations[15]{arity,end_line,file,guard,kind,module,name,pattern,project,start_line}:
  2,873,lib/phoenix/controller.ex,"is_binary(template) or is_atom(template)",def,Phoenix.Controller,render,"conn, template",default,872
  2,877,lib/phoenix/controller.ex,"",def,Phoenix.Controller,render,"conn, assigns",default,876
  3,951,lib/phoenix/controller.ex,"is_atom(template) and (is_map(assigns) or is_list(assigns))",def,Phoenix.Controller,render,"conn, template, assigns",default,944
  ...
function_pattern: render
module_pattern: null
```

## Find in Specific Module

```bash
code_search --format toon location --module Phoenix.Channel --function reply
```

Output:
```
locations[2]{arity,end_line,file,guard,kind,module,name,pattern,project,start_line}:
  2,675,lib/phoenix/channel.ex,"is_atom(status)",def,Phoenix.Channel,reply,"socket_ref, status",default,674
  2,679,lib/phoenix/channel.ex,"",def,Phoenix.Channel,reply,"{transport_pid, serializer, topic, ref, join_ref}, {status, payload}",default,678
function_pattern: reply
module_pattern: Phoenix.Channel
```

## Find with Specific Arity

```bash
code_search --format toon location --module Phoenix.Controller --function render --arity 3
```

## Regex Pattern for Multiple Functions

```bash
code_search --format toon location --function 'handle_.*' --regex --limit 20
```

## Understanding the Output

Each location shows:
- `pattern`: The function head arguments (e.g., `socket_ref, status`)
- `guard`: The `when` clause if present (e.g., `is_atom(status)`)
- `kind`: `def`, `defp`, `defmacro`, `defmacrop`
- `start_line:end_line`: Line range of the clause

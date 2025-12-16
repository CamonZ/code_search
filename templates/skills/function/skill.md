# function - Examples

## Get Function Signature

```bash
code_search --format toon function --module Phoenix.Controller --function render
```

Output:
```
functions[2]{args,arity,module,name,project,return_type}:
  "Plug.Conn.t(), Keyword.t() | map() | binary() | atom()",2,Phoenix.Controller,render,default,"Plug.Conn.t()"
  "Plug.Conn.t(), binary() | atom(), Keyword.t() | map()",3,Phoenix.Controller,render,default,"Plug.Conn.t()"
function_pattern: render
module_pattern: Phoenix.Controller
```

## Filter by Arity

```bash
code_search --format toon function --module Phoenix.Controller --function render --arity 2
```

Output:
```
functions[1]{args,arity,module,name,project,return_type}:
  "Plug.Conn.t(), Keyword.t() | map() | binary() | atom()",2,Phoenix.Controller,render,default,"Plug.Conn.t()"
```

## Regex Search for Multiple Functions

```bash
code_search --format toon function --module Phoenix.Controller --function 'put_.*' --regex
```

## Understanding the Output

- `args`: Comma-separated argument types from @spec
- `return_type`: Return type from @spec
- `arity`: Number of arguments

Note: This data comes from @spec definitions. Functions without specs won't appear.

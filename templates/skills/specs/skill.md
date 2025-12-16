# specs - Examples

## All Specs in a Module

```bash
code_search --format toon specs Phoenix.Controller
```

Output:
```
specs[27]{arity,definition,kind,line,module,name,project}:
  1,"@spec __info__(:attributes | :compile | :functions | ...) :: any()",spec,1,Phoenix.Controller,__info__,default
  2,"@spec accepts(Plug.Conn.t(), [binary()]) :: Plug.Conn.t()",spec,1520,Phoenix.Controller,accepts,default
  1,"@spec action_name(Plug.Conn.t()) :: atom()",spec,321,Phoenix.Controller,action_name,default
  2,"@spec json(Plug.Conn.t(), term()) :: Plug.Conn.t()",spec,362,Phoenix.Controller,json,default
  ...
module: Phoenix.Controller
```

## Filter by Function Name

```bash
code_search --format toon specs Phoenix.Controller --function render
```

Output:
```
specs[2]{arity,definition,kind,line,module,name,project}:
  2,"@spec render(Plug.Conn.t(), Keyword.t() | map() | binary() | atom()) :: Plug.Conn.t()",spec,869,Phoenix.Controller,render,default
  3,"@spec render(Plug.Conn.t(), binary() | atom(), Keyword.t() | map()) :: Plug.Conn.t()",spec,943,Phoenix.Controller,render,default
```

## Only Callbacks

```bash
code_search --format toon specs Phoenix.Channel --kind callback
```

## Regex Search Across Modules

```bash
code_search --format toon specs 'Phoenix\..*' --regex --limit 50
```

## Understanding Output

- `definition`: Full @spec or @callback text
- `kind`: `spec` or `callback`
- `line`: Line number where the spec is defined

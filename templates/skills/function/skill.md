# function - Examples

## Get Function Signature

```bash
code_search --format toon function Phoenix.Controller render
```

Output:
```
functions[2]{args,arity,module,name,project,return_type}:
  "Plug.Conn.t(), Keyword.t() | map() | binary() | atom()",2,Phoenix.Controller,render,default,"Plug.Conn.t()"
  "Plug.Conn.t(), binary() | atom(), Keyword.t() | map()",3,Phoenix.Controller,render,default,"Plug.Conn.t()"
```

## Filter by Arity

```bash
code_search --format toon function Phoenix.Controller render --arity 2
```

Output:
```
functions[1]{args,arity,module,name,project,return_type}:
  "Plug.Conn.t(), Keyword.t() | map() | binary() | atom()",2,Phoenix.Controller,render,default,"Plug.Conn.t()"
```

## Regex Search for Multiple Functions

```bash
code_search --format toon function Phoenix.Controller 'put_.*' --regex
```

## Understanding the Output

- `args`: Comma-separated argument types from @spec
- `return_type`: Return type from @spec
- `arity`: Number of arguments

Note: This data comes from @spec definitions. Functions without specs won't appear.

## Options Reference

| Argument/Option | Description | Default |
|-----------------|-------------|---------|
| `<MODULE>` | Module name (exact match or pattern with --regex) | required |
| `<FUNCTION>` | Function name (exact match or pattern with --regex) | required |
| `-a, --arity <N>` | Filter by specific arity | all arities |
| `-r, --regex` | Treat patterns as regular expressions | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

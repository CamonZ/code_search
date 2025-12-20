# search - Examples

## Find Modules by Name

```bash
code_search --format toon search Phoenix
```

Output:
```
modules[69]{name,project}:
  Inspect.Phoenix.Socket.Message,default
  Mix.Phoenix,default
  Phoenix,default
  Phoenix.Channel,default
  Phoenix.Controller,default
  ...
```

## Find Functions by Pattern

```bash
code_search --format toon search render --kind functions
```

Output:
```
functions[12]{args,module,name,project,return_type}:
  "Plug.Conn.t(), Keyword.t() | map() | binary() | atom()",Phoenix.Controller,render/2,default,"Plug.Conn.t()"
  "Plug.Conn.t(), binary() | atom(), Keyword.t() | map()",Phoenix.Controller,render/3,default,"Plug.Conn.t()"
  ...
```

## Regex Search for Module Prefix

```bash
code_search --format toon search '^Phoenix\.Channel' --regex
```

Output:
```
modules[3]{name,project}:
  Phoenix.Channel,default
  Phoenix.Channel.Server,default
  Phoenix.ChannelTest,default
```

## Search with Limit

```bash
code_search --format toon search Controller --limit 5
```

## Search in Specific Project

```bash
code_search --format toon search User --project my_app
```

## Options Reference

| Option | Description | Default |
|--------|-------------|---------|
| `-k, --kind <KIND>` | What to search for: `modules` or `functions` | `modules` |
| `-r, --regex` | Treat pattern as regular expression | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

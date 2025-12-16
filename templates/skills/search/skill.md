# search - Examples

## Find Modules by Name

```bash
code_search --format toon search --pattern Phoenix
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
pattern: Phoenix
```

## Find Functions by Pattern

```bash
code_search --format toon search --pattern render --kind functions
```

Output:
```
functions[12]{args,module,name,project,return_type}:
  "Plug.Conn.t(), Keyword.t() | map() | binary() | atom()",Phoenix.Controller,render/2,default,"Plug.Conn.t()"
  "Plug.Conn.t(), binary() | atom(), Keyword.t() | map()",Phoenix.Controller,render/3,default,"Plug.Conn.t()"
  ...
kind: functions
pattern: render
```

## Regex Search for Module Prefix

```bash
code_search --format toon search --pattern '^Phoenix\.Channel' --regex
```

Output:
```
modules[3]{name,project}:
  Phoenix.Channel,default
  Phoenix.Channel.Server,default
  Phoenix.ChannelTest,default
pattern: ^Phoenix\.Channel
```

## Search with Limit

```bash
code_search --format toon search --pattern Controller --limit 5
```

## Search in Specific Project

```bash
code_search --format toon search --pattern User --project my_app
```

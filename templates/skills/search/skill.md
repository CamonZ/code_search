# search - Examples

## Find Modules by Name

```bash
code_search --format toon search Phoenix
```

Output:
```
modules[69]{name,source}:
  Inspect.Phoenix.Socket.Message,unknown
  Mix.Phoenix,unknown
  Phoenix,unknown
  Phoenix.Channel,unknown
  Phoenix.Controller,unknown
  ...
```

## Find Functions by Pattern

```bash
code_search --format toon search render --kind functions
```

Output:
```
functions[12]{arity,module,name,return_type}:
  2,Phoenix.Controller,render,""
  3,Phoenix.Controller,render,""
  ...
```

## Regex Search for Module Prefix

```bash
code_search --format toon search '^Phoenix\.Channel' --regex
```

Output:
```
modules[3]{name,source}:
  Phoenix.Channel,unknown
  Phoenix.Channel.Server,unknown
  Phoenix.ChannelTest,unknown
```

## Search with Limit

```bash
code_search --format toon search Controller --limit 5
```

## Options Reference

| Option | Description | Default |
|--------|-------------|---------|
| `-k, --kind <KIND>` | What to search for: `modules` or `functions` | `modules` |
| `-r, --regex` | Treat pattern as regular expression | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |

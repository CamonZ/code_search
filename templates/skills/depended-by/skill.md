# depended-by

Show what modules depend on a given module (incoming module dependencies).

## Purpose

Find all modules that call into a given module. This shows who relies on this module, useful for impact analysis.

## Usage

```bash
code_search --format toon depended-by --module <MODULE> [OPTIONS]
```

## Required Options

| Option | Description |
|--------|-------------|
| `-m, --module <MODULE>` | Module to analyze |

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-r, --regex` | Treat module as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
dependents[N]{call_count,module,project}:
  11,Phoenix.Endpoint.RenderErrors,default
  5,Phoenix.ConnTest,default
```

## When to Use

- Impact analysis: who will be affected by changes
- Finding consumers of a module
- Understanding how widely used a module is
- Architecture review: identifying core vs peripheral modules

## See Also

- [examples.md](examples.md) for detailed usage examples
- `depends-on` - Reverse: what does this module depend on
- `calls-to` - Function-level incoming calls

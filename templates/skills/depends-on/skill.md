# depends-on

Show what modules a given module depends on (outgoing module dependencies).

## Purpose

Find all modules that a given module calls into. This is a module-level view of dependencies, aggregating all function calls.

## Usage

```bash
code_search --format toon depends-on --module <MODULE> [OPTIONS]
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
dependencies[N]{call_count,module,project}:
  6,Phoenix.Channel.Server,default
  2,Phoenix.PubSub,default
```

## When to Use

- Understanding module coupling
- Finding external dependencies
- Analyzing architecture: what does this module rely on?
- Identifying tightly coupled modules

## See Also

- [examples.md](examples.md) for detailed usage examples
- `depended-by` - Reverse: who depends on this module
- `calls-from` - Function-level outgoing calls

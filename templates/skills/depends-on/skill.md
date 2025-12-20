# depends-on - Examples

## Find Module Dependencies

```bash
code_search --format toon depends-on Phoenix.Channel
```

Output:
```
dependencies[1]{call_count,module,project}:
  6,Phoenix.Channel.Server,default
```

## Find Dependencies of Multiple Modules

```bash
code_search --format toon depends-on 'Phoenix\.Controller.*' --regex
```

## Understanding the Output

- `module`: The module being depended on
- `call_count`: Number of calls from source to this module

Higher call counts indicate stronger coupling.

## Use Case: Architecture Analysis

Check what a core module depends on:
```bash
code_search --format toon depends-on MyApp.Accounts
```

This reveals:
- Database access patterns (Repo calls)
- External service integrations
- Shared utility usage

## Options Reference

| Argument/Option | Description | Default |
|-----------------|-------------|---------|
| `<MODULE>` | Module name (exact match or pattern with --regex) | required |
| `-r, --regex` | Treat patterns as regular expressions | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

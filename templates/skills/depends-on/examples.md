# depends-on - Examples

## Find Module Dependencies

```bash
code_search --format toon depends-on --module Phoenix.Channel
```

Output:
```
dependencies[1]{call_count,module,project}:
  6,Phoenix.Channel.Server,default
module_pattern: Phoenix.Channel
```

## Find Dependencies of Multiple Modules

```bash
code_search --format toon depends-on --module 'Phoenix\.Controller.*' --regex
```

## Understanding the Output

- `module`: The module being depended on
- `call_count`: Number of calls from source to this module

Higher call counts indicate stronger coupling.

## Use Case: Architecture Analysis

Check what a core module depends on:
```bash
code_search --format toon depends-on --module MyApp.Accounts
```

This reveals:
- Database access patterns (Repo calls)
- External service integrations
- Shared utility usage

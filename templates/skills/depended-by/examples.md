# depended-by - Examples

## Find Dependents of a Module

```bash
code_search --format toon depended-by --module Phoenix.Controller
```

Output:
```
dependents[3]{call_count,module,project}:
  11,Phoenix.Endpoint.RenderErrors,default
  5,Phoenix.ConnTest,default
  1,Phoenix.Token,default
module_pattern: Phoenix.Controller
```

## Find Dependents with Regex

```bash
code_search --format toon depended-by --module 'Ecto\..*' --regex
```

## Understanding the Output

- `module`: The module that depends on the target
- `call_count`: Number of calls from that module to target

## Use Case: Impact Analysis

Before changing `Phoenix.Controller`:
```bash
code_search --format toon depended-by --module Phoenix.Controller
```

Shows 3 modules with 17 total calls would be affected.

## Use Case: Finding Core Modules

Modules with many dependents are core infrastructure:
```bash
code_search --format toon depended-by --module MyApp.Repo
```

If many modules depend on Repo, it's a central piece of the architecture.

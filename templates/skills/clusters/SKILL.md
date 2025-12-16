---
name: clusters
description: Analyze module connectivity using namespace-based clustering. Use to understand system organization and identify coupled module groups.
---

# clusters

Analyze module connectivity using namespace-based clustering.

## Purpose

Group modules by their namespace/prefix to understand how different parts of the system interact. Use this to identify tightly coupled module groups and understand system organization.

## Usage

```bash
code_search --format toon clusters [OPTIONS]
```

## Optional Flags

| Option | Description | Default |
|--------|-------------|---------|
| `-m, --module <PATTERN>` | Module pattern to filter | all |
| `-r, --regex` | Treat patterns as regex | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

## Output Fields (toon format)

```
clusters[N]{cluster_name,module_count,total_connections,modules[N]{incoming,outgoing,name,total}}:
  MyApp.Web,5,45,MyApp.Web.Controller,12,8,MyApp.Web.Controller,20
  MyApp.Domain,3,23,MyApp.Domain.User,8,6,MyApp.Domain.User,14
```

## When to Use

- Understanding system organization by namespace
- Identifying tightly coupled module groups
- Planning refactoring and module reorganization
- Analyzing architectural boundaries

## See Also

- [examples.md](examples.md) for detailed usage examples
- `depends-on` - See module dependencies
- `hotspots` - Find high-connectivity functions
- `boundaries` - Find architectural boundary modules

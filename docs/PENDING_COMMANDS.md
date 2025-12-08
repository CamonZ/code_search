# Pending Commands

Commands not yet implemented in the CLI.

| Command | Description |
|---------|-------------|
| `types` | Show detailed type signatures with all clause variants |
| `returns` | Find functions that return a specific type pattern |
| `accepts` | Find functions that accept a specific type pattern |
| `struct-usage` | Find all functions that use a specific struct |
| `genservers` | List all modules implementing GenServer callbacks |
| `callbacks` | Show which OTP callbacks a module implements |
| `message-handlers` | Extract message patterns from handle_call/cast/info |
| `complexity` | Show complexity metrics for a module |
| `large-functions` | Find functions with many clauses |
| `cycles` | Find circular dependencies between modules |
| `clusters` | Group modules by coupling |
| `boundaries` | Identify boundary modules |
| `god-modules` | Find modules with too many functions/dependencies |
| `dynamic-typed` | Find functions with mostly dynamic() types |
| `struct-graph` | Show relationships between structs |
| `struct-modules` | Show which modules create/manipulate each struct |

## Covered by Existing Commands

These were originally planned but are already supported:

| Original Idea | Use Instead |
|---------------|-------------|
| `call-sites` | `calls-to -m Module -f function` — shows all call locations with file/line/column |
| `orphan-functions` | `unused --private-only` — finds private functions never called |
| `wide-modules` | `hotspots -k outgoing` — modules/functions with high fan-out |
| `deep-modules` | `hotspots -k incoming` — modules/functions with high fan-in |
| `entry-points` | `unused --public-only` (inverted) — public functions with zero internal callers |

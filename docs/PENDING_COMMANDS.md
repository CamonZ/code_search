# Pending Commands

Commands not yet implemented in the CLI.

| Command | Description |
|---------|-------------|
| `unused` | Find functions that are never called |
| `modules` | List all modules (optionally filtered by pattern) |
| `hotspots` | Find functions with most incoming/outgoing calls |
| `file` | Show all functions defined in a file |
| `types` | Show detailed type signatures with all clause variants |
| `returns` | Find functions that return a specific type pattern |
| `accepts` | Find functions that accept a specific type pattern |
| `struct-usage` | Find all functions that use a specific struct |
| `genservers` | List all modules implementing GenServer callbacks |
| `callbacks` | Show which OTP callbacks a module implements |
| `message-handlers` | Extract message patterns from handle_call/cast/info |
| `complexity` | Show complexity metrics for a module |
| `large-functions` | Find functions with many clauses |
| `wide-modules` | Find modules with high fan-out |
| `deep-modules` | Find modules with high fan-in |
| `cycles` | Find circular dependencies between modules |
| `clusters` | Group modules by coupling |
| `boundaries` | Identify boundary modules |
| `god-modules` | Find modules with too many functions/dependencies |
| `orphan-functions` | Find private functions never called internally |
| `dynamic-typed` | Find functions with mostly dynamic() types |
| `struct-graph` | Show relationships between structs |
| `struct-modules` | Show which modules create/manipulate each struct |
| `call-sites` | Show all locations where a function is called |
| `entry-points` | Find functions called externally but not internally |

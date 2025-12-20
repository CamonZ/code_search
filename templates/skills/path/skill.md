# path - Examples

## Find Path Between Two Functions

```bash
code_search --format toon path \
  --from-module MyApp.Web.UserController \
  --from-function create \
  --to-module Ecto.Repo \
  --to-function insert
```

Output (if path exists):
```
paths[1]{path}:
  [["MyApp.Web.UserController","create",2],["MyApp.Accounts","create_user",1],["MyApp.Repo","insert",2]]
from_arity: null
from_function: create
from_module: MyApp.Web.UserController
to_arity: null
to_function: insert
to_module: Ecto.Repo
```

## No Path Found

```bash
code_search --format toon path \
  --from-module Phoenix.Channel \
  --from-function join \
  --to-module Mix.Tasks.Compile \
  --to-function run
```

Output:
```
paths[0]{path}:
...
```

Empty results means no path exists within the search depth.

## With Specific Arities

```bash
code_search --format toon path \
  --from-module MyApp.API --from-function handle --from-arity 2 \
  --to-module MyApp.Repo --to-function get --to-arity 2 \
  --depth 15
```

## Understanding Paths

Each path is a list of `[module, function, arity]` tuples showing the call chain:
1. Source function
2. Intermediate functions (in order)
3. Target function

## Options Reference

| Option | Description | Default |
|--------|-------------|---------|
| `--from-module <MODULE>` | Source module name | required |
| `--from-function <FUNCTION>` | Source function name | required |
| `--from-arity <N>` | Source function arity | all arities |
| `--to-module <MODULE>` | Target module name | required |
| `--to-function <FUNCTION>` | Target function name | required |
| `--to-arity <N>` | Target function arity | all arities |
| `--depth <N>` | Maximum depth to search (1-20) | 10 |
| `-l, --limit <N>` | Max paths to return (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

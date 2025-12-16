# hotspots - Examples

## Find Most Called Functions

```bash
code_search --format toon hotspots
```

Output:
```
hotspots[20]{function,incoming,module,outgoing,total}:
  web_path,20,Mix.Phoenix,0,20
  expand_alias,16,Phoenix.Router,0,16
  copy_from,14,Mix.Phoenix,0,14
  json_library,14,Phoenix,0,14
  eval_from,12,Mix.Phoenix,0,12
  ...
kind: incoming
module_filter: null
```

## Find Functions with High Fan-Out

```bash
code_search --format toon hotspots --kind outgoing
```

Functions that call many other functions - potential god functions to refactor.

## Find Total Connections

```bash
code_search --format toon hotspots --kind total
```

## Filter to Specific Module Namespace

```bash
code_search --format toon hotspots --module Phoenix.Router
```

## Understanding the Output

- `incoming`: How many places call this function
- `outgoing`: How many functions this calls
- `total`: Sum of incoming + outgoing

## Use Cases

**Find Core Utilities:**
```bash
code_search --format toon hotspots --kind incoming --limit 10
```

**Find Complex Functions (high fan-out):**
```bash
code_search --format toon hotspots --kind outgoing --limit 10
```

**Find Coupling Hotspots:**
```bash
code_search --format toon hotspots --kind total --limit 10
```

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

## Find Boundary Functions

```bash
code_search --format toon hotspots --kind ratio
```

Functions with high incoming/outgoing ratio - these are API boundaries.

## Filter to Specific Module Namespace

```bash
code_search --format toon hotspots Phoenix.Router
```

## Understanding the Output

- `incoming`: How many places call this function
- `outgoing`: How many functions this calls
- `total`: Sum of incoming + outgoing

## Use Cases

**Find Core Utilities:**
```bash
code_search --format toon hotspots --kind incoming -l 10
```

**Find Complex Functions (high fan-out):**
```bash
code_search --format toon hotspots --kind outgoing -l 10
```

**Find Coupling Hotspots:**
```bash
code_search --format toon hotspots --kind total -l 10
```

**Find Boundary Functions:**
```bash
code_search --format toon hotspots --kind ratio -l 10
```

## Options Reference

| Argument/Option | Description | Default |
|-----------------|-------------|---------|
| `[MODULE]` | Module pattern to filter results (substring or regex with -r) | all modules |
| `-k, --kind <KIND>` | Type of hotspots: `incoming`, `outgoing`, `total`, `ratio` | `incoming` |
| `--exclude-generated` | Exclude macro-generated functions | false |
| `-r, --regex` | Treat patterns as regular expressions | false |
| `-l, --limit <N>` | Max results (1-1000) | 100 |
| `--project <NAME>` | Project to search in | `default` |

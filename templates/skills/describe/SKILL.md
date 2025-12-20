---
name: describe
description: Get detailed information about available commands and their usage. Use this to discover what the code_search tool can do and how to use specific commands.
---

# describe

Get detailed information about available commands and their usage.

## Purpose

List all available commands or get detailed documentation for specific commands. Use this to understand what the code_search tool can do and how to use specific commands.

## Usage

```bash
code_search --format toon describe [COMMANDS]...
```

## Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `[COMMANDS]...` | Command(s) to describe (if empty, lists all) | all commands |

## Examples

```bash
code_search describe                             # List all available commands
code_search describe calls-to                    # Detailed info about calls-to command
code_search describe calls-to calls-from trace   # Describe multiple commands
```

## Output Fields (toon format)

For command listing:
```
categories[N]{category,commands[N]{brief,name}}:
  Query Commands,calls-to Find callers of a given function,calls-to
  Analysis Commands,hotspots Find high-connectivity functions,hotspots
```

For specific command details:
```
description: Find callers of a given function
examples[N]{command,description}:
  code_search calls-to MyApp.Repo get,Find all callers
name: calls-to
related[N]: calls-from
usage: code_search calls-to <MODULE> [FUNCTION] [ARITY]
```

## When to Use

- Discovering what commands are available
- Learning how to use a specific command
- Understanding command parameters and output formats
- Getting usage examples for commands

## See Also

- Individual command documentation (e.g., `calls-to`, `hotspots`)

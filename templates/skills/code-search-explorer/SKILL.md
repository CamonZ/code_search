---
name: code-search-explorer
description: Fast exploration of Elixir/Erlang codebases using the code_search CLI tool and static call graph analysis. Find definitions, trace calls, analyze dependencies, discover dead code, detect code smells, and explore GenServer patterns. Powered by Haiku for efficient analysis.
---

# Code Search Explorer

Explore Elixir/Erlang codebases through static call graph analysis using the `code_search` CLI tool.

> **Note**: This skill uses the `code-search-explorer` agent powered by Haiku for fast, cost-efficient exploration.

## When to Use This Skill

Use the code-search-explorer when you need to:

- **Find definitions**: "Where is the authenticate function defined?"
- **Trace calls**: "What calls this function?" or "What does this function call?"
- **Understand flow**: "How do we get from controller action to database?"
- **Analyze dependencies**: "What modules does this depend on?"
- **Find dead code**: "What functions are never called?"
- **Check quality**: "Which modules are too complex or coupled?"
- **Explore structure**: "What does this module contain?"
- **Impact analysis**: "What will break if I change this?"

## Prerequisites

The codebase must have a call graph extracted and imported:

1. **Extract call graph** using [ex_ast](https://github.com/CamonZ/ex_ast):
   ```bash
   # In your Elixir project
   mix ex_ast.extract --output call_graph.json
   ```

2. **Import into database**:
   ```bash
   code_search setup
   code_search import --file call_graph.json
   ```

3. **Verify data**:
   ```bash
   code_search describe
   ```

## Quick Examples

### Finding Function Definitions

```
Where is the process_payment function defined?
```

**What happens**: Agent runs `code_search --format toon location process_payment` and shows you the file path and line number.

### Understanding Call Relationships

```
What functions call create_user?
```

**What happens**: Agent runs `code_search --format toon calls-to --function create_user` and lists all callers with locations.

### Tracing Execution Flow

```
Trace the execution path from handle_request in ApiController
```

**What happens**: Agent runs `code_search --format toon trace ApiController handle_request --depth 3` showing the full call tree.

### Module Analysis

```
Show me everything in the Authentication module
```

**What happens**: Agent runs `code_search --format toon browse-module Authentication` listing all public functions, types, and specs.

### Dependency Analysis

```
What modules does PaymentGateway depend on?
```

**What happens**: Agent runs `code_search --format toon depends-on PaymentGateway` showing direct and transitive dependencies.

### Finding Dead Code

```
Find unused functions in this codebase
```

**What happens**: Agent runs `code_search --format toon unused` listing functions with zero callers.

### Code Quality Checks

```
Which modules are too large or complex?
```

**What happens**: Agent runs multiple commands like `god-modules`, `complexity`, `large-functions` to identify problematic code.

## Command Categories

The `code_search` tool has several command categories:

| Category | Commands | Use Cases |
|----------|----------|-----------|
| **Discovery** | `search`, `browse-module`, `describe` | Find modules/functions, explore interfaces |
| **Location** | `location`, `function` | Find where things are defined |
| **Call Graph** | `calls-from`, `calls-to`, `trace`, `reverse-trace`, `path` | Navigate call relationships |
| **Dependencies** | `depends-on`, `depended-by`, `clusters`, `cycles` | Analyze module coupling |
| **Types** | `accepts`, `returns`, `struct-usage` | Type-based queries |
| **Quality** | `unused`, `duplicates`, `hotspots`, `god-modules`, `complexity`, `large-functions`, `many-clauses`, `boundaries` | Identify code smells |

## Common Questions → Commands

| Question | Command |
|----------|---------|
| Where is function X? | `location <function>` |
| What's in module X? | `browse-module <module>` |
| What calls X? | `calls-to --function <name>` |
| What does X call? | `calls-from <module> <function>` |
| How to get from A to B? | `path --from-module A --to-module B` |
| What depends on X? | `depended-by <module>` |
| What does X depend on? | `depends-on <module>` |
| Any unused code? | `unused` |
| Most called functions? | `hotspots --kind incoming` |
| Circular dependencies? | `cycles` |
| Too large modules? | `god-modules` |
| Complex functions? | `complexity` |

## Workflows

### Understanding a New Codebase

1. Get overview: "List the main modules"
2. Find entry points: "Show me the most-called functions"
3. Explore core: "Show me the UserManager module"
4. Check architecture: "Find architectural boundaries"

### Before Refactoring

1. Find impact: "What calls this function?"
2. Reverse trace: "What depends on this module?"
3. Check coupling: "Show module dependencies"
4. Assess risk: "Find all places this is used"

### Code Quality Audit

1. Find smells: "Show me god modules"
2. Check complexity: "Find complex functions"
3. Discover dead code: "List unused functions"
4. Identify cycles: "Are there circular dependencies?"

### Debugging/Understanding Flow

1. Find entry point: "Where does the request start?"
2. Trace execution: "Trace from handle_request"
3. Find implementation: "Where is this function defined?"
4. Check callers: "What calls this?"

## Output Formats

The agent uses `--format toon` for token efficiency, but you can also run commands directly:

- **Table** (default): Human-readable output
  ```bash
  code_search location authenticate
  ```

- **JSON**: For scripts/tools
  ```bash
  code_search --format json location authenticate
  ```

- **Toon**: Token-efficient for LLMs (what the agent uses)
  ```bash
  code_search --format toon location authenticate
  ```

## Database Configuration

Database is automatically searched in this order:
1. `.code_search/cozo.sqlite` (project-local, created by default)
2. `./cozo.sqlite` (current directory)
3. `~/.code_search/cozo.sqlite` (user-global)

Override with `--db` flag if needed:
```bash
code_search --db /path/to/project.sqlite <command>
```

## Tips for Best Results

1. **Be specific**: "Find the authenticate function in AuthController" is better than "find authenticate"
2. **Use context**: "Trace calls from the Phoenix controller" helps narrow results
3. **Iterate**: Start broad, then drill down based on findings
4. **Verify**: Ask agent to read source files for details after finding locations
5. **Use filters**: `--project`, `--module`, `--limit` help focus results

## Limitations

- Only analyzes static call graphs (no runtime behavior)
- Private function (defp) calls from outside modules won't appear
- Macros expanded at compile time may not show in graph
- Dynamic calls (apply, send, etc.) may be incomplete
- GenServer callbacks need manual interpretation

## Manual Usage

You can also run `code_search` directly:

```bash
# Get help
code_search --help
code_search <command> --help

# Common queries
code_search location my_function
code_search browse-module MyApp.Module
code_search calls-to --function process
code_search trace MyModule process --depth 2
code_search depends-on MyApp.Core
code_search unused --project my_app
code_search hotspots --kind incoming --limit 20
code_search god-modules
```

## Related Resources

- **Agent**: `.claude/agents/code-search-explorer.md` - The Haiku-powered agent
- **Templates**: `templates/skills/` - Individual command documentation
- **Workflows**: `templates/skills/workflows/` - Detailed workflow guides
- **Tool**: `code_search --help` - CLI documentation

## Examples in Practice

### Example 1: Understanding Authentication Flow

**Question**: "How does authentication work in this app?"

**Agent workflow**:
1. `code_search --format toon search auth` → Find auth-related modules
2. `code_search --format toon browse-module AuthController` → See public interface
3. `code_search --format toon trace AuthController login` → Follow login flow
4. Read relevant source files for implementation details
5. Summarize the authentication flow

### Example 2: Impact Analysis Before Deletion

**Question**: "Can I safely delete the User.send_notification function?"

**Agent workflow**:
1. `code_search --format toon calls-to --function send_notification` → Find callers
2. `code_search --format toon reverse-trace User send_notification` → Full reverse tree
3. Report: "This function is called from 3 places: OrderController, AlertService, and AdminPanel"
4. User can decide based on impact

### Example 3: Finding Dead Code

**Question**: "What code can I delete from the Reports module?"

**Agent workflow**:
1. `code_search --format toon unused --module Reports` → Find unused functions
2. `code_search --format toon browse-module Reports` → Show all functions
3. Compare and identify private functions with zero callers
4. Report candidates for deletion with file paths

## Troubleshooting

**Issue**: "Database not found"
- **Solution**: Run `code_search setup` first (creates `.code_search/cozo.sqlite`)

**Issue**: "No results found"
- **Solution**: Check if data is imported with `code_search describe`
- **Solution**: Try broader search terms or remove filters

**Issue**: "Too many results"
- **Solution**: Use `--limit`, `--project`, or `--module` to narrow results
- **Solution**: Be more specific in function/module names

**Issue**: "Function not found but I know it exists"
- **Solution**: Check if it's a private function (defp)
- **Solution**: Verify the call graph data is up-to-date

## See Also

- `/code-graph` skill - Alternative query interface
- Individual command skills in `templates/skills/*/SKILL.md`
- Workflow guides in `templates/skills/workflows/`

---
name: structural-code-analysis
description: Analyze Elixir/Erlang codebases using static call graph analysis. Use this skill to understand application architecture, trace execution flows, find dependencies, discover dead code, and assess code quality.
---

# Structural Code Analysis

Analyze Elixir/Erlang codebases through static call graph analysis stored in CozoDB.

## When to Use This Skill

Use structural code analysis when you need to:
- Understand how an unfamiliar codebase is organized
- Trace execution flow through the application
- Find where functions are defined and who calls them
- Analyze module dependencies and coupling
- Discover dead code and refactoring opportunities
- Assess code quality and architectural health

## Prerequisites

The codebase must have a call graph extracted and imported:
```bash
# 1. Extract call graph from Elixir project (using code_intelligence_tracer)
# 2. Import into database
code_search import --file call_graph.json
```

## Core Concepts

### Data Model

The call graph captures:
- **Modules**: Elixir/Erlang modules with their source files
- **Functions**: Function definitions with arity, visibility (def/defp), location
- **Calls**: Edges between functions (caller → callee) with call site locations
- **Specs**: @spec type signatures (argument types, return types)
- **Types**: @type/@typep/@opaque definitions
- **Structs**: Struct definitions with field names, defaults, and inferred types

### Command Categories

| Category | Commands | Purpose |
|----------|----------|---------|
| **Discovery** | `search`, `browse-module`, `describe` | Find modules/functions, explore interfaces |
| **Location** | `location`, `function` | Find definitions, get signatures |
| **Call Graph** | `calls-from`, `calls-to`, `trace`, `reverse-trace`, `path` | Navigate call relationships |
| **Dependencies** | `depends-on`, `depended-by`, `clusters`, `cycles` | Module-level dependencies |
| **Types** | `accepts`, `returns`, `struct-usage` | Type-based analysis |
| **Code Smells** | `unused`, `duplicates`, `hotspots`, `god-modules`, `large-functions`, `many-clauses`, `complexity`, `boundaries` | Quality analysis |

## Question → Command Mapping

| Question | Command |
|----------|---------|
| "Where is function X defined?" | `location <function>` |
| "What does module X contain?" | `browse-module <module>` |
| "What does function X call?" | `calls-from <module> <function>` |
| "What calls function X?" | `calls-to <module> <function>` |
| "How do we get from A to B?" | `path --from-module A --from-function a --to-module B --to-function b` |
| "What modules does X depend on?" | `depends-on <module>` |
| "What depends on module X?" | `depended-by <module>` |
| "What functions accept type T?" | `accepts <type-pattern>` |
| "What functions return type T?" | `returns <type-pattern>` |
| "What functions are never called?" | `unused` |
| "What are the most called functions?" | `hotspots` |
| "Are there circular dependencies?" | `cycles` |
| "Which modules are too large?" | `god-modules` |

## Common Workflows

### Understanding a New Codebase

1. **Get an overview of modules**:
   ```bash
   code_search search "" --limit 50  # List top modules
   ```

2. **Identify architectural boundaries**:
   ```bash
   code_search boundaries
   code_search clusters
   ```

3. **Find entry points** (most-called public functions):
   ```bash
   code_search hotspots --kind incoming
   ```

4. **Explore a core module**:
   ```bash
   code_search browse-module MyApp.Core
   code_search depends-on MyApp.Core
   code_search depended-by MyApp.Core
   ```

### Tracing Execution Flow

See: [workflows/trace-execution-flow.md](../workflows/trace-execution-flow.md)

1. Start from an entry point (controller action, GenServer callback)
2. Use `trace` to follow forward call chains
3. Use `calls-from` for single-level exploration
4. Use `path` to find how two functions connect

### Understanding a Feature

See: [workflows/understand-feature.md](../workflows/understand-feature.md)

1. Search for related modules/functions by name
2. Browse the main module to see its interface
3. Trace dependencies up and down
4. Map the data flow through type analysis

### Impact Analysis (Before Refactoring)

See: [workflows/impact-analysis.md](../workflows/impact-analysis.md)

1. Find all callers with `calls-to` and `reverse-trace`
2. Check module-level impact with `depended-by`
3. Identify the blast radius before making changes

### Finding Dead Code

See: [workflows/find-dead-code.md](../workflows/find-dead-code.md)

1. Find unused functions with `unused`
2. Focus on private functions (definitely dead): `unused -p`
3. Check for unused modules (no incoming dependencies)

### Code Quality Audit

See: [workflows/code-quality-audit.md](../workflows/code-quality-audit.md)

1. Find god modules: `god-modules`
2. Find complex functions: `complexity`, `large-functions`, `many-clauses`
3. Check for cycles: `cycles`
4. Identify coupling hotspots: `hotspots --kind total`
5. Find duplication: `duplicates`

## Output Format

All commands support three output formats:

| Format | Flag | Use Case |
|--------|------|----------|
| Table | `--format table` | Human-readable, default |
| JSON | `--format json` | Programmatic processing |
| Toon | `--format toon` | Token-efficient for LLMs |

For LLM consumption, always use `--format toon`:
```bash
code_search --format toon <command> [args]
```

## Database Location

Default: `./cozo.sqlite`

Override with `--db`:
```bash
code_search --db /path/to/project.sqlite <command>
```

## See Also

- Individual command skills in `templates/skills/<command>/skill.md`
- Workflow guides in `templates/skills/workflows/`

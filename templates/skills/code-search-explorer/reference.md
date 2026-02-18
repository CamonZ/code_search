# Code Search CLI Quick Reference

## Essential Commands

### Discovery & Search
```bash
code_search code search <pattern>              # Find modules/functions by name
code_search code browse-module <module>        # Show module contents
code_search code location <function>           # Find where function is defined
code_search code function <module> <function>  # Get function details
```

### Call Graph Navigation
```bash
code_search code calls-from <module> <function>          # What does X call?
code_search code calls-to --function <name>              # What calls X?
code_search code trace <module> <function> --depth N     # Forward call tree
code_search code reverse-trace <module> <function>       # Backward call tree
code_search code path --from-module A --to-module B      # Find call path A→B
```

### Module Dependencies
```bash
code_search code depends-on <module>           # What does module depend on?
code_search code depended-by <module>          # What depends on module?
code_search code clusters                      # Find module clusters
code_search code cycles                        # Find circular dependencies
code_search code boundaries                    # Identify architectural boundaries
```

### Code Quality
```bash
code_search code unused                        # Find unused functions
code_search code hotspots --kind <type>        # Find coupling hotspots
code_search code god-modules                   # Find overly large modules
code_search code complexity                    # Find complex functions
code_search code large-functions               # Find long functions
code_search code many-clauses                  # Functions with many clauses
code_search code duplicates                    # Find duplicate signatures
```

### Type Analysis
```bash
code_search code accepts <type-pattern>        # Functions accepting a type
code_search code returns <type-pattern>        # Functions returning a type
code_search code struct-usage <struct>         # Where is struct used?
```

## Global Flags

```bash
--db <path>                # Database location (auto-resolves: .code_search/, ./, ~/.code_search/)
--format <fmt>             # Output format: table|json|toon
```

## Common Filter Flags

```bash
--module <pattern>         # Filter by module pattern (regex)
--regex                    # Treat module pattern as regex
--limit <N>                # Limit results
--depth <N>                # Trace depth (for trace commands)
```

## Hotspot Kinds

```bash
--kind incoming            # Most-called functions
--kind outgoing            # Functions that call many others
--kind total               # Highest total coupling (incoming + outgoing)
```

## Output Format Examples

### Table (Human-readable)
```bash
code_search code location authenticate
# Shows: module, function, file, line in a table
```

### JSON (Programmatic)
```bash
code_search --format json code location authenticate
# Returns structured JSON
```

### Toon (Token-efficient for LLMs)
```bash
code_search --format toon code location authenticate
# Returns compact format:
# results[1]:
#   module: MyApp.Auth
#   function: authenticate
#   file: lib/my_app/auth.ex
#   line: 42
```

## Typical Workflows

### Find and Explore a Function
```bash
# 1. Find it
code_search code location authenticate

# 2. See what it calls
code_search code calls-from MyApp.Auth authenticate

# 3. See what calls it
code_search code calls-to --function authenticate

# 4. Trace its execution
code_search code trace MyApp.Auth authenticate --depth 2
```

### Module Impact Analysis
```bash
# 1. What does it contain?
code_search code browse-module MyApp.Payment

# 2. What does it depend on?
code_search code depends-on MyApp.Payment

# 3. What depends on it?
code_search code depended-by MyApp.Payment

# 4. Any circular deps?
code_search code cycles
```

### Code Quality Audit
```bash
# Find all quality issues
code_search code unused
code_search code god-modules
code_search code complexity --limit 20
code_search code large-functions --limit 10
code_search code cycles
code_search code hotspots --kind total --limit 15
```

## Tips

1. **Start broad, then narrow**: Begin with `search` or `browse-module`, then drill down
2. **Use --format toon for agents**: More efficient for LLM processing
3. **Limit results**: Add `--limit` for large codebases
4. **Use regex for patterns**: Add `--regex` flag for complex module patterns

## Setup & Import

```bash
# Create database schema (creates .code_search/surrealdb.rocksdb)
code_search setup

# Import call graph data (from ex_ast)
code_search code import --file call_graph.json
```

## Need More Help?

```bash
code_search --help                    # General help
code_search code <command> --help          # Command-specific help
```

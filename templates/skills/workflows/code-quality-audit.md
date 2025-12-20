# Workflow: Code Quality Audit

Systematically assess codebase health by identifying architectural issues, complexity hotspots, and improvement opportunities.

## When to Use

- Technical debt assessment: "How healthy is this codebase?"
- Sprint planning: "What should we prioritize for refactoring?"
- New project onboarding: "What are the problem areas?"
- Quarterly review: "Has quality improved or degraded?"

## Audit Categories

| Category | What It Reveals | Key Commands |
|----------|-----------------|--------------|
| **Modularity** | God modules, poor separation | `god-modules`, `large-functions` |
| **Coupling** | Circular deps, tight coupling | `cycles`, `clusters`, `hotspots` |
| **Complexity** | Hard to maintain code | `complexity`, `many-clauses` |
| **Dead Weight** | Unused code, duplication | `unused`, `duplicates` |
| **Architecture** | Boundaries, layers | `boundaries`, `depends-on` |

## Step-by-Step Audit

### 1. Check for Circular Dependencies

Cycles create maintenance nightmares:
```bash
# Find all circular dependencies
code_search --format toon cycles

# Limit to shorter cycles (more problematic)
code_search --format toon cycles --max-length 3

# Check if a specific module is involved in cycles
code_search --format toon cycles --involving MyApp.Core
```

**Red flags**: Any cycles, especially short ones (2-3 modules).

### 2. Find God Modules

Modules doing too much:
```bash
# Find modules with many functions and high connectivity
code_search --format toon god-modules

# Adjust thresholds for your codebase
code_search --format toon god-modules --min-functions 30 --min-total 20

# Filter to specific namespace
code_search --format toon god-modules MyApp.Core
```

**Red flags**: Modules with 50+ functions or 30+ total connections.

### 3. Identify Complex Functions

Functions that are hard to understand:
```bash
# Find functions with high complexity scores
code_search --format toon complexity --min 10

# Find deeply nested functions
code_search --format toon complexity --min-depth 4

# Find large functions by line count
code_search --format toon large-functions --min-lines 50

# Find functions with many pattern-match clauses
code_search --format toon many-clauses --min-clauses 8
```

**Red flags**: Complexity > 15, nesting > 4, lines > 100, clauses > 10.

### 4. Analyze Coupling Hotspots

Find over-connected code:
```bash
# Most called functions (might be over-used)
code_search --format toon hotspots --kind incoming -l 20

# Functions that call too many things (god functions)
code_search --format toon hotspots --kind outgoing -l 20

# Total connectivity hotspots
code_search --format toon hotspots --kind total -l 20
```

**Red flags**: Functions with > 20 incoming or > 10 outgoing calls.

### 5. Check Architectural Boundaries

Verify layer separation:
```bash
# Find boundary modules (high fan-in, low fan-out)
code_search --format toon boundaries

# Analyze module clusters
code_search --format toon clusters --show-dependencies

# Check a specific module's position
code_search --format toon depends-on MyApp.Web.UserController
code_search --format toon depended-by MyApp.Web.UserController
```

**Red flags**: Controllers depending on many modules, data layer with high fan-out.

### 6. Find Dead Weight

Code that can be removed:
```bash
# Unused functions
code_search --format toon unused -x

# Focus on definitely-dead private functions
code_search --format toon unused -px

# Find duplicated code
code_search --format toon duplicates

# Modules with most duplication
code_search --format toon duplicates --by-module
```

**Red flags**: > 5% unused functions, significant duplication.

## Generating an Audit Report

Run all checks and compile results:

```bash
# 1. Cycles
echo "=== CIRCULAR DEPENDENCIES ==="
code_search --format toon cycles --max-length 4

# 2. God Modules
echo "=== GOD MODULES ==="
code_search --format toon god-modules -l 10

# 3. Complexity
echo "=== COMPLEX FUNCTIONS ==="
code_search --format toon complexity --min 10 -l 10

# 4. Large Functions
echo "=== LARGE FUNCTIONS ==="
code_search --format toon large-functions --min-lines 75 -l 10

# 5. Coupling Hotspots
echo "=== COUPLING HOTSPOTS ==="
code_search --format toon hotspots --kind total -l 10

# 6. Unused Code
echo "=== UNUSED CODE ==="
code_search --format toon unused -px -l 20

# 7. Duplicates
echo "=== DUPLICATES ==="
code_search --format toon duplicates -l 10
```

## Interpreting Results

### Severity Levels

| Issue | Low | Medium | High | Critical |
|-------|-----|--------|------|----------|
| Cycles | None | 4+ module | 3 module | 2 module |
| God modules | < 30 funcs | 30-50 funcs | 50-100 funcs | > 100 funcs |
| Complexity | < 10 | 10-20 | 20-30 | > 30 |
| Function lines | < 50 | 50-100 | 100-200 | > 200 |
| Hotspot incoming | < 10 | 10-20 | 20-50 | > 50 |
| Unused private | < 5 | 5-20 | 20-50 | > 50 |

### Priority Matrix

| Severity | Frequency | Priority |
|----------|-----------|----------|
| Critical | Any | P0 - Fix now |
| High | Many | P1 - This sprint |
| High | Few | P2 - Next sprint |
| Medium | Many | P2 - Next sprint |
| Medium | Few | P3 - Backlog |
| Low | Any | P4 - Nice to have |

## Common Patterns and Fixes

| Finding | Common Cause | Fix |
|---------|--------------|-----|
| 2-module cycle | Shared state | Extract shared module |
| God module | Feature creep | Split by responsibility |
| High complexity | Nested conditionals | Extract functions, use pattern matching |
| Many clauses | State machine | Use behaviours or state pattern |
| High fan-out | Controller doing logic | Extract service/context modules |
| Unused code | Deprecated features | Delete it |
| Duplicates | Copy-paste | Extract shared functions |

## Tips

- **Run regularly**: Monthly audits catch regression
- **Track over time**: Compare counts between audits
- **Focus on trends**: One god module isn't critical; growing count is
- **Context matters**: Library code has different standards than app code
- **Prioritize by pain**: Frequently-changed modules need better quality

## Related Commands

| Command | Measures |
|---------|----------|
| `cycles` | Circular dependencies |
| `god-modules` | Module size and coupling |
| `complexity` | Function complexity |
| `large-functions` | Function line count |
| `many-clauses` | Pattern match complexity |
| `hotspots` | Coupling hotspots |
| `boundaries` | Architectural boundaries |
| `clusters` | Module groupings |
| `unused` | Dead code |
| `duplicates` | Code duplication |

## See Also

- [find-dead-code.md](find-dead-code.md) - Deep dive into dead code
- [impact-analysis.md](impact-analysis.md) - Before fixing issues
- [understand-feature.md](understand-feature.md) - Understanding problem areas

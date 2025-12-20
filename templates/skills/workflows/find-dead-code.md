# Workflow: Find Dead Code

Identify and safely remove unused code to reduce maintenance burden and improve codebase clarity.

## When to Use

- Cleanup: "What code can we safely delete?"
- Maintenance: "Are there functions nobody uses?"
- Refactoring: "Before restructuring, what's already dead?"
- Auditing: "How much cruft has accumulated?"

## Types of Dead Code

| Type | Certainty | Command |
|------|-----------|---------|
| Unused private functions | **Definite** dead code | `unused -p` |
| Unused public functions | Possibly dead (might be called externally) | `unused -P` |
| Orphan modules | No incoming dependencies | `depended-by` |
| Duplicate implementations | Candidates for consolidation | `duplicates` |

## Step-by-Step Process

### 1. Find Definitely Dead Code (Private Functions)

Private functions that are never called are guaranteed dead:
```bash
# Find all unused private functions
code_search --format toon unused -p

# Exclude compiler-generated functions
code_search --format toon unused -px

# Filter to specific area
code_search --format toon unused -p MyApp.Legacy
```

**These are safe to delete** - private functions can only be called from within their module.

### 2. Find Potentially Dead Public Functions

Public functions might be called from external code:
```bash
# Find unused public functions
code_search --format toon unused -P

# Exclude generated functions (__struct__, __info__, etc.)
code_search --format toon unused -Px
```

**Before deleting**, verify:
- Not called from tests (test files might not be in call graph)
- Not called via dynamic dispatch (`apply/3`, `&Module.function/arity`)
- Not an API/library function called by external projects
- Not a callback (GenServer, Plug, Phoenix)

### 3. Find Orphan Modules

Modules with no incoming dependencies might be dead:
```bash
# For each suspicious module, check dependents
code_search --format toon depended-by MyApp.OldFeature

# If result shows 0 dependents and not a known entry point, likely dead
```

Common false positives:
- Application entry points (`MyApp.Application`)
- Phoenix endpoint/router
- Supervision tree modules
- Mix tasks
- Test helpers

### 4. Find Duplicate Code

Code that's duplicated might indicate dead or consolidatable functions:
```bash
# Find duplicate implementations
code_search --format toon duplicates

# See which modules have most duplication
code_search --format toon duplicates --by-module
```

### 5. Validate Before Deleting

For each candidate:
```bash
# Double-check no callers
code_search --format toon calls-to ModuleName function_name

# Check if it's a boundary/entry point
code_search --format toon hotspots ModuleName
# Functions with 0 incoming but called from outside Elixir won't show here
```

## Example: Cleaning Up a Legacy Module

```bash
# 1. Find all unused functions in legacy area
code_search --format toon unused MyApp.Legacy

# Result shows 12 unused functions:
# - 8 private (defp) - safe to delete
# - 4 public (def) - need verification

# 2. Check the private functions
code_search --format toon unused -p MyApp.Legacy
# Confirmed: 8 private functions never called

# 3. For each public function, verify
code_search --format toon calls-to MyApp.Legacy old_helper
# 0 callers - can delete

code_search --format toon calls-to MyApp.Legacy format_data
# 0 internal callers, but check if it's API

# 4. Check if the whole module is orphaned
code_search --format toon depended-by MyApp.Legacy
# 0 dependents - entire module might be deletable

# 5. Safe to remove:
# - All 8 private functions
# - 3 of 4 public functions (after verification)
# - Possibly the entire module
```

## Safety Checklist

Before deleting, verify the function is NOT:

- [ ] A GenServer/Supervisor callback
- [ ] A Phoenix controller action (might be in router)
- [ ] A Plug callback (`init/1`, `call/2`)
- [ ] An Ecto callback (`changeset/2` in schema)
- [ ] A behaviour implementation
- [ ] Called via `apply/3` or `Kernel.apply/3`
- [ ] Called via `&Module.function/arity` capture
- [ ] A public API for external consumers
- [ ] Referenced in tests (if tests aren't in call graph)

## Incremental Cleanup Strategy

1. **Week 1**: Delete all unused private functions (`unused -px`)
2. **Week 2**: Investigate unused public functions, delete confirmed dead ones
3. **Week 3**: Check for orphan modules
4. **Ongoing**: Run `unused` in CI to catch new dead code

## Tips

- **Start with private**: `unused -p` is always safe
- **Check generated**: Use `-x` to skip `__struct__`, `__info__`, etc.
- **Mind the tests**: Test helpers might not be in the call graph
- **Watch for callbacks**: Framework callbacks appear unused but aren't
- **Verify public carefully**: External calls aren't tracked

## Related Commands

| Command | Use For |
|---------|---------|
| `unused` | Find uncalled functions |
| `unused -p` | Private only (definitely dead) |
| `unused -P` | Public only (potentially dead) |
| `unused -x` | Exclude generated functions |
| `depended-by` | Check for orphan modules |
| `calls-to` | Verify no callers |
| `duplicates` | Find consolidation candidates |

## See Also

- [impact-analysis.md](impact-analysis.md) - Assessing change risk
- [code-quality-audit.md](code-quality-audit.md) - Broader quality analysis

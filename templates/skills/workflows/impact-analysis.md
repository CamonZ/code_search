# Workflow: Impact Analysis

Determine the blast radius of a change before refactoring or modifying code.

## When to Use

- Before refactoring: "What will break if I change this function?"
- API changes: "Who uses this public function?"
- Deprecation: "Can I safely remove this module?"
- Risk assessment: "How risky is this change?"

## Step-by-Step Process

### 1. Identify What You're Changing

Be specific about the change:
- Changing a function signature?
- Modifying return values?
- Renaming or moving a function?
- Deleting a module?

### 2. Find Direct Callers

Start with immediate impact:
```bash
# Who calls this function directly?
code_search --format toon calls-to MyApp.Payments charge

# For a specific arity
code_search --format toon calls-to MyApp.Payments charge 2
```

### 3. Find Transitive Callers

Changes propagate up the call chain:
```bash
# Trace backwards to find all paths to this function
code_search --format toon reverse-trace MyApp.Payments charge --depth 5

# Increase depth for widely-used functions
code_search --format toon reverse-trace MyApp.Repo insert --depth 10
```

### 4. Check Module-Level Impact

For broader changes, check module dependencies:
```bash
# What modules depend on this one?
code_search --format toon depended-by MyApp.Payments

# Check if any of those are also widely used
code_search --format toon depended-by MyApp.PaymentsWeb
```

### 5. Assess Type-Level Impact

If changing types or structs:
```bash
# Who uses this type?
code_search --format toon struct-usage Payment.t

# Who accepts this type?
code_search --format toon accepts Payment.t

# Who returns this type?
code_search --format toon returns Payment.t
```

### 6. Check for Boundary Crossings

See if the change affects architectural boundaries:
```bash
# Is this a boundary module (many callers, few dependencies)?
code_search --format toon boundaries MyApp.Payments

# Check cluster membership
code_search --format toon clusters MyApp.Payments
```

Changes to boundary modules have wider impact.

### 7. Quantify the Impact

Get concrete numbers:
```bash
# Count callers
code_search --format toon calls-to MyApp.Payments charge
# Look at the count in output

# Count dependent modules
code_search --format toon depended-by MyApp.Payments
# Look at the count in output

# Check hotspot status
code_search --format toon hotspots MyApp.Payments
# High incoming = high impact
```

## Example: Changing a Core Function Signature

You want to change `MyApp.Accounts.get_user/1` to `get_user/2` (adding options):

```bash
# 1. Find direct callers
code_search --format toon calls-to MyApp.Accounts get_user
# Result: 15 direct callers

# 2. Trace backwards
code_search --format toon reverse-trace MyApp.Accounts get_user --depth 4
# Result: Shows call chains from controllers, jobs, other contexts

# 3. Check module impact
code_search --format toon depended-by MyApp.Accounts
# Result: 8 modules depend on Accounts

# 4. See if it's a hotspot
code_search --format toon hotspots MyApp.Accounts
# Result: get_user has 15 incoming calls - medium impact

# Conclusion: Need to update 15 call sites across 8 modules
```

## Example: Deleting a Module

You want to remove `MyApp.LegacyPayments`:

```bash
# 1. Check if anything depends on it
code_search --format toon depended-by MyApp.LegacyPayments
# Result: 0 dependents - safe to delete!

# Or if there are dependents:
# Result: 3 modules still depend on it

# 2. Find specific usages
code_search --format toon calls-to MyApp.LegacyPayments
# Shows exactly which functions are still called

# 3. Check for type usage
code_search --format toon struct-usage LegacyPayment.t
# Shows if the struct is still used anywhere
```

## Impact Levels

| Level | Indicators | Action |
|-------|------------|--------|
| **Low** | < 5 callers, 1-2 modules | Safe to change directly |
| **Medium** | 5-20 callers, 3-5 modules | Careful review, maybe deprecate first |
| **High** | > 20 callers, > 5 modules, boundary module | Deprecation period, staged rollout |
| **Critical** | Hotspot, many clusters affected | Major version, extensive testing |

## Risk Mitigation Strategies

Based on impact level:

1. **Low impact**: Change directly, update callers
2. **Medium impact**:
   - Add new function alongside old
   - Deprecate old with warning
   - Update callers incrementally
3. **High impact**:
   - Feature flag the change
   - Parallel implementations
   - Gradual migration
4. **Critical**:
   - RFC/design doc first
   - Deprecation in previous release
   - Migration guide

## Tips

- **Check both levels**: Function callers AND module dependents
- **Follow the types**: Type changes have hidden impact
- **Consider tests**: Many callers often means many test updates
- **Check boundaries**: Changing boundary modules affects everything above
- **Depth matters**: `--depth 3` might miss important transitive callers

## Related Commands

| Command | Use For |
|---------|---------|
| `calls-to` | Direct function callers |
| `reverse-trace` | Transitive callers |
| `depended-by` | Module-level dependents |
| `struct-usage` | Type usage |
| `hotspots` | Identify high-impact functions |
| `boundaries` | Find architectural boundaries |

## See Also

- [understand-feature.md](understand-feature.md) - Full feature exploration
- [find-dead-code.md](find-dead-code.md) - Finding safe-to-delete code

# Architectural Patterns & Refactoring Rules

## Query-Level vs Output-Level Concerns

**Pattern:** Distinguish between query filters and output metadata.

**Query filters** (applied during database query):
- `project`, `module_pattern`, `function_pattern`
- `regex`, `limit`, `depth`
- These are parameters, not data

**Output metadata** (included in result struct):
- `total_items`, `entries`, `items`
- `file`, `kind`, `start_line`
- These describe the results

**Rule:** Never include query-level filters in output result structs.

**Benefits:**
- Cleaner result types that only carry data
- No confusion about what's data vs. configuration
- Easier to cache/serialize results

## Avoid Custom Outputable Implementations

**Anti-pattern:**

```rust
impl Outputable for CustomResult {
    fn to_table(&self) -> String {
        // 40+ lines of boilerplate layout logic
    }
}
```

**Preferred pattern:**

```rust
impl TableFormatter for ModuleGroupResult<Entry> {
    type Entry = Entry;
    // 15-20 lines of domain-specific formatting
    fn format_header(&self) -> String { ... }
    fn format_entry(&self, entry, module, file) -> String { ... }
    // TableFormatter default impl handles all layout
}
```

This rule applies to all module-grouped output commands.

See [NEW_COMMANDS.md](./NEW_COMMANDS.md) for detailed implementation patterns and checklist for adding new module-grouped commands.

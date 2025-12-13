# Pending Commands

Commands not yet implemented in the CLI.

| Command | Description | Ticket |
|---------|-------------|--------|
| `complexity` | Display complexity metrics for functions and modules | #22 |
| `large-functions` | Find functions with many lines of code (large line span) | #15 |
| `many-clauses` | Find functions with many pattern-matched heads | #16 |
| `returns` | Find functions that return a specific type pattern | #19 |
| `accepts` | Find functions that accept a specific type pattern | #21 |
| `cycles` | Find circular dependencies between modules | #23 |
| `clusters` | Group modules by coupling | #24 |
| `boundaries` | Identify boundary modules | #13 |
| `god-modules` | Find modules with too many functions/dependencies | #14 |
| `struct-modules` | Show which modules work with each struct (via specs) | #25 |
| `duplicates` | Find functions with identical or near-identical implementations | #17 |
| `duplicate-hotspots` | List modules ranked by number of duplicated functions | #18 |

## Schema Prerequisites

Some commands require schema updates from Ticket #00 (Schema Foundation):

| Command | Required Fields |
|---------|----------------|
| `complexity` | `complexity`, `max_nesting_depth` |
| `large-functions` | `generated_by` (for filtering) |
| `many-clauses` | `generated_by` (optional filtering) |
| `god-modules` | `complexity` (for enhanced scoring) |

## Covered by Existing Commands

These were originally planned but are already supported:

| Original Idea | Use Instead |
|---------------|-------------|
| `call-sites` | `calls-to -m Module -f function` — shows all call locations with file/line/column |
| `orphan-functions` | `unused --private-only` — finds private functions never called |
| `wide-modules` | `hotspots -k outgoing` — modules/functions with high fan-out |
| `deep-modules` | `hotspots -k incoming` — modules/functions with high fan-in |
| `entry-points` | `unused --public-only` (inverted) — public functions with zero internal callers |

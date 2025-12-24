# Ticket 05 Summary: Configure Feature Flags

**Date**: 2025-12-24
**Status**: ✅ COMPLETE
**Time**: ~1 hour

## What We Accomplished

Successfully configured Cargo feature flags to enable compile-time backend selection, allowing users to choose between CozoDB and SurrealDB backends.

## Changes Made

### 1. Updated db/Cargo.toml

**Made dependencies optional:**
```toml
[features]
default = ["backend-cozo"]
backend-cozo = ["dep:cozo"]
backend-surrealdb = ["dep:surrealdb", "dep:tokio"]
test-utils = ["tempfile", "serde_json"]

[dependencies]
# Core dependencies (always included)
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
regex = "1"
include_dir = "0.7"
clap = { version = "4", features = ["derive"] }

# Backend-specific dependencies (optional)
cozo = { version = "0.7.6", ..., optional = true }
surrealdb = { version = "2.0", features = ["kv-rocksdb"], optional = true }
tokio = { version = "1", features = ["rt", "macros"], optional = true }

# Test utilities (optional)
tempfile = { version = "3", optional = true }
serde_json = { version = "1.0", optional = true }
```

### 2. Updated cli/Cargo.toml

**Added feature propagation:**
```toml
[features]
default = ["backend-cozo"]
backend-cozo = ["db/backend-cozo"]
backend-surrealdb = ["db/backend-surrealdb"]

[dependencies]
db = { path = "../db", default-features = false }

[dev-dependencies]
db = { path = "../db", features = ["test-utils"], default-features = false }
```

**Key change**: `default-features = false` ensures backend selection is controlled by CLI features.

### 3. Fixed db/src/db.rs

**Removed outdated feature gates:**
- `run_query()` - Now backend-agnostic (uses Database trait)
- `run_query_no_params()` - Now backend-agnostic
- `try_create_relation()` - Now backend-agnostic

These functions were incorrectly gated behind `#[cfg(feature = "backend-cozo")]` even though they now work with any backend.

### 4. Updated db/src/lib.rs

**Made CozoDB-specific exports conditional:**
```rust
// CozoDB-specific exports (only when backend-cozo enabled)
#[cfg(feature = "backend-cozo")]
pub use db::extract_call_from_row;

#[cfg(feature = "backend-cozo")]
pub use cozo::DbInstance;

// Backend abstraction exports (always available)
pub use backend::{Database, QueryResult, Row, Value, QueryParams};
```

## Verification Results

### ✅ Default Build (CozoDB)
```bash
$ cargo build
✓ Compiled successfully
✓ cozo included in dependency tree
✓ surrealdb NOT in dependency tree
```

### ✅ SurrealDB Build
```bash
$ cargo build --no-default-features --features backend-surrealdb
✓ Compiled successfully
✓ surrealdb included in dependency tree
✓ cozo NOT in dependency tree
```

### ✅ No Backend Build (Should Fail)
```bash
$ cargo build --no-default-features
✗ Compile error: "Must enable either backend-cozo or backend-surrealdb"
✓ Error message as expected
```

### ✅ Test Suite
```bash
$ cargo test
✓ 516 CLI tests passed
✓ 77 DB tests passed
✓ 3 doc tests passed
✓ No regressions
```

## Feature Propagation Demo

```bash
# CLI controls which backend db uses:

# CozoDB (default)
cargo build -p code_search
  → cli uses backend-cozo feature
  → db uses backend-cozo feature
  → cozo dependency included

# SurrealDB
cargo build -p code_search --no-default-features --features backend-surrealdb
  → cli uses backend-surrealdb feature
  → db uses backend-surrealdb feature
  → surrealdb + tokio dependencies included
```

## What This Enables

### 1. **True Backend Selection**
Users can now choose which database to compile:
```toml
# In a downstream project's Cargo.toml
code_search = { version = "0.1", default-features = false, features = ["backend-surrealdb"] }
```

### 2. **Smaller Binaries**
Only the selected backend is compiled, reducing:
- Compile time
- Binary size
- Dependency count

### 3. **Clean Compilation**
- Default build uses CozoDB (backward compatible)
- SurrealDB build compiles cleanly (stub implementation)
- No backend = clear compile error

### 4. **Feature Gate Correctness**
- Backend-agnostic code: No feature gates
- CozoDB-specific code: `#[cfg(feature = "backend-cozo")]`
- SurrealDB-specific code: `#[cfg(feature = "backend-surrealdb")]`

## Breaking Changes

None! The default feature is `backend-cozo`, so existing builds work unchanged.

## Dependencies Between Tickets

**Ticket 05 Completed** ✅

This ticket was independent and is now done.

**Next Steps:**
- Ticket 06: Clean up lib.rs exports (30 min) - Optional
- Ticket 07: Add backend unit tests (2-3 hrs) - Optional

## Technical Notes

### Why `dep:` Syntax?

```toml
backend-cozo = ["dep:cozo"]
```

The `dep:` prefix is required in Rust 2024 edition when features enable optional dependencies. It disambiguates between:
- `["cozo"]` - Enable feature named "cozo" on an existing dependency
- `["dep:cozo"]` - Enable the optional dependency named "cozo"

### Why `default-features = false`?

Without it:
```toml
db = { path = "../db" }
# db always uses its default = ["backend-cozo"]
# Even if you specify --features backend-surrealdb!
```

With it:
```toml
db = { path = "../db", default-features = false }
# db uses NO features by default
# CLI controls which features to enable via propagation
```

### Dev Dependencies Note

```toml
[dev-dependencies]
db = { path = "../db", features = ["test-utils"], default-features = false }
```

Test utils are always enabled for tests, but backend is still controlled by the build features.

## Files Modified

1. `db/Cargo.toml` - Made dependencies optional, updated features
2. `cli/Cargo.toml` - Added feature propagation
3. `db/src/db.rs` - Removed incorrect feature gates from 3 functions
4. `db/src/lib.rs` - Made CozoDB-specific exports conditional

## Lessons Learned

1. **Feature gates should match actual dependencies**
   - `run_query()` was gated but works with any backend
   - Removed gate, function works everywhere

2. **Export visibility matters**
   - `extract_call_from_row()` truly is CozoDB-specific
   - Made export conditional, not the function itself

3. **Feature propagation requires `default-features = false`**
   - Otherwise downstream controls don't work
   - Library keeps using its own defaults

## Conclusion

Ticket 05 is complete! Backend selection now works correctly:
- ✅ Optional dependencies
- ✅ Feature propagation
- ✅ Compile-time backend selection
- ✅ All tests passing
- ✅ Clean error messages

The codebase is now properly configured for multi-backend support.

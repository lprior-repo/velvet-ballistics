# Architectural Drift Report: `status_args_hammer`

**File Attacked:** `/home/lewis/src/velvet-ballistics/crates/vb_cli/src/args/tests/status.rs`
**Original Size:** 334 lines
**Size Limit:** 300 lines
**Violation:** YES — 34 lines over limit (11.3% excess)

---

## 1. Line Count Breakdown

| Section | Lines | Tests |
|---------|-------|-------|
| `system status` tests (lines 4–132) | 129 | 10 tests |
| `status` tests (lines 134–334) | 201 | 17 tests |
| **Total** | **334** | **27 tests** |

---

## 2. Primitive Obsession Violations

### 2.1 Raw Numeric Primitives in `StatusOptions`

**Location:** `args.rs` lines 222–227

```rust
pub(crate) struct StatusOptions {
    pub(crate) active_runs: Option<usize>,      // PRIMITIVE OBSESSION
    pub(crate) queue_depth: Option<usize>,     // PRIMITIVE OBSESSION
    pub(crate) trace_dropped: Option<u64>,     // PRIMITIVE OBSESSION
    pub(crate) emit_yaml: bool,
}
```

**Problem:** `usize` and `u64` are raw machine types with no domain semantics.

**Should Be:**
```rust
pub(crate) struct StatusOptions {
    pub(crate) active_runs: Option<ActiveRunCount>,    // domain type
    pub(crate) queue_depth: Option<QueueDepth>,       // domain type
    pub(crate) trace_dropped: Option<TraceDropped>,    // domain type
    pub(crate) emit_yaml: bool,
}
```

### 2.2 Magic Numbers in Validation

| Line | Error Message | Magic Value | Should Be |
|------|---------------|-------------|-----------|
| 272 | `"--queue-depth must be <= 1024"` | 1024 | `QueueDepth::MAX` |
| 286 | `"--active-runs must be <= 1024"` | 1024 | `ActiveRunCount::MAX` |
| 297 | `"1024"` (test input) | 1024 | `QueueDepth::MAX` |
| 312 | `"1024"` (test input) | 1024 | `ActiveRunCount::MAX` |
| 327 | `"18446744073709551615"` | u64::MAX | `u64::MAX` constant |

### 2.3 Raw String Manipulation

**Location:** Lines 201, 272, 286 — error messages constructed via `format!`

```rust
// Line 201 - raw format string
Err(ParseError::InvalidStatusArgument(ref s)) if s == "--queue-depth must be a usize"

// Lines 272, 286 - magic number embedded in string
"--queue-depth must be <= 1024"
"--active-runs must be <= 1024"
```

**Problem:** Validation logic leaks raw numeric constraints into error strings.

---

## 3. Test Design Violations

### 3.1 Missing Domain Value Generators

The tests use raw string slices:
```rust
"--active-runs", "5"   // raw string, raw number
"--queue-depth", "3"
"--trace-dropped", "0"
```

**Missing:** A `StatusArgs` test builder or domain value generator:
```rust
fn status_args_with(values: StatusArgValues) -> Vec<OsString> { ... }
fn active_runs(n: impl Into<ActiveRunCount>) -> (OsString, OsString) { ... }
```

### 3.2 No Test Data Isolation

27 tests share no common setup. Each test manually constructs `args(&[...])`.

**Evidence of copy-paste pattern:**
- Lines 4–15, 18–38, 41–49, 53–62, 65–74, 77–86, 89–99, 103–114, etc.

### 3.3 Mixed Subcommand Testing

The file tests two distinct subcommands (`status` vs `system status`) with different option sets:

| Subcommand | Options | Errors |
|------------|---------|--------|
| `status` | `--active-runs`, `--queue-depth`, `--trace-dropped`, `--emit` | `InvalidStatusArgument` |
| `system status` | `--profile`, `--server`, `--emit` | `InvalidSystemStatusArgument`, `UnknownProfile`, `UnknownServerMode` |

**Violation:** Two separate concerns in one file. Should be split:
- `args/tests/status.rs` — for `status` subcommand
- `args/tests/system_status.rs` — for `system status` subcommand

---

## 4. Validation Logic Location

The validation bounds are defined in `args/status.rs` lines 146–168:

```rust
fn validate_status_options(options: StatusOptions) -> Result<StatusOptions, ParseError> {
    let config = vb_runtime::shard::ShardConfig::default();
    validate_status_usize_limit(
        options.queue_depth,
        config.command_queue_capacity,
        "--queue-depth",
    )?;
    validate_status_usize_limit(options.active_runs, config.max_active_runs, "--active-runs")?;
    Ok(options)
}
```

**Problem:** Magic numbers 1024 are obtained from runtime config, not compile-time constants. The tests hardcode 1024 as expected value (lines 272, 286, 297, 312) but the actual limit comes from `ShardConfig`.

---

## 5. Test/Production Coupling

**Location:** Line 147 in `args/status.rs`
```rust
let config = vb_runtime::shard::ShardConfig::default();
```

The production parsing code imports from `vb_runtime` shard config. Tests cannot run in isolation without this dependency.

---

## 6. Specific Refactoring Recommendations

### 6.1 Extract Domain Types (Required)

```rust
// crates/vb_cli/src/args/status_types.rs

/// Number of concurrent active runs.
/// Range: 0..=1024
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ActiveRunCount(usize);

impl ActiveRunCount {
    pub const MAX: usize = 1024;
    pub const fn new(n: usize) -> Result<Self, ()> { ... }
    pub const fn value(self) -> usize { self.0 }
}

/// Queue depth for command buffer.
/// Range: 0..=1024
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct QueueDepth(usize);

impl QueueDepth {
    pub const MAX: usize = 1024;
    pub const fn new(n: usize) -> Result<Self, ()> { ... }
    pub const fn value(self) -> usize { self.0 }
}

/// Counter for dropped traces.
/// Range: 0..=u64::MAX
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct TraceDropped(u64);

impl TraceDropped {
    pub const fn new(n: u64) -> Self { Self(n) }
    pub const fn value(self) -> u64 { self.0 }
}
```

### 6.2 Split Test File (Required)

| File | Subcommand | Test Count |
|------|------------|------------|
| `status.rs` | `status` | 17 tests |
| `system_status.rs` | `system status` | 10 tests |

### 6.3 Add Test Builders (Required)

```rust
// crates/vb_cli/src/args/tests/shared.rs

pub struct StatusTestBuilder {
    active_runs: Option<ActiveRunCount>,
    queue_depth: Option<QueueDepth>,
    trace_dropped: Option<TraceDropped>,
    emit: OutputFormat,
}

impl StatusTestBuilder {
    pub fn active_runs(mut self, n: usize) -> Self { ... }
    pub fn queue_depth(mut self, n: usize) -> Self { ... }
    pub fn build(self) -> Vec<OsString> { ... }
}
```

---

## 7. Summary of Violations

| Rule | Severity | Description |
|------|----------|-------------|
| `<300 line limit` | **CRITICAL** | 334 lines (11.3% over) |
| Primitive obsession | **HIGH** | Raw `usize`/`u64` instead of domain types |
| Single responsibility | **HIGH** | Two subcommands in one test file |
| No magic numbers | **MEDIUM** | Hardcoded 1024 in tests and error messages |
| Test isolation | **MEDIUM** | No shared test builders |
| Test coupling | **LOW** | Tests depend on runtime ShardConfig |

---

## 8. Recommended Action

1. **IMMEDIATE:** Split `status.rs` into `status.rs` (17 tests) and `system_status.rs` (10 tests)
2. **NEXT:** Extract `ActiveRunCount`, `QueueDepth`, `TraceDropped` domain types
3. **NEXT:** Replace magic numbers with named constants from domain types
4. **NEXT:** Add test builder utilities to reduce repetition
5. **VERIFY:** Each resulting file ≤300 lines

---

*Report generated by: arch-drift-hammer*
*JJ workspace: /home/lewis/src/velvet-ballistics/arch-drift-hammer*

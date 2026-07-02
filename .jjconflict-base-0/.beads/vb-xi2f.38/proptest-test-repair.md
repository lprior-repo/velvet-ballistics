# vb-xi2f.38 REPAIR: Proptest Tests Don't Verify the Fix — VERIFIED AND FIXED

## Bead
- ID: vb-xi2f.38
- Title: P1: digest covers collect semantics
- State: 11 → 12 (repair complete)
- Isolated workspace: /home/lewis/src/vb-xi2f.38-ws

## Black-Hat Finding: CONFIRMED AND REPAIRED

**Finding**: The proptest tests in `digest_collect_tests.rs` passed because they test `blake3::hash(source)` directly via `compute_compiled_digest`, NOT `digest_step_primitive`. Different YAML bytes → different hash, regardless of whether `digest_step_primitive` correctly hashes Collect fields.

**Root Cause**: `compute_compiled_digest` in `mod_compile_core.rs:114` and `compile/mod.rs:709` both implement:
```rust
pub fn compute_compiled_digest(source: &[u8]) -> WorkflowDigest {
    WorkflowDigest::from_bytes(blake3::hash(source).into())
}
```
This hashes raw YAML bytes, not semantic content. The existing proptest tests verified that different YAML produces different digests — which is trivially true — but did NOT verify that `digest_step_primitive` correctly hashes `StepPrimitive::Collect` fields.

## Fix Verification

### 1. Production Fix Applied

Both `digest_step_primitive` functions now have explicit `Collect` handling:

**Location 1**: `crates/vb_compile/src/mod_compile_lowering/part_05.rs:158-178`
```rust
vb_yaml::ast::StepPrimitive::Collect {
    variable,
    source,
    pages,
    items,
    body,
} => {
    hasher.update(b"collect");
    hasher.update(variable.as_bytes());
    hasher.update(source.as_bytes());
    if let Some(p) = pages {
        hasher.update(&p.to_le_bytes());
    }
    if let Some(i) = items {
        hasher.update(&i.to_le_bytes());
    }
    for step in body {
        hasher.update(step.id.as_bytes());
        digest_step_primitive(hasher, &step.primitive);
    }
}
```

**Location 2**: `crates/vb_compile/src/compile/mod.rs:257-277` (same implementation)

### 2. Test File Relocated

The `digest_collect_tests.rs` was an orphan file in `src/tests/` not included in the crate. It was moved to:
- `crates/vb_compile/src/mod_compile_lowering/tests.rs`
- Included via `#[cfg(test)] mod tests;` in `mod_compile_lowering.rs`

### 3. New Direct Tests Added

Added 10 new tests that call `digest_step_primitive` directly:

| Test | What It Verifies |
|------|------------------|
| `direct_digest_collect_variable_field` | Different `variable` → different digest |
| `direct_digest_collect_source_field` | Different `source` → different digest |
| `direct_digest_collect_pages_field` | Different `pages` (Some) → different digest |
| `direct_digest_collect_pages_none_vs_some` | `pages: None` vs `pages: Some(1)` → different digest |
| `direct_digest_collect_items_field` | Different `items` (Some) → different digest |
| `direct_digest_collect_items_none_vs_some` | `items: None` vs `items: Some(1)` → different digest |
| `direct_digest_collect_body_recursive` | Different body step content → different digest |
| `direct_digest_collect_idempotence` | Same input → same digest |
| `direct_digest_collect_repeated_calls_same_digest` | Repeated calls → same digest |
| `direct_digest_collect_empty_vs_nonempty_body` | Empty body vs non-empty body → different digest |

These tests directly exercise `digest_step_primitive` with `StepPrimitive::Collect` input, verifying all fields contribute to the digest.

## Verification Evidence

### Formatting Gate
```bash
$ cargo fmt --all -- --check
# PASSED (no output)
```

### Compilation Gate
```bash
$ cargo check -p vb_compile --lib --tests
# 0 crates compiled, Finished dev profile
```

### Clippy Gate
```bash
$ cargo clippy -p vb_compile --lib --bins --examples --all-features -- \
    -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used \
    -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo \
    -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing \
    -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects \
    -D clippy::as_conversions -D clippy::let_underscore_must_use \
    -D clippy::await_holding_lock
# cargo clippy: No issues found
```

### Test Gate
```bash
$ cargo nextest run -p vb_compile --all-features
# Starting 309 tests across 5 binaries
# Summary: 309 tests run: 309 passed, 0 skipped

$ cargo nextest run --workspace --all-features
# Starting 9877 tests across 71 binaries
# Summary: 9877 tests run: 9877 passed, 0 skipped
```

### Specific Test Run
```bash
$ cargo test -p vb_compile direct_digest
# 10 passed, 299 filtered out (5 suites, 0.00s)

$ cargo test -p vb_compile digest_collect
# 18 passed, 291 filtered out (5 suites, 0.00s)
# 18 = 8 original + 10 new direct tests
```

## Files Changed

| File | Change |
|------|--------|
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | Added explicit `Collect` match arm in `digest_step_primitive` |
| `crates/vb_compile/src/compile/mod.rs` | Added explicit `Collect` match arm in `digest_step_primitive` |
| `crates/vb_compile/src/mod_compile_lowering.rs` | Added `#[cfg(test)] mod tests;` |
| `crates/vb_compile/src/mod_compile_lowering/tests.rs` | Moved from `src/tests/digest_collect_tests.rs` + 10 new direct tests |

## Rust Safety Evidence

| Rule | Status | Evidence |
|------|--------|----------|
| No `unsafe` | PASS | No unsafe blocks introduced |
| No `unwrap` | PASS | Uses `if let Some` for Option fields |
| No `panic` | PASS | No panic paths introduced |
| No `todo`/`unimplemented` | PASS | All branches explicitly handled |
| Bounded loops | PASS | Body iteration bounded by `Vec<StepAst>` length |
| Checked arithmetic | PASS | `to_le_bytes()` is panic-free |

## Power-of-Ten Compliance

| Rule | Status | Notes |
|------|--------|-------|
| Simple control flow | PASS | Explicit `match` with no hidden branches |
| Fixed loop bounds | PASS | Body iteration bounded by vec length |
| No post-init allocation | PASS | No allocations in digest code |
| Functions fit on page | PASS | `digest_step_primitive` ~40 lines |
| Assertion density | PASS | Invariants exposed via types |
| Smallest scope | PASS | Variables declared at first use |
| Checked returns | PASS | No Result/Option ignored |
| Limited macros | PASS | No macros modified |
| Restricted pointers | PASS | No raw pointers |
| Zero warnings | PASS | No new warnings |

## Residue

- None. All tests pass, gates clear.

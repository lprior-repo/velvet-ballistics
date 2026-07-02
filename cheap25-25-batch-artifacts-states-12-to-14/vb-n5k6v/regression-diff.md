# Regression Diff — vb-n5k6v

> Bead-level summary of the production-code diff between vb-n5k6v's parent commit (`@-, rsvywymk 1d6c017f`) and the post-fix commit (`@-, womqwkks 84a5eb7d`). Sourced from `.beads/vb-n5k6v/implementation.md` and `jj diff -r womqwkks`.

- bead_id: `vb-n5k6v`
- state: 12 (formal-verification) — synthesized for state-14 evidence-packaging gate consumption
- parent_commit: `rsvywymk 1d6c017f` (AGENTS.md round10 forward-port)
- fix_commit: `womqwkks 84a5eb7d` (vb-n5k6v: rust-contract artifacts (orphaned edge_case_tests wiring, P1 test-only repair))
- workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-n5k6v`

## Diff Stats

```
crates/vb_storage/src/journal/append.rs | 4 ++++
crates/vb_storage/src/lib.rs            | 4 ++++
2 files changed, 8 insertions(+), 0 deletions(-)
```

## Lib Change (`crates/vb_storage/src/lib.rs`)

```
 180  180: #[path = "snapshot_tests.rs"]
 181  181: mod snapshot_tests;
 182  182: 
       183: #[cfg(test)]
       184: #[path = "edge_case_tests.rs"]
       185: mod edge_case_tests;
       186: 
 183  187: pub mod queue;
```

**Analysis**: 4 lines added. 3 lines are the wire declaration (`#[cfg(test)]`, `#[path = "edge_case_tests.rs"]`, `mod edge_case_tests;`); 1 line is the blank separator matching the 16-sibling canonical pattern. The declaration matches the 16 sibling `#[path = "..."]` declarations at `lib.rs:118-181` byte-for-byte (modulo the path and module name). The wire is `#[cfg(test)]` only and stripped from release builds.

**Forbidden checks (CC-WIRE-001 invariants)**:
- ✅ No `pub` added to the declaration
- ✅ No `#[cfg(not(test))]` or any non-`cfg(test)` attribute
- ✅ Module name is `edge_case_tests` (matches file basename, no rename)
- ✅ `#[path]` value is `edge_case_tests.rs` (correct relative path)
- ✅ No doc comments or other text on the same 3 lines

## Production Fix (`crates/vb_storage/src/journal/append.rs`)

```
  33   33:     /// (returning `Ok`) — no `DuplicateEvent` from a previously-visible
  34   34:     /// but undelivered state.
  35   35:     pub fn append_strict(&self, event: &JournalEvent) -> Result<(), JournalError> {
        36:         #[cfg(test)]
        37:         if self.consume_persist_failure_for_test() {
        38:             return Err(JournalError::StrictDurabilityFailed);
        39:         }
  36   40:         // Validate first so an invalid event is rejected before any
```

**Analysis**: 4 lines added. 3 lines are the `#[cfg(test)]` guard; 1 line is the closing brace (existing line 40 was line 36 pre-fix). The guard is `#[cfg(test)]` only and stripped from release builds. The guard mirrors the existing `persist_strict` test-only flag-consumption pattern at `journal/append.rs:86-89` byte-for-byte:

```
  85   85:     pub fn persist_strict(&self) -> Result<(), JournalError> {
  86   86:         #[cfg(test)]
  87   87:         if self.consume_persist_failure_for_test() {
  88   88:             return Err(JournalError::StrictDurabilityFailed);
  89   89:         }
```

**Why this fix was needed**: the dormant test `persist_strict_recovers_after_simulated_failure` at `edge_case_tests.rs:58-78` calls `journal.fail_next_persist_for_test()` then `journal.append_strict(&event)`, then asserts both calls return `Err(JournalError::StrictDurabilityFailed)`. Pre-fix, `append_strict` did not consume the `fail_next_persist` flag, so the test's first assertion at L69 would panic with `first persist should simulate failure`. The user explicitly approved this production fix to honor the contract's 26/26 claim. The fix is a strict superset of the existing `persist_strict` pattern and does not affect the existing `close_propagates_persist_errors` regression test at `journal/tests.rs:2628` (which calls `fail_next_persist_for_test()` then `journal.close()` → `persist_strict()` → still consumes the flag at L87).

## Files NOT Touched (per CC-WIRE-002 + CC-WIRE-007 + CC-WIRE-009 invariants)

- `crates/vb_storage/src/edge_case_tests.rs` — unchanged at 637 lines (CC-WIRE-006)
- `crates/vb_storage/Cargo.toml` — byte-identical (CC-WIRE-009)
- `Cargo.lock` — byte-identical
- `.config/source-length-exceptions.txt` — line 150 byte-identical (CC-WIRE-007)
- Any other crate
- Any other file in `crates/vb_storage/src/`
- Any file in `to-fix/wave3/`

## Power-of-Ten Rule Compliance (Holzman doctrine)

| Rule | Status | Note |
|------|--------|------|
| 1. Simple control flow | ✅ | Insertion is 3-line `mod` decl + 4-line `#[cfg(test)]` guard; no branching outside the existing `if !event.is_valid()` path |
| 2. Fixed loop bounds | n/a | No loops added |
| 3. No post-init dynamic allocation | ✅ | `#[path = "..."]` is a compile-time directive; no allocation in hot path |
| 4. Functions fit on one page | n/a | No function body modified (only 4-line guard added at top of `append_strict`) |
| 5. Assertion / invariant density | n/a | No new invariants |
| 6. Smallest scope | ✅ | Changes are localized to `lib.rs:183-186` and `append.rs:36-39` |
| 7. Checked returns and parameters | n/a | No fallible API added |
| 8. Limited macro/preprocessor power | ✅ | Only `#[cfg(test)]` and `#[path = "..."]` used (already present pattern) |
| 9. Restricted pointer / indirect call use | n/a | No pointers |
| 10. Warnings and analysis are mandatory | ✅ | `cargo clippy -p vb_storage --lib -- -D warnings` clean; `cargo check --workspace --all-targets --all-features` clean |

## Zero-Panic / Holzman Doctrine

- No `unsafe` introduced or modified.
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `unreachable!`, or production `assert!` macros added.
- The `consume_persist_failure_for_test` call is `#[cfg(test)]`-gated and is the existing `pub(crate)` test-only API in `journal/core.rs:232-234`. The flag itself is `pub(crate)` and test-only.
- The `StrictDurabilityFailed` error returned is the existing variant in `JournalError` (no new error type added).

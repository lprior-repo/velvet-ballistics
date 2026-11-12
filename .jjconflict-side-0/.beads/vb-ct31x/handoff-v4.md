# VB-CT31X V4 Module Split Handoff

## Summary
Moved `put_emergency_deny()` method from `crates/vb_storage/src/batch/putters.rs` to new sibling module `crates/vb_storage/src/batch/emergency_deny.rs`. putters.rs now 291 lines (under 300 limit).

## Changes
- **New file:** `crates/vb_storage/src/batch/emergency_deny.rs` (16 lines) — contains `impl JournalWriteBatch` for `put_emergency_deny()`
- **Modified:** `crates/vb_storage/src/batch/putters.rs` — removed `put_emergency_deny()`, now 291 lines
- **Modified:** `crates/vb_storage/src/batch/mod.rs` — added `mod emergency_deny;`

## Evidence
- DENY_V4_01: `cargo test -p vb_storage emergency_deny --all-features` — PASS
- DENY_V4_02: `cargo test -p vb_storage --lib --all-features` — PASS
- DENY_V4_03b: `moon ci :fmt :source-length :lint-src --downstream none --force --concurrency 2` — PASS
- All evidence at `.beads/vb-ct31x/evidence/`

## Commands Used
```
/tmp/vb-run-gate.py 300 .beads/vb-ct31x/evidence/DENY_V4_01 rustup run nightly-2026-04-28 cargo test -p vb_storage emergency_deny --all-features
/tmp/vb-run-gate.py 600 .beads/vb-ct31x/evidence/DENY_V4_02 rustup run nightly-2026-04-28 cargo test -p vb_storage --lib --all-features
/tmp/vb-run-gate.py 600 .beads/vb-ct31x/evidence/DENY_V4_03b moon ci :fmt :source-length :lint-src --downstream none --force --concurrency 2
```

## Remaining Limits
- No test changes made
- No VCS/bd mutations
- No .config/.moon/scripts/AGENTS.md/MASTER/Cargo deps changes
- No exceptions/suppressions added

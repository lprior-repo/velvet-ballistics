# Architecture Refactor — vb-l2d7 Retry 15

STATUS: REFACTORED

## Drift fixes

- Split `crates/vb_doc/src/reconcile.rs` from 384 lines into cohesive modules:
  - `reconcile/contradictions.rs` — stale taint and Finish contradiction collection.
  - `reconcile/evidence_claims.rs` — evidence-bounded claim checks.
  - `reconcile/vocabulary.rs` — taint lattice/vocabulary checks.
  - `reconcile/workspace.rs` — master-doc path and non-goal boundary checks.
- Split `tests/vb_l2d7_doc_reconciliation_contract_red.rs` from 1323 lines into focused test modules under `tests/vb_l2d7_doc_reconciliation_contract_red/`.
- Removed the hardcoded `/tmp` workspace fixture path from the doc reconciliation test; it now uses `std::env::temp_dir()` like the other path fixtures.

## Validation

- Focused file-length scan: all refactored production/test files are under 300 lines.
- Focused function-length scan: PASS.
- Panic/unsafe scan for focused production files: PASS.
- `rtk cargo fmt --check`: PASS.
- `python scripts/check-doc-taint-consistency.py velvet-ballistics-MASTER.md`: PASS.
- `rtk cargo nextest run -p velvet-ballastics-workspace --test vb_l2d7_doc_reconciliation_contract_red`: 70 passed.
- `rtk cargo nextest run -p vb_runtime --test vb_l2d7_joined_taint_propagation_red`: 24 passed.
- Finish contradiction probes: Secret/DerivedFromSecret rejection wording fails closed; allowed no-rejection wording passes.
- Focused clippy:
  - `rtk cargo clippy -p vb_doc --all-targets -- -D warnings -D clippy::unwrap_used -D clippy::panic -D clippy::expect_used`: 0 errors.
  - `rtk cargo clippy -p vb_runtime --test vb_l2d7_joined_taint_propagation_red -- -D warnings -D clippy::unwrap_used -D clippy::panic -D clippy::expect_used`: 0 errors.

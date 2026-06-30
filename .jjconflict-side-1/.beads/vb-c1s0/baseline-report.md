# vb-c1s0 Baseline Report

Date: 2026-05-19
Source checkout: /home/lewis/src/velvet-ballistics

## cargo build --workspace

**Result: SUCCESS (with warnings)**

```
cargo build --workspace
  0 errors, 2 warnings
  5 crates compiled
```

Warnings:
- `unused import: vb_core::StepIdx` in `crates/vb_storage/src/journal/incident.rs:6`
- `function 'build_repair_hints_cli' is never used` in `crates/vb_cli/src/commands_incident.rs:57`

## cargo test --workspace

**Result: FAIL (compilation errors)**

```
cargo test --workspace
  8 errors, 3 warnings (7 crates)
```

Test compilation errors:
- `build_repair_hints_cli` not found (only `build_repair_hints` exists in vb_storage) — multiple call sites in `crates/vb_cli/src/commands_incident.rs`
- `vb_storage::JournalEvent` unresolved import in `crates/vb_storage/src/journal/incident.rs:123`
- Missing `vb_core::{RunId, ActionId}` imports in test code

## cargo clippy --workspace --all-targets

**Result: FAIL (lint errors)**

Multiple categories of errors:
- `unwrap_err()` on Result values (39+ instances)
- `panic!()` or assertion in functions returning Result (40+ instances)
- arithmetic operation with potential side-effects
- indexing that may panic (20+ instances)
- slicing that may panic (17+ instances)
- field assignment outside initializer for Default::default() instances (16+ instances)

Files with most violations:
- `crates/vb_core/src/action.rs` — unwrap_err, indexing
- `crates/vb_ui_snapshot/tests/comprehensive_unchecked.rs` — constant assertions, field assignments
- `crates/vb_ui_model/src/emitter/binary/tests.rs` — slicing

## Notes

- Build is green but test compilation fails due to mismatched function name (`build_repair_hints_cli` vs `build_repair_hints`)
- Clippy failures are concentrated in test code (unchecked indexing, panic! in Result-returning functions)
- Production code build succeeds cleanly

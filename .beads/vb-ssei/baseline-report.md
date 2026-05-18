bead_id: vb-ssei
phase: 1
updated_at: 2026-05-18T21:50:13Z
attempt: 1-of-7

# Baseline / pre-land report

Source checkout was not used for implementation. Isolated workspace: `/home/lewis/isolated/go-skill-vb-ssei-close-git`.

Known global gate debt observed during State 11:
- `moon ci` affected all tasks and failed `velvet-ballastics:fmt` on unrelated files: `crates/vb_codegen/src/tests.rs`, `crates/vb_storage/src/kani_recovery_hydrate.rs`, `crates/vb_storage/src/recovery/recover.rs`, `crates/vb_storage/src/recovery/recovery_unit_tests.rs`.
- `moon ci` failed `velvet-ballastics:check` on unrelated `crates/vb_storage/src/recovery/recovery_unit_tests.rs` unused/dead-code errors.

Local scoped gates for this bead pass; see `machine-gate-report.md`.

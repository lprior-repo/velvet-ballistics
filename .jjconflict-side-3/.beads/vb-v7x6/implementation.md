bead_id: vb-v7x6
phase: 10
attempt: 1-of-7

Implementation:
- `xtask/tests/ui_release_gates.rs`: robust xtask command builder with explicit workspace root for evidence/fixtures.
- `crates/vb_storage/src/recovery/recovery_unit_tests.rs`: removed unused import and unused helpers blocking all-target check.
- Rustfmt normalized pre-existing unformatted touched files.

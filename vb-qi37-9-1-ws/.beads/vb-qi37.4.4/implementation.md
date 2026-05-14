bead_id: vb-qi37.4.4
bead_title: runtime: Add admission durability errors
phase: State 6 - implementation
updated_at: 2026-05-11T00:00:00Z

# Implementation

Holzman references used:
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`

## Changes
- Added RED tests for admission durability diagnostics in `crates/vb_runtime/src/lib.rs`.
- Added `ADMISSION_DURABILITY_ERROR_RUNTIME_CODE` and `ADMISSION_HEADER_PERSISTENCE_FAILED_CODE`.
- Mapped `RuntimeError::StorageJournalAppend` to the admission durability runtime code and dedicated diagnostic code.

## Command Evidence
- RED before implementation: `rtk cargo test -p vb_runtime admission_header_persistence_failure_has_dedicated_diagnostic` failed on generic diagnostic code.
- RED before implementation: `rtk cargo test -p vb_runtime admission_durability_errors_have_stable_codes_distinct_from_generic_storage` failed with `Some("STORAGE_ERROR")` vs expected `Some("ADMISSION_DURABILITY_ERROR")`.
- GREEN after implementation: both commands passed, `1 passed` each.

## Risk
- Broader State 8 gates must decide whether remapping all `StorageJournalAppend` errors is too broad for the admission-specific contract.
# State 6 Repair Addendum

- Repaired the overly broad diagnostic mapping by introducing an explicit `RuntimeError::AdmissionHeaderPersistenceFailed` variant for admission-before-ack durability errors.
- Restored generic `StorageJournalAppend` diagnostic/runtime mapping so existing storage-journal regression behavior remains stable.
- Evidence: targeted admission diagnostic and storage diagnostic regression commands in `moon-report.md` passed.

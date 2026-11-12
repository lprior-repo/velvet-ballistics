STATUS: PASS_WITH_DEFERRED_GLOBAL

Primary bead blocker classification: REQUIRED_OBLIGATION_FAIL resolved.
- Before: scanner found DISCARD-004 at lines 78 and 111.
- After: scanner reports `NoViolationFound`; verify-standard passes.

Deferred global moon-ci debt:
- `moon ci` fmt failed on unrelated pre-existing formatting in `crates/vb_codegen/src/tests.rs`, `crates/vb_storage/src/recovery/recover.rs`, and `crates/vb_storage/src/recovery/recovery_unit_tests.rs`.
- `moon ci` check failed in `crates/vb_storage/src/recovery/recovery_unit_tests.rs` for unused import/dead code unrelated to touched harness file.
- Classification: DEFERRED_GLOBAL for this bead; not a regression from `crates/vb_storage/src/kani_recovery_hydrate.rs` repair.

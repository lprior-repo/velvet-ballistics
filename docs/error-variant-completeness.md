# Error Variant Completeness Audit

The executable audit for public `*Error` enums is dispersed across per-crate completeness suites: `crates/vb_compile/src/tests/error_variant_tests.rs` (every `CompileError` variant asserted exactly) and `crates/vb_compile/tests/v1_primitive_lowering.rs` (exact error variants for the contract taxonomy); `crates/vb_core/src/errors/tests.rs` (`CoreError` diagnostic codes per variant) with `crates/vb_core/src/diagnostic/tests_and_verification.rs` and `crates/workspace_tests/tests/diagnostic_code_ranges_test.rs` (registry uniqueness and global code partitioning); and `crates/vb_storage/src/error_tests.rs`, `crates/vb_storage/src/error_code_tests.rs`, and `crates/vb_storage/src/tests/error_variants_and_bdd.rs` (`JournalError` variants, codes, and exact assertions).

Coverage policy:

- Public diagnostic errors must be constructed variant-by-variant and checked for exact, unique diagnostic codes.
- Public constructible non-diagnostic errors must be constructed and passed through exhaustive no-wildcard matches.
- External wrapper variants are still covered by exhaustive matches, but are not all synthetically constructed when their wrapped third-party error has no stable public constructor.

Residual gap, recorded 2026-11-13: the dispersed suites do not yet cover every `JournalError` variant the policy above names. `crates/vb_storage/src/error_tests.rs` carries its own tested/untested audit; its untested list is partly stale (13 of its 29 listed variants are covered in `crates/vb_storage/src/tests/error_variants_and_bdd.rs`) and partly a genuine gap (no direct test in the three `JournalError` suites for `KeyCapacity`, `WriteLockPoisoned`, `QueueCapacity`, `QueueFull`, `QueueShutdown`, `WrongRun`, `SequenceOverflow`, `InvalidEvent`, `ArtifactMalformed`, `ArtifactChecksumMismatch`, `ArtifactNotFound`, `InvalidRunId`, `StrictDurabilityFailed`, `ProcessLockHeld`).

Documented external-wrapper exceptions:

- `CompileError::Parse`, wrapping `saphyr::ScanError`.
- `JournalError::Fjall`, wrapping `fjall::Error`.
- `JournalError::Encode`, wrapping `postcard::Error`.
- Wrapper variants over project-local errors are constructed with a stable local sample.

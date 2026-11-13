# Error Variant Completeness Audit

The executable audit for public `*Error` enums is dispersed across per-crate completeness suites: `crates/vb_compile/src/tests/error_variant_tests.rs` (every `CompileError` variant asserted exactly) and `crates/vb_compile/tests/v1_primitive_lowering.rs` (exact error variants for the contract taxonomy); `crates/vb_core/src/errors/tests.rs` (`CoreError` diagnostic codes per variant) with `crates/vb_core/src/diagnostic/tests_and_verification.rs` and `crates/workspace_tests/tests/diagnostic_code_ranges_test.rs` (registry uniqueness and global code partitioning); and `crates/vb_storage/src/error_tests.rs`, `crates/vb_storage/src/error_code_tests.rs`, and `crates/vb_storage/src/tests/error_variants_and_bdd.rs` (`JournalError` variants, codes, and exact assertions).

Coverage policy:

- Public diagnostic errors must be constructed variant-by-variant and checked for exact, unique diagnostic codes.
- Public constructible non-diagnostic errors must be constructed and passed through exhaustive no-wildcard matches.
- External wrapper variants are still covered by exhaustive matches, but are not all synthetically constructed when their wrapped third-party error has no stable public constructor.

Residual gap, corrected 2026-11-13: the three `JournalError` suites now directly cover 64 of the 66 `JournalError` variants: 46 constructed variant-by-variant with exact diagnostic codes in `crates/vb_storage/src/error_tests.rs`, and 18 more directly tested in the sibling suites (`crates/vb_storage/src/error_code_tests.rs` and `crates/vb_storage/src/tests/error_variants_and_bdd.rs`). The 14 variants the previous paragraph named as untested were miscounted: `WrongRun` and `ArtifactNotFound` already had direct diagnostic-code tests in `error_code_tests.rs`, and the `SequenceOverflow` BDD test never touches the variant (it only checks that `checked_add` returns `None`); all 14 are now covered, plus `ProcessLockIo` from the same untested list. The earlier "13 of 29" count was also wrong: the untested list held 31 entries, 13 of which are covered in `error_variants_and_bdd.rs`. Remaining gap, closed 2026-11-13: the 18 variants never entered in the audit (`DuplicateStagedKey`, `JournalBatchBytesExceeded`, `BatchAborted`, `ReplayKeyMismatch`, `ReplayEnvelopeSequenceMismatch`, `RecordKindPayloadMismatch`, `PostcardEncodeFailed`, `ArtifactValidation`, `ArtifactActionRegistry`, `WorkflowReconstruction`, `CompiledIrReadback`, `AdmissionAllocationFailed`, `OwnedInputs`, `IndexStatusStateCollision`, `InvalidConfig`, `UnsupportedReadOnly`, `EmergencyDenyMarkerUnsupportedVersion`, `EmergencyDenyMarkerMalformed`) are now constructed variant-by-variant with exact diagnostic codes in `crates/vb_storage/src/error_tests.rs` (51 new tests). The three suites now directly cover 64 of the 66 `JournalError` variants: 46 in `error_tests.rs` and 18 in the sibling suites. `Fjall` and `Encode` remain documented external-wrapper exceptions below.

Documented external-wrapper exceptions:

- `CompileError::Parse`, wrapping `saphyr::ScanError`.
- `JournalError::Fjall`, wrapping `fjall::Error`.
- `JournalError::Encode`, wrapping `postcard::Error`.
- Wrapper variants over project-local errors are constructed with a stable local sample.

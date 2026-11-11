# Contract

Move the existing fully test-gated diagnostic property module from `crates/vb_runtime/src/error/tests_diagnostics/proptest_vb_edvbj_diagnostic_code.rs` to `crates/vb_runtime/src/error/tests_diagnostics/diagnostic_code_tests.rs`, retaining module identifier `proptest_vb_edvbj_diagnostic_code` through an explicit `#[path = "tests_diagnostics/diagnostic_code_tests.rs"]` attribute in `crates/vb_runtime/src/error/tests_diagnostics.rs`.

The before and after fully qualified test catalog is exactly `vb_runtime::error::tests_diagnostics::proptest_vb_edvbj_diagnostic_code::unmapped_diagnostic_codes_invariant` and `vb_runtime::error::tests_diagnostics::proptest_vb_edvbj_diagnostic_code::every_event_kind_maps_to_the_internal_invariant_code`.

Preserve all 24 event-kind values exactly and compare the before/after domain: RunSubmitted, RunAdmission, CompatibilityDecision, TenantAdmission, RetryCheckpoint, RunFinished, RunFailed, RunCancelled, RunKilled, ActionScheduled, ActionCompleted, ActionScheduledTicket, ActionCompletedEnvelope, ActionFailed, ActionAbandoned, WaitScheduled, WaitResolved, AskScheduled, AskAnswered, AskTimedOut, SlotWritten, StepStarted, StepSucceeded, Resumed.

Preserve exactly three assertions per event kind: `CoreError::INTERNAL_INVARIANT_CODE`, `DiagnosticCode::new(0x1309)`, and `runtime_code() == None`; retain both the property-based test and the unconditional deterministic iteration over every event kind.

Remove indexed event selection by passing `event_kind: &'static str` into the assertion helper, using direct iteration over `EVENT_KINDS`, and using `sample::select(EVENT_KINDS.to_vec())` for the property strategy.

Baseline source SHA-256 before relocation is `cf334449b0ba98ecd0fd679bb8946fd1efe07fe27d757a7f490810acca65b4a6`; the moved file must retain the same test semantics and literal domain.

No production source, APIs, schemas, proof math, dependencies, gates, configurations, or unrelated files change.

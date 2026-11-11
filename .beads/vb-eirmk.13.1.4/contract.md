# vb-eirmk.13.1.4 - Exact Three-Event Owned Submission Roundtrip

## Problem

The existing owned-inputs submission roundtrip test verifies only two journal events (`RunAccepted` and `RunAdmission`), missing the `CompatibilityDecisionEvent` that `FjallJournal::commit_run_submission` appends for Strict policy. The test uses a lenient `find` search over events rather than exact ordered slice matching, and does not independently reconstruct expected event payloads from the fixture.

## Contract

Update `crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs` to:
- Assert exactly three submission events for Strict policy: `RunAccepted` (seq 0), `RunAdmission` (seq 1), `CompatibilityDecisionEvent` (seq 2)
- Replace the `find`-based admission lookup with ordered destructuring of all three events
- Independently construct expected event payloads from the fixture (run ID, workflow digest, artifact digest, input digest, policy pin, capabilities, policy, and compat decision) rather than serializing observed output
- Preserve all existing assertions: ownership graph restoration, reopen identity, input/output digests, roots, checkpoint, source equality, header retention, and workflow parts

## Scope

- Single test file: `crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs`
- No production source changes; test validates existing `stage_acceptance` behavior

## Acceptance

- Test `strict_owned_submission_reopens_with_exact_arenas_roots_taints_and_receipt` passes
- All three events matched by exact variant and sequence number
- All event fields asserted against independently reconstructed expected values
- No regressions in owned-inputs roundtrip or admission validation

# vb-qi37.6 Proof Plan Review Input

## Review Scope

- Review State 4 proof-planner-owned artifacts after State 3 ledger repair expanded the contract ledger to 24 rows.
- Inputs consumed: `contract.md`, `delivery-scope.jsonl`, `traceability-matrix.jsonl`, `proof-obligations.jsonl`, and `proof-obligations.planned.jsonl`.
- Outputs updated: `proof-strategy.md`, `proof-plan-review-input.md`, and `proof-obligations.planned.jsonl`.

## Required Review Checks

- Confirm `proof-obligations.planned.jsonl` has exactly 24 JSONL rows.
- Confirm these 24 IDs appear exactly once: `PRE-001-TLA-ENVELOPE`, `PRE-002-TLA-GATE15`, `PRE-003-FUZZ-SCHEMA`, `PRE-004-API-GRANTS`, `PRE-005-TLA-CONTRACT-SLICE`, `PRE-006-UI-SOURCE`, `POST-001-VERUS-EXACT`, `POST-002-TLA-GATE-DENIAL`, `POST-003-TLA-CARDINALITY-DENIAL`, `POST-004-TLA-MISSING-EXACT`, `POST-005-TLA-SUCCESS-JOURNAL`, `POST-006-TLA-DO-CHECKS`, `POST-007-TLA-NO-CONTRACT-DENY`, `POST-008-TLA-LEGACY-BYPASS`, `POST-009-UI-PARITY`, `INV-001-KANI-EXACT-SETUP`, `INV-002-KANI-CARDINALITY-SETUP`, `INV-003-TLA-GATE-CONTRACT`, `INV-004-VERUS-PERSISTENCE`, `INV-005-TLA-DENIAL-ATOMIC`, `INV-006-TLA-SHARD-CONTRACTS`, `INV-007-STATIC-LEGACY`, `INV-008-TLA-PUBLIC-GRANTS`, and `GAUNTLET-010`.
- Confirm no row status is `PASS` and no row claims completed evidence before State 10/11 execution.
- Confirm every row has `requirement_id`, `contract_clause`, `layer`, `checker`, `command`, `expected_evidence`, `required`, `mode`, `owner_state`, `rerun_from`, and `status`.
- Confirm all 23 PRE/POST/INV clauses from `contract.md` plus the release-gate row are covered.
- Confirm traceability rows and planned rows are aligned by contract clause/proof ID and evidence artifact.

## Formal Lane Checks

- TLA+ rows must use `tlc` commands against `verification/tla/CapabilityLifecycle.tla` and the focused configs: `CapabilityLifecycleAll.cfg`, `CapabilityLifecycleGateMismatch.cfg`, `CapabilityLifecycleExactProfile.cfg`, `CapabilityLifecycleExcessGrant.cfg`, `CapabilityLifecycleNoContract.cfg`, and `CapabilityLifecycleLegacyBypass.cfg`.
- TLA+ rows must include module/model/config metadata, variables, actions, invariants, finite state constraints, and refinement notes.
- Verus rows must use `verus verification/verus/capability_artifact_model.rs` and include proof/spec metadata, trusted boundary, and shell exclusions.
- Kani rows must not be treated as State 5 proof-writer failures. They are State 8 setup obligations with State 11 execution commands after setup.
- Fuzz rows must not be treated as State 5 proof-writer failures. They are State 8 fuzz-bin setup obligations with State 11 execution commands after setup.

## Kani/Fuzz Routing

- `INV-001-KANI-EXACT-SETUP`: executable setup check verifies `crates/vb_core/src/kani.rs` or `crates/vb_core/src/kani/mod.rs`; State 11 then runs `cargo kani -p vb_core --harness capability_name_grants_harness`.
- `INV-002-KANI-CARDINALITY-SETUP`: executable setup check verifies the same upstream Kani module wiring; State 11 then runs `cargo kani -p vb_runtime --harness check_capability_grants_exact_match`.
- `PRE-003-FUZZ-SCHEMA`: executable setup check verifies `fuzz/Cargo.toml` has bins for `capability_name_schema` and `capability_contract_schema`; State 11 then runs both `cargo fuzz run` commands.
- `GAUNTLET-010`: remains State 11 and is blocked until State 8 Kani/fuzz setup is repaired or explicit waivers are approved by the later verifier owner.

## Discovery Summary

- Scoped scan found auth/security, serialization, temporal state, queue/cancel, Kani, TLA+, and Verus triggers in the delivery-scope files.
- Focused TLA+ and Verus artifacts exist in the isolated checkout.
- Initial JSONL validation found 24 traceability rows, 24 contract obligation rows, 24 planned rows, no duplicate primary IDs, and no `PASS` statuses.

## Expected Reviewer Outcome

- Approve if the 24-row planned ledger is valid, traceable, and future-evidence-only.
- Reject if any ID is missing/duplicated, any row claims `PASS`, any TLA+/Verus row lacks executable command metadata, or Kani/fuzz setup is routed back to State 5 instead of State 8/11.

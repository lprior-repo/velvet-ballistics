# Archived Proof Review — vb-7m21 State 6 Attempt 3 (Rejected)

reviewer_skill: proof-reviewer
reviewer_invocation_id: proof-reviewer-vb-7m21-state6-003
writer_invocation_id: proof-writer-vb-7m21-state5-006
bead_id: vb-7m21
state: 6
sublane: proof-review
reviewed_artifacts_existed_before_start: true

## Findings

1. **CRITICAL — Official State 5 PASS is only ledger/trust schema repair, not proof discharge.**
   - Obligations: PO-vb-7m21-001 through PO-vb-7m21-039.
   - Artifacts: `.beads/vb-7m21/proof-writer-report.md`; `.beads/vb-7m21/proof-evidence.md`; `.beads/vb-7m21/state5-official-validator-evidence.json`.
   - Evidence: `state5-official-validator-evidence.json:23-28` records State 5 validator `PASS`, but `proof-evidence.md:3-5` says the evidence records ledger integrity only and does not claim State 6 approval or final formal proof success. `proof-writer-report.md:31-33` says attempt 6 does not upgrade smoke artifacts into final proof approval.
   - Required fix: provide raw verifier output or approved waivers for each required obligation. A State 5 structural validator pass cannot be used as State 6 proof evidence.

2. **CRITICAL — Required Verus obligations remain explicitly non-exec-bound trusted debt.**
   - Obligations: PO-vb-7m21-001, PO-vb-7m21-006, PO-vb-7m21-011, PO-vb-7m21-017, PO-vb-7m21-022, PO-vb-7m21-027, PO-vb-7m21-031, PO-vb-7m21-036.
   - Artifacts: `verification/verus/vb_7m21_001.rs` through `verification/verus/vb_7m21_008.rs`; `.beads/vb-7m21/trusted-base-ledger.jsonl`.
   - Evidence: `trusted-base-ledger.jsonl:9-16` records every Verus row as `trusted_kind: non_exec_binding_limit`, with `reviewer_disposition: pending_review`, because each artifact is a standalone smoke spec and not a production exec-function contract.
   - Required fix: bind Verus contracts to actual Rust implementation functions or obtain explicit approved waivers/downgrades. Pending trusted-base rows are not approval.

3. **CRITICAL — Required Kani obligations are still bounded by assumptions/abstractions and lack accepted successful raw Kani output.**
   - Obligations: PO-vb-7m21-002, PO-vb-7m21-007, PO-vb-7m21-012, PO-vb-7m21-018, PO-vb-7m21-023, PO-vb-7m21-028, PO-vb-7m21-032, PO-vb-7m21-037.
   - Artifacts: `crates/vb_storage/src/kani_vb_7m21_001.rs` through `crates/vb_storage/src/kani_vb_7m21_008.rs`; `.beads/vb-7m21/trusted-base-ledger.jsonl`; `.beads/vb-7m21/proof-evidence.md`.
   - Evidence: `trusted-base-ledger.jsonl:1-8` records active pending Kani assumptions, a disabled legacy harness scope reduction, and five bounded model abstractions. The active `proof-evidence.md:18-20` points only to State 5 validator evidence and does not include `VERIFICATION:- SUCCESSFUL` output for any Kani harness.
   - Required fix: provide successful raw `cargo kani` evidence for every required harness with assumptions, bounds, covers, stubs, and disabled checks audited, or provide explicit approved waivers.

4. **HIGH — Flux obligations remain standalone refinement sketches, not behavior-affecting checked implementation refinements.**
   - Obligations: PO-vb-7m21-003, PO-vb-7m21-008, PO-vb-7m21-013, PO-vb-7m21-019, PO-vb-7m21-024, PO-vb-7m21-033, PO-vb-7m21-038.
   - Artifacts: `verification/flux/vb_7m21_001.rs` through `verification/flux/vb_7m21_008.rs` except 006; `.beads/vb-7m21/trusted-base-ledger.jsonl`.
   - Evidence: `trusted-base-ledger.jsonl:17-23` records each Flux artifact as `trusted_kind: standalone_refinement_limit` with `reviewer_disposition: pending_review`, explicitly stating the artifacts are not attached to behavior-affecting Rust code.
   - Required fix: attach Flux refinements to behavior-affecting Rust code or provide checked bridge evidence that the standalone artifacts constrain implementation behavior.

5. **HIGH — Proptest lane still has an active classifier-only residual for storage behavior obligations.**
   - Obligations: PO-vb-7m21-020, PO-vb-7m21-025, PO-vb-7m21-029, PO-vb-7m21-034, PO-vb-7m21-039.
   - Artifact: `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs`; `.beads/vb-7m21/trusted-base-ledger.jsonl`.
   - Evidence: `trusted-base-ledger.jsonl:24` records `trusted_kind: test_oracle_abstraction` and states archived review found classifier-only residual concerns for five storage requirements.
   - Required fix: construct and observe actual public storage API behavior for side-index parity, sequence gap, duplicate, snapshot recovery, and manifest-keyspace outcomes.

## Provenance Review

- Current reviewer invocation: `proof-reviewer-vb-7m21-state6-003`.
- Reviewed writer invocation: `proof-writer-vb-7m21-state5-006` (`agent-invocation-ledger.jsonl:14`).
- No self-approval: reviewer skill/invocation differs from writer skill/invocation.
- `reviewed_artifacts_existed_before_start=true`: the active State 5 inputs existed before this State 6 review; the official State 5 PASS is recorded at `state5-official-validator-evidence.json:23-28`.
- Trust ledger rows 1-26 all retain `reviewer_disposition: pending_review`; this review does not approve those trusted surfaces.

## Raw Evidence References

- `proof-evidence.md:3-5` — ledger integrity only; no State 6 or final proof claim.
- `proof-writer-report.md:31-33` — no upgrade from smoke artifacts to final proof approval.
- `state5-official-validator-evidence.json:23-28` — official State 5 structural validator PASS.
- `trusted-base-ledger.jsonl:1-8` — Kani pending assumptions/abstractions.
- `trusted-base-ledger.jsonl:9-16` — Verus pending non-exec binding limitations.
- `trusted-base-ledger.jsonl:17-23` — Flux pending standalone refinement limitations.
- `trusted-base-ledger.jsonl:24` — proptest classifier-only residual.

## Review Decision

The proof package is rejected. The latest State 5 repair intentionally fixed ledger schema/hash/trust-marker validation only and explicitly disclaims final formal proof success. Required Verus, Kani, Flux, and proptest obligations remain pending trusted debt or disconnected from implementation behavior.

STATUS: REJECTED

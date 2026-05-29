# Proof Review — vb-7m21 State 6 Attempt 4

reviewer_skill: proof-reviewer
reviewer_invocation_id: proof-reviewer-vb-7m21-state6-004
writer_invocation_id: proof-writer-vb-7m21-state5-007
bead_id: vb-7m21
state: 6
sublane: proof-review
reviewed_artifacts_existed_before_start: true

## Findings

1. **CRITICAL — Official State 5 PASS is a structural validator pass, not proof discharge.**
   - Obligations: PO-vb-7m21-001 through PO-vb-7m21-039.
   - Artifacts: `.beads/vb-7m21/state5-official-validator-evidence.json`; `.beads/vb-7m21/proof-evidence.md`; `.beads/vb-7m21/proof-writer-report.md`; `.beads/vb-7m21/archive/proof-review-state6-attempt3-rejected.md`.
   - Evidence: `state5-official-validator-evidence.json:20-35` records State 5 validator `PASS`, but `proof-evidence.md:3-5` explicitly says the evidence records an archive/ledger repair and does not claim State 6 approval or final proof success. `proof-writer-report.md:32-34` states the pass does not upgrade smoke artifacts into final proof approval and preserves blockers. The archived Attempt 3 review at `archive/proof-review-state6-attempt3-rejected.md:61-65` already rejected the same proof package for the same reason.
   - Required fix: provide raw successful verifier output or approved waivers for every required obligation. A State 5 validator pass cannot be used as State 6 proof approval.

2. **CRITICAL — Required Verus obligations remain pending non-exec-bound trusted debt.**
   - Obligations: PO-vb-7m21-001, PO-vb-7m21-006, PO-vb-7m21-011, PO-vb-7m21-017, PO-vb-7m21-022, PO-vb-7m21-027, PO-vb-7m21-031, PO-vb-7m21-036.
   - Artifacts: `verification/verus/vb_7m21_001.rs` through `verification/verus/vb_7m21_008.rs`; `.beads/vb-7m21/trusted-base-ledger.jsonl`.
   - Evidence: `trusted-base-ledger.jsonl:9-16` records every Verus row with `trusted_kind: non_exec_binding_limit`, `status: active`, and `reviewer_disposition: pending_review`. `proof-evidence.md:20-24` preserves these blockers instead of claiming discharge.
   - Required fix: bind Verus contracts to actual Rust implementation functions or provide explicit approved waivers/downgrades. Pending trusted-base rows are not approval.

3. **CRITICAL — Required Kani obligations remain assumption/abstraction-limited and lack accepted raw successful Kani output.**
   - Obligations: PO-vb-7m21-002, PO-vb-7m21-007, PO-vb-7m21-012, PO-vb-7m21-018, PO-vb-7m21-023, PO-vb-7m21-028, PO-vb-7m21-032, PO-vb-7m21-037.
   - Artifacts: `crates/vb_storage/src/kani_vb_7m21_001.rs` through `crates/vb_storage/src/kani_vb_7m21_008.rs`; `.beads/vb-7m21/trusted-base-ledger.jsonl`; `.beads/vb-7m21/proof-evidence.md`.
   - Evidence: `trusted-base-ledger.jsonl:1-8` records active Kani assumptions, a disabled legacy harness scope reduction, and bounded model abstractions, all with `reviewer_disposition: pending_review`. `proof-evidence.md:22-24` says these obligations remain assumption/abstraction-limited unless later raw successful `cargo kani` output is supplied.
   - Required fix: provide successful raw `cargo kani` evidence for every required harness with assumptions, bounds, covers, stubs, disabled checks, and harness inventory audited, or provide explicit approved waivers.

4. **HIGH — Flux obligations remain standalone refinement sketches, not checked behavior-affecting implementation refinements.**
   - Obligations: PO-vb-7m21-003, PO-vb-7m21-008, PO-vb-7m21-013, PO-vb-7m21-019, PO-vb-7m21-024, PO-vb-7m21-033, PO-vb-7m21-038.
   - Artifacts: `verification/flux/vb_7m21_001.rs` through `verification/flux/vb_7m21_008.rs` except 006; `.beads/vb-7m21/trusted-base-ledger.jsonl`.
   - Evidence: `trusted-base-ledger.jsonl:17-23` records active `standalone_refinement_limit` rows with `reviewer_disposition: pending_review`; `proof-evidence.md:24` preserves the Flux blockers as standalone refinement sketches.
   - Required fix: attach Flux refinements to behavior-affecting Rust code or provide checked bridge evidence that the standalone artifacts constrain implementation behavior.

5. **HIGH — Proptest lane still has an active classifier-only/test-oracle abstraction residual for storage behavior obligations.**
   - Obligations: PO-vb-7m21-020, PO-vb-7m21-025, PO-vb-7m21-029, PO-vb-7m21-034, PO-vb-7m21-039.
   - Artifacts: `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs`; `.beads/vb-7m21/trusted-base-ledger.jsonl`.
   - Evidence: `trusted-base-ledger.jsonl:24` records active `trusted_kind: test_oracle_abstraction` with pending review; `proof-evidence.md:25` preserves classifier-only residual concerns for public storage behavior.
   - Required fix: construct and observe actual public storage API behavior for side-index parity, sequence gap, duplicate, snapshot recovery, and manifest-keyspace outcomes.

## Provenance Review

- Current reviewer invocation: `proof-reviewer-vb-7m21-state6-004`.
- Reviewed writer invocation: `proof-writer-vb-7m21-state5-007` (`agent-invocation-ledger.jsonl:16`).
- No self-approval: the latest writer entry is `skill: proof-writer`; this review is `skill: proof-reviewer`.
- `reviewed_artifacts_existed_before_start=true`: the active State 5 attempt 7 artifacts existed before this State 6 review.
- Trust ledger rows 1-26 all retain `reviewer_disposition: pending_review`; this review does not approve those trusted surfaces.
- Raw inspection evidence captured during this review: `OBLIGATIONS 39 REQUIRED 39`; `OFFICIAL_STATE5_STATUS PASS`; `LATEST_LEDGER_SEQ 16 proof-writer-vb-7m21-state5-007 proof-writer 5 final-review-repair`; `TRUST_ROWS 26 PENDING 26`.

## Raw Evidence References

- `state5-official-validator-evidence.json:20-35` — official State 5 structural validator PASS.
- `proof-evidence.md:3-5` — archive/ledger repair only; no State 6 or final proof claim.
- `proof-evidence.md:20-25` — preserved Verus/Kani/Flux/proptest blockers.
- `proof-writer-report.md:32-34` — no upgrade from smoke artifacts to final proof approval.
- `trusted-base-ledger.jsonl:1-8` — Kani pending assumptions/abstractions and disabled legacy harness scope.
- `trusted-base-ledger.jsonl:9-16` — Verus pending non-exec binding limitations.
- `trusted-base-ledger.jsonl:17-23` — Flux pending standalone refinement limitations.
- `trusted-base-ledger.jsonl:24` — proptest classifier-only/test-oracle residual.
- `archive/proof-review-state6-attempt3-rejected.md:61-65` — prior State 6 rejection remains substantively unresolved.

## Review Decision

The proof package is rejected. Attempt 7 made the State 5 validator pass by honestly archiving the prior rejection and normalizing ledger/trust metadata, while explicitly preserving the proof blockers. No required proof obligation is newly discharged by raw verifier output or approved waiver.

STATUS: REJECTED

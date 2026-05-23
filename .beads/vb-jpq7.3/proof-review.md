# Proof Review: vb-jpq7.3

Reviewer: proof-reviewer  
Reviewer invocation: proof-reviewer-gpt55-2026-05-23-vb-jpq7-3-canonical-schema-post-approval-rereview  
Date: 2026-05-23  
Review state: final proof-artifact/evidence re-review after canonical proof-plan schema repair and proof-plan approval  
Prior proof-plan approval: `.beads/vb-jpq7.3/proof-plan-review.md` (`review_state: approved`, approved final status)  
Scope inspected: `.beads/vb-jpq7.3/proof-plan-review.md`, `.beads/vb-jpq7.3/verifier-lane-review.md`, `.beads/vb-jpq7.3/verifier-lane-review.jsonl`, `.beads/vb-jpq7.3/proof-obligations.planned.jsonl`, `.beads/vb-jpq7.3/verifier-lane-decisions.jsonl`, `.beads/vb-jpq7.3/waiver-candidates.jsonl`, `.beads/vb-jpq7.3/verification-ledger.jsonl`, `.beads/vb-jpq7.3/traceability-matrix.jsonl`, `.beads/vb-jpq7.3/proof-to-implementation.md`, `.beads/vb-jpq7.3/trusted-base-plan.md`, `.beads/vb-jpq7.3/agent-invocation-ledger.jsonl`, `verification/tla/EngineYamlRecovery.tla`, `verification/tla/EngineYamlRecovery.cfg`, `verification/verus/vb_jpq724_events_for_run_production.rs`, `verification/verus/recovery_hydration_contracts.rs`, `crates/vb_storage/src/kani_admission.rs`, `crates/vb_storage/src/kani_recovery_hydrate.rs`, `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs`, Kani raw log `/home/lewis/.local/share/opencode/tool-output/tool_e543ab843002yJmWdm7rPpi1ed`, and latest Moon raw log `/home/lewis/.local/share/opencode/tool-output/tool_e54cfc867001em3UkY7dnDZZ7z`.

## Findings

### ACCEPTED — Canonical proof-plan schema repair is reviewed and proof-plan-approved

- Obligations: all proof-plan rows.
- Artifacts: `.beads/vb-jpq7.3/proof-obligations.planned.jsonl`, `.beads/vb-jpq7.3/verifier-lane-decisions.jsonl`, `.beads/vb-jpq7.3/verifier-lane-review.jsonl`, `.beads/vb-jpq7.3/waiver-candidates.jsonl`, `.beads/vb-jpq7.3/verification-ledger.jsonl`, `.beads/vb-jpq7.3/proof-plan-review.md`, `.beads/vb-jpq7.3/verifier-lane-review.md`.
- Evidence: JSONL parse/count audit found 16 `proof-obligation/v1` rows, 72 `verifier-lane-decision/v1` rows with `risk_tags`, 72 `verifier-lane-review/v1` rows all `accepted`, 6 `waiver-candidate/v1` rows all `behavior_affecting: false`, 35 `verification-ledger/v1` rows, and 9 `traceability/v1` rows. `proof-plan-review.md` is approved and cites latest Moon `tool_e54cfc867001em3UkY7dnDZZ7z` plus scoped Kani `tool_e543ab843002yJmWdm7rPpi1ed`.
- Review judgment: accepted. The prior proof-plan schema/lane-review/stale-approval blockers are resolved for the proof package.

### LIMITATION ACCEPTED — Verus is auxiliary/spec-seam evidence only, not production-bound exec proof

- Obligations: `obl-verus-replay-001`, `obl-verus-recovery-001`.
- Artifacts: `verification/verus/vb_jpq724_events_for_run_production.rs`, `verification/verus/recovery_hydration_contracts.rs`, `.beads/vb-jpq7.3/proof-obligations.planned.jsonl:2-3`, `.beads/vb-jpq7.3/verification-ledger.jsonl:25-26`, `.beads/vb-jpq7.3/proof-to-implementation.md:184-190`.
- Evidence: ledger records `5 verified, 0 errors` for replay and `10 verified, 0 errors` for recovery. Targeted trust-marker search found no `assume`, `external_body`, `axiom`, or `admit` in the two reviewed Verus files; `recovery_hydration_contracts.rs` contains only a comment declaring storage/journal ordering trusted boundaries.
- Review judgment: accepted only as auxiliary mirror/spec-seam evidence. These artifacts do not prove `FjallJournal`, `RunFrame`, Fjall iteration, postcard, codec internals, or live replay/hydration.

### LIMITATION ACCEPTED — TLA+ is bounded abstract temporal evidence (`MaxSeq = 3`)

- Obligation: `obl-tla-recovery-001`.
- Artifacts: `verification/tla/EngineYamlRecovery.tla`, `verification/tla/EngineYamlRecovery.cfg`, `.beads/vb-jpq7.3/proof-obligations.planned.jsonl:1`, `.beads/vb-jpq7.3/verification-ledger.jsonl:24`.
- Evidence: model has `TypeOK`, typed snapshot-failure transitions, explicit `snapshot_seq = MaxSeq` overflow fail-closed transition, missing-first-tail fail-closed transition, strict tail-start invariant, typed failed-closed error invariant, liveness property, and `CHECK_DEADLOCK FALSE`. Ledger reports TLC PASS: `87074 states generated`, `43531 distinct states`, depth `6`, no errors.
- Review judgment: accepted as bounded abstract fail-closed evidence only. It does not prove Fjall key ordering, digest/postcard implementations, Rust range iteration, or unbounded `u64` arithmetic beyond the modeled overflow boundary.

### LIMITATION ACCEPTED — Kani proves scoped allocation-free seams only

- Obligations: `obl-kani-replay-next-001`, `obl-kani-replay-limit-001`, `obl-kani-snapshot-metadata-001`, `obl-kani-taint-001`, `obl-kani-recovery-presence-001`, `obl-kani-admission-001`.
- Artifacts: `crates/vb_storage/src/kani_recovery_hydrate.rs`, `crates/vb_storage/src/kani_admission.rs`, `.beads/vb-jpq7.3/verification-ledger.jsonl:28-29`, raw log `/home/lewis/.local/share/opencode/tool-output/tool_e543ab843002yJmWdm7rPpi1ed`.
- Evidence: raw Kani log contains 12 `VERIFICATION:- SUCCESSFUL` summaries and 12 `Complete - 1 successfully verified harnesses, 0 failures, 1 total.` summaries. Marker audit found 0 `VERIFICATION:- FAILED`, 0 `UNSATISFIED`, and 0 `FAILURE`. Source uses `kani::any()` / `kani::Arbitrary`; the reviewed assumptions constrain positive replay limits or intentionally force mismatch branches rather than erasing bad outcomes.
- Review judgment: accepted only for the named seams: next-sequence overflow, replay push-limit arithmetic, snapshot run-mismatch metadata, bounded tail metadata predicates, recovery-data presence, finite taint-read lattice, and adjacent admission digest/flag invariants. The 3 `kani_admission::*` harnesses are adjacent admission evidence and do not close storage replay/recovery claims.

### ACCEPTED — Current behavior/global evidence is latest and non-stale

- Obligations: `obl-test-workspace-contract-001`, `obl-test-storage-replay-001`, `obl-test-storage-recovery-001`, `obl-test-storage-trimming-001`, `obl-test-storage-durability-001`, `obl-source-scan-discard-001`, `obl-moon-ci-001`.
- Artifacts: `.beads/vb-jpq7.3/verification-ledger.jsonl`, `.beads/vb-jpq7.3/proof-to-implementation.md`, `.beads/vb-jpq7.3/traceability-matrix.jsonl`, `.beads/vb-jpq7.3/trusted-base-plan.md`, raw Moon log `/home/lewis/.local/share/opencode/tool-output/tool_e54cfc867001em3UkY7dnDZZ7z`.
- Evidence: latest ledger row `vl-035` is `GLOBAL_PASS` for `moon ci` at `tool_e54cfc867001em3UkY7dnDZZ7z` with `Tasks: 25 completed (3 cached)`, `12169 tests run: 12169 passed (5 slow), 0 skipped`, `test integrity: PASS base=HEAD`, two `NoViolationFound` markers, supply-chain completed, and source-length PASS with deferred-global notices only. Public contract row `vl-003` reports `11 passed; 0 failed`. Traceability maps all behavior requirements to source refs, commands, and evidence.
- Review judgment: accepted. Current proof artifacts do not overclaim older `12167` / 10-test evidence as latest closure evidence. Historical older logs remain referenced as prior/superseded context only. Non-proof reviewer artifacts such as `black-hat-review.md` and `qa-review.md` still contain stale closure-packaging rejection prose from earlier audits; that is outside this proof-artifact approval and should be refreshed before final evidence packaging, but it is not a proof-obligation blocker after the approved proof-plan rerun.

### ACCEPTED — Versioned slot-write extra envelope repair closes the prior corrupt-extra downgrade

- Obligations: `obl-test-workspace-contract-001`, `obl-test-storage-recovery-001`, `obl-source-scan-discard-001`; requirements `vb-jpq7.3:taint-read-fail-closed` and `vb-jpq7.3:no-silent-discard`.
- Artifacts: `crates/vb_storage/src/slot_extra.rs`, `crates/vb_storage/src/recovery/replay/summary.rs`, `crates/vb_storage/src/recovery/types.rs`, `crates/vb_runtime/src/journal/chunk_002.rs`, `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs`, `.beads/vb-jpq7.3/verification-ledger.jsonl:3,30,33-35`.
- Evidence: public contract source has 11 `#[test]` cases and includes both `given_full_journal_slot_taint_metadata_is_corrupt_when_hydrating_then_recovery_fails_closed` and `given_legacy_collect_frame_extra_when_hydrating_full_journal_then_extra_is_not_corrupt_taint`. Ledger records direct full-journal recovery (`5 passed`), runtime envelope tests, public contract `11 passed`, and latest Moon `12169` tests.
- Review judgment: accepted. Corrupt prefixed taint payloads now fail closed while legacy unprefixed frame-extra remains compatible.

## Waiver / Non-Applicability Review

Accepted: `WV-MIRI-001`, `WV-LOOM-001`, `WV-FLUX-001`, `WV-PROPTEST-001`, `WV-FUZZ-001`, and `WV-PERF-001` are non-behavior-affecting, include compensating evidence and promotion triggers, and do not waive behavior tests, scoped Kani, or canonical Moon CI. Live Fjall/`RunFrame`/codec limitations remain explicit in `proof-to-implementation.md:184-190`, `trusted-base-plan.md`, and this review.

## Commands Run During This Review

- Loaded `proof-reviewer` skill.
- `python3` JSONL parse/count audit over proof obligations, verifier lane decisions, verifier lane reviews, waiver candidates, verification ledger, traceability matrix, and agent invocation ledger.
- `python3` raw marker audit over latest Moon and Kani logs: Moon marker counts matched `12169` latest closure evidence; Kani marker counts matched 12 successful harnesses with no failed/unsatisfied markers.
- Targeted `grep`/read inspections for TLA bounds/invariants, Verus trust markers, Kani assumptions/non-vacuity markers, public 11-test contract functions, stale `12167`/`10-test`/rejection references, proof-to-implementation limitations, traceability, and trusted-base declarations.

## Blockers

None for proof-artifact approval. Closure packaging still should refresh stale non-proof reviewer prose in `black-hat-review.md` / `qa-review.md` before claiming the entire assurance bundle is final.

## Verdict

APPROVED with explicit limitations. The proof package is acceptable only under the scoped interpretation recorded above: Verus is auxiliary/spec-seam evidence, TLA+ is bounded abstract `MaxSeq = 3` evidence, Kani is limited to allocation-free seams, and live Fjall/`RunFrame`/codec behavior is closed by behavior tests, source scans, and trusted-base declarations. Current latest closure evidence is Moon `/home/lewis/.local/share/opencode/tool-output/tool_e54cfc867001em3UkY7dnDZZ7z` (`12169` tests) and public contract `11 passed`; older `12167` / `10-test` evidence is historical/superseded only.

STATUS: APPROVED

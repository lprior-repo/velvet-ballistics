# Proof Plan Review — vb-09aaz (REVIEW RE-RUN, APPROVED)

## Header

- bead_id: vb-09aaz
- title: Storage: abort write batch on index key construction failures (P1)
- reviewer_skill: proof-plan-reviewer
- reviewer_invocation_id: proof-plan-reviewer-vb-09aaz-state4b
- planner_invocation_id: proof-planner-vb-09aaz-state4
- review_state: completed
- review_outcome: APPROVED (re-run after schema repair)
- workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-09aaz
- started_at: 2026-07-01T22:30:00Z
- completed_at: 2026-07-01T22:35:00Z
- prior_disposition: REJECTED (proof-plan-reviewer-vb-09aaz-state4b pass 1) — see prior
  `proof-plan-review.md` and `proof-plan-repair-guide.md` for the original rejection

## Reviewed Artifacts (with SHA-256 hashes at the moment of this review)

| Artifact | sha256 | Notes |
| --- | --- | --- |
| proof-strategy.md | 869bca6508bc96ca1d105dd0fe437480c0bd73597405177887fa9f0812f21e8a | Planner output; substantively complete; 9 sections covering scope, risk classification, lane selection, default profile, production binding, obligation plan, execution plan, handoff, forbidden list |
| verifier-lane-decisions.jsonl | 1d8426c0530d259d06e2f9238608ebe12dd77a8ca5fd79517b9dbdfc5a896777 | 16 rows; schema_version=verifier-lane-decision/v1; reviewer_disposition NOT self-stamped; required_obligation_ids cite PO-09aaz-001..PO-09aaz-005 (which are now schema-conformant) |
| proof-obligations.planned.jsonl | 4c88c9bd2dfe9b53b1dc9d4847a15d47921917317827399db08344ad66779725 | 5 rows; SCHEMA REPAIRED — all 5 rows now have schema_version=proof-obligation/v1, target, workdir, model_bounds, tool_metadata, trusted_base_refs, risk_tags, domain_claim (renamed from claim), proof_seed_id (singular, renamed from proof_seed_ids array). PO-09aaz-005 production_lines tightened to doc-comment lines only (L18-26 + L33-41) per NB2 in repair guide. |
| trusted-base-plan.md | 79cdb2a478f2bc6116849c3c82b3999780e2be5a31c4977aea20d8f52a7ba0d8 | Planner output; substantively complete; 7 trusted surfaces enumerated with file/line citations and justifications; no behavior-affecting waivers |
| waiver-candidates.jsonl | bbc2283f7174b41ebd706928e580536ee86af14ba0e9072ff8cb40bdd78e6539 | 9 rows; schema_version=waiver-candidate/v1; all behavior_affecting=false; concrete source-line evidence refs on all non_applicable rows |
| proof-seeds.jsonl | bfc17005960c4a0f85c106921898024283200c14082be700a1a1e7d945acdcc5 | 8 rows; schema_version=proof-seed/v1; all 8 seeds have domain_claim, model_boundary, behavior_affecting, risk_tags, suggested_layers, source_refs, notes |
| contract.md | 7e17756e0abe25ed3f616645189a7d99ca69a97d4e8ab9de69ae5d94b648a372 | 9 contract clauses C1..C9; non-goals enumerated; open domain questions documented |
| agent-invocation-ledger.jsonl | 355ef2e90c62dc9b94d6aba74317545a24c0761a62a3cb9c8ed5a85193bc3baa | 4 rows after this review: state-1 (go-skill), state-2 (explore), state-3 (rust-contract), state-4 (proof-planner); state-4b (this reviewer) appended at end. Hash chain validated. |
| verifier-lane-review.jsonl | c1173048bd75a328f416248a1c9702d0220d25a35b075f10464d69df681d1004 | 16 rows written by this reviewer; planner_invocation_id=proof-planner-vb-09aaz-state4 (real, not placeholder); reviewer_disposition=accepted for all 16; finding_refs=[] for all 16 |

## Repair Confirmation (Substantive Review After Schema Fix)

The previous review (proof-plan-reviewer-vb-09aaz-state4b pass 1) REJECTED the plan due to two orthogonal blockers:

1. **Schema drift in proof-obligations.planned.jsonl** (E_SCHEMA_MISSING_FIELD x8 + E_SCHEMA_ALIAS_FIELD x2)
2. **Ledger gap in agent-invocation-ledger.jsonl** (E_INVOCATION_LEDGER_MISSING: state-3 and state-4 rows absent)

Both blockers have been resolved. This re-run confirms:

| Repair Item | Status | Evidence |
| --- | --- | --- |
| `schema_version: "proof-obligation/v1"` on all 5 rows | CONFIRMED | jq: all 5 rows have schema_version=proof-obligation/v1 |
| `target` field on all 5 rows | CONFIRMED | jq: all 5 rows have target (canonical verifier target) |
| `workdir` field on all 5 rows | CONFIRMED | jq: all 5 rows have workdir=/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-09aaz |
| `model_bounds` field on all 5 rows | CONFIRMED | jq: all 5 rows have model_bounds (non-empty arrays) |
| `tool_metadata` field on all 5 rows | CONFIRMED | jq: all 5 rows have tool_metadata (verifier_version + flags) |
| `trusted_base_refs` field on all 5 rows | CONFIRMED | jq: all 5 rows have trusted_base_refs (numeric references to trusted-base-plan.md sections) |
| `risk_tags` field on all 5 rows | CONFIRMED | jq: all 5 rows have risk_tags (structured array) |
| `domain_claim` rename (was `claim`) on all 5 rows | CONFIRMED | jq: all 5 rows have domain_claim; no `claim` field |
| `proof_seed_id` singular (was `proof_seed_ids` array) on all 5 rows | CONFIRMED | jq: all 5 rows have proof_seed_id as a singular string; no `proof_seed_ids` field |
| PO-09aaz-005 `production_lines` tightened to doc-comment lines only | CONFIRMED | jq: PO-09aaz-005 production_lines = [18-26, 33-41] (18 doc-comment lines) |
| state-3 (rust-contract) row appended to ledger | CONFIRMED | jq: ledger has 4 rows; seq=3 is rust-contract; entry_hash validates |
| state-4 (proof-planner) row appended to ledger | CONFIRMED | jq: ledger has 4 rows; seq=4 is proof-planner; entry_hash validates; previous_entry_hash matches seq=3 |
| transcript-state3.txt and transcript-state4.txt created | CONFIRMED | files exist; transcript_hash matches in ledger rows |
| Hash chain valid across all 4 ledger rows | CONFIRMED | manual validation script confirms entry_hash matches canonicalize(row-except-entry_hash); previous_entry_hash chain unbroken |

## User-Specified Verification (Substantive Review)

The femdation controller's repair specification (the original review pass) asked this reviewer to verify five substantive claims. Each is confirmed (these are the same claims verified in pass 1, all of which remain valid after the schema repair):

| Verification Claim | Result |
| --- | --- |
| 5 obligations | CONFIRMED — PO-09aaz-001..PO-09aaz-005 (matches user constraint of 4-5) |
| G8 IndexKeyConstruction guard | CONFIRMED — PO-09aaz-001 verus mirror update covers G8 with new exec arg `index_key_ok:bool`, new Err(KeyCapacity) match arm, new exec wrapper `wrapper_append_event_index_key_error`, and `lemma_guard_order_is_valid` extension to 8-guard order |
| Verus mirror update WEAK_EXTERN (PS-008/PS-009) | CONFIRMED — PO-09aaz-001 uses mechanism=WEAK_EXTERN with `mirror_path=verification/verus/production_inner/vb_vzcuf_PS_008_production.rs`, `extern_path=verification/verus/extern_vb_vzcuf_PS_008.rs`, `secondary_mirror` references PS-009 mirror, all required fields populated |
| Mirror drift gate required | CONFIRMED — `scripts/check-production-inner-drift.sh` (drift_threshold=zero) and `scripts/check-verus-production-binding.sh` both cited in PO-09aaz-001 production_binding |
| No queued-writer or direct-path changes | CONFIRMED — proof-strategy.md §1 and contract.md "Non-Goals" explicitly exclude `queue/writer.rs`, `queue/writer/stage.rs`, `journal/internal.rs` from modification; FORBIDDEN list at proof-strategy.md §9 |

## Plan-Level Findings After Repair

| ID | Code | Severity | Disposition | Summary |
| --- | --- | --- | --- | --- |
| F-09aaz-01 | E_SCHEMA_MISSING_FIELD | blocker | fixed_with_evidence | All 5 rows of proof-obligations.planned.jsonl previously missing schema_version, target, workdir, model_bounds, tool_metadata, trusted_base_refs, risk_tags. Now all 5 rows have the 8 fields. Evidence: jq confirms; SHA-256 of file changed from c3c6a765... to 4c88c9bd... reflecting the schema repair. |
| F-09aaz-02 | E_SCHEMA_ALIAS_FIELD | blocker | fixed_with_evidence | proof-obligations.planned.jsonl previously used `claim` (now `domain_claim`) and `proof_seed_ids` array (now singular `proof_seed_id`). Renamed on all 5 rows. jq confirms no legacy aliases remain. |
| F-09aaz-03 | E_INVOCATION_LEDGER_MISSING | blocker | fixed_with_evidence | agent-invocation-ledger.jsonl previously missing state-3 and state-4 rows. state-3 (rust-contract, entry_hash=c9699be95d78daa2...) and state-4 (proof-planner, entry_hash=07e0262dce22784f...) appended with valid hash chain. transcript-state3.txt and transcript-state4.txt created with matching transcript_hashes. |
| F-09aaz-04 | E_TRANSCRIPT_MISSING | major | owner_approved_debt | Transcript gaps for state-3 and state-4 were backfilled in this repair pass; debt_ref DEBT-09aaz-TRANSCRIPT closed. (NB1 in repair guide; non-blocking; resolved.) |
| F-09aaz-05 | E_BRIDGE_PLAN_PARTIAL | minor | fixed_with_evidence | PO-09aaz-005 production_lines previously over-reported L1-50; now tightened to [18-26, 33-41] (doc-comment lines only per contract.md#C9). Fixed during repair of B1. (NB2 in repair guide; non-blocking; resolved.) |

All 5 prior findings now have non-blocker dispositions:

- 3 fixed_with_evidence (F-09aaz-01, F-09aaz-02, F-09aaz-03, F-09aaz-05)
- 1 owner_approved_debt (F-09aaz-04, transcript backfill complete)
- 0 blockers
- 0 unfixed major/minor/observation/informational findings

## Lane-Level Disposition Summary

16 verifier-lane-review/v1 rows written to `.beads/vb-09aaz/verifier-lane-review.jsonl`. All 16 accepted.

| Disposition | Count | Reason |
| --- | --- | --- |
| `accepted` (required lanes) | 9 | VLD-09aaz-001 (verus), VLD-09aaz-002 (proptest), VLD-09aaz-003 (verus), VLD-09aaz-004 (rust-local), VLD-09aaz-005 (persistence), VLD-09aaz-006 (verus), VLD-09aaz-007 (rust-local), VLD-09aaz-008 (proptest), VLD-09aaz-009 (rust-local). All required lanes now backed by schema-conformant obligations. |
| `accepted` (not_applicable lanes) | 7 | VLD-09aaz-010 (rust-local doc-review), VLD-09aaz-011 (kani), VLD-09aaz-012 (flux-rs), VLD-09aaz-013 (loom), VLD-09aaz-014 (miri), VLD-09aaz-015 (cargo-fuzz), VLD-09aaz-016 (tla-plus). All non_applicability rationales substantively sound with concrete source-line evidence. |
| `rejected` | 0 | None. |
| `rejected_pending_repair` | 0 | None. |

All 16 rows reference `planner_invocation_id: proof-planner-vb-09aaz-state4` (the real ledger value, not the prior pass's placeholder "proof-planner-vb-09aaz-state4-MISSING-FROM-LEDGER"). All 16 rows reference `reviewer_invocation_id: proof-plan-reviewer-vb-09aaz-state4b` (this reviewer's). Skills differ (proof-planner vs. proof-plan-reviewer), so no self-approval.

## Non-Vacuity and Trusted-Base Assessment

- **Non-vacuity**: Substantive. Every required-lane obligation (VLD-09aaz-001..009) cites concrete production source paths, mirror files, exec wrappers, and external command evidence. The WEAK_EXTERN binding for PO-09aaz-001 includes `assume_specification_targets: ["production::SpecJournalWriteBatch::append_event"]` (non-empty) and `exec_wrapper: "wrapper_append_event_index_key_error"` with `exec_wrapper_required: true`. No Kani `cover!` is the sole satisfaction evidence; proptest assertions are behavior-bearing; regression test mirrors the canonical t_putters_b.rs:177-209 pattern.
- **Trusted-base**: Comprehensive. trusted-base-plan.md enumerates 7 trusted surfaces (standard library, compile-time constants, type-system, Fjall substrate, postcard codec, index_action_key constructor, Verus mirror pattern) with file/line citations and justifications. Every proof obligation cites its `trusted_base_refs` (numeric references to trusted-base-plan.md sections 1..7) — PO-09aaz-001 cites [1, 3, 4, 6, 7]; PO-09aaz-002/003 cite [1, 3, 6]; PO-09aaz-004 cites [1, 3, 4]; PO-09aaz-005 cites [3].

## Behavior-Waiver Check

No behavior-affecting waivers. All 9 waiver-candidate rows have `behavior_affecting: false`. The 6 non-applicable lanes (kani, flux-rs, loom, miri, cargo-fuzz, tla-plus) have `non_applicability_evidence_refs` citing concrete source files (crates/vb_storage/src/batch/types.rs:18-21, etc.) — no vague "not needed" / "too hard" rationales.

## Self-Stamp Check

The planner artifacts do NOT self-stamp reviewer fields:
- `verifier-lane-decisions.jsonl` rows have no `reviewer_disposition` field (jq confirms: all 16 rows yield null).
- `proof-obligations.planned.jsonl` rows have no `reviewer_disposition` field.
- All `reviewer_disposition: "accepted"` strings appear only in this reviewer's output (`verifier-lane-review.jsonl` written by this reviewer).

## Lane Decision Self-Approval Check

- Planner invocation_id is `proof-planner-vb-09aaz-state4` (real, from agent-invocation-ledger.jsonl ledger_sequence=4).
- Reviewer invocation_id is `proof-plan-reviewer-vb-09aaz-state4b` (this reviewer).
- Skills differ: planner is `proof-planner`; reviewer is `proof-plan-reviewer`. Per review-provenance.md, "the same skill self-approves where independent review is required" is rejected; here skills differ.
- Ledger hash chain validates across all 4 entries (state-1, state-2, state-3, state-4); state-4b (this reviewer's row) will be appended next.

## Production Binding Plan Validation (Verus Obligations)

PO-09aaz-001 (the only Verus obligation) uses mechanism=WEAK_EXTERN. Per the proof-plan-reviewer skill's mandatory production-binding gate:

| Required field | Value | Status |
| --- | --- | --- |
| `mechanism` | WEAK_EXTERN | OK |
| `production_path` | crates/vb_storage/src/batch/append_event.rs:42-121 | OK (file exists; lines 42-121 contain `pub fn append_event`) |
| `production_lines` | [42, 104, 114, 115, 119] | OK (non-empty) |
| `extern_path` | verification/verus/extern_vb_vzcuf_PS_008.rs | OK (file exists; binds to production_inner mirror) |
| `mirror_path` | verification/verus/production_inner/vb_vzcuf_PS_008_production.rs | OK (file exists; drift-gated) |
| `drift_gate_script` | scripts/check-production-inner-drift.sh | OK (file exists; zero-tolerance drift policy) |
| `drift_threshold` | zero | OK |
| `assume_specification_targets` | ["production::SpecJournalWriteBatch::append_event"] | OK (non-empty; target bound to exec mirror) |
| `exec_wrapper` | wrapper_append_event_index_key_error | OK (matches PS-008/PS-009 exec wrapper pattern) |
| `exec_wrapper_required` | true | OK |
| `secondary_mirror` | verification/verus/production_inner/vb_vzcuf_PS_009_production.rs | OK (file exists; PS-009 mirror for symmetry) |
| `ALLOWED_EXCEPTIONS` escape hatch | NOT USED | OK (no backdoor mechanism) |

PO-09aaz-001 is NOT a vacuum proof: it is bound to production via WEAK_EXTERN with concrete file/line citations, a non-empty `assume_specification_targets` array, and a named exec wrapper.

## Repair Path (Resolved)

The previous rejection was mechanical. Both blocker categories are now resolved:

1. **Schema drift** (B1 in repair guide): proof-obligations.planned.jsonl now conforms to proof-obligation/v1 with all 8 missing fields added, 2 alias renames applied, and PO-09aaz-005 production_lines tightened.
2. **Ledger gap** (B2 in repair guide): state-3 (rust-contract) and state-4 (proof-planner) rows appended with valid hash chain. Transcript-state3.txt and transcript-state4.txt created. planner_invocation_id in all 16 lane reviews is now the real ledger value (proof-planner-vb-09aaz-state4).

After this approval, the plan can advance to State 5 (proof-writer). The proof-writer must:

1. Regenerate the PS-008/PS-009 production mirrors (verification/verus/production_inner/vb_vzcuf_PS_008_production.rs:78-95 and _PS_009_production.rs:67-93) to enumerate G8.
2. Add the `index_key_ok: bool` exec arg to SpecJournalWriteBatch::append_event in both spec files (vb-vzcuf-PS-008.rs and vb-vzcuf-PS-009.rs).
3. Add a new match arm for `Err(KeyCapacity)` from G8 in `assume_specification` requiring `spec_state_preserved_except_aborted(*old(batch), *final(batch))` with witness `!index_key_ok`.
4. Add a new exec wrapper `wrapper_append_event_index_key_error` to exercise G8 from `verus!` context.
5. Update the doc-comment at append_event.rs:18-26 (Guard Precedence) and L33-41 (Postconditions) per contract.md#C9.
6. Write the regression test in `crates/vb_storage/src/batch/t_append_event.rs` (mirrors t_putters_b.rs:177-209).
7. Write the proptest variant (or extend proptest_vb_vzcuf_PS_004.rs).
8. Write the master §49 integration test using a real Fjall instance.
9. Run `bash scripts/verify-verus.sh && bash scripts/check-verus-production-binding.sh && bash scripts/check-production-inner-drift.sh` to confirm all three Verus gates pass.

## Approval Summary

| Count | Disposition |
| --- | --- |
| Blockers (fixed_with_evidence) | 4 (F-09aaz-01, F-09aaz-02, F-09aaz-03, F-09aaz-05) |
| Major (owner_approved_debt, now resolved) | 1 (F-09aaz-04, transcript backfill complete) |
| Minor (fixed_with_evidence) | 0 (F-09aaz-05 reclassified as fixed) |
| Lane reviews accepted | 16/16 |

All 5 prior findings resolved. All 16 lane reviews accepted. No remaining blockers. No behavior-affecting waivers. Hash chain validated. Production binding gate satisfied.

STATUS: APPROVED

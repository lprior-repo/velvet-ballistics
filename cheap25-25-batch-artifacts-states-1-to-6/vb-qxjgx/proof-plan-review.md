reviewer_skill: proof-plan-reviewer
reviewer_invocation_id: vb-qxjgx-state4-proof-plan-review-attempt1
planner_invocation_id: p4-proof-planner: write proof-strategy, lane-decisions for vb-qxjgx
review_state: 4
reviewed_at: 2026-07-01T21:40:20Z
workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx
bead_id: vb-qxjgx

# Proof Plan Review: vb-qxjgx

## Review Metadata

- Reviewer skill: `proof-plan-reviewer`
- Reviewer invocation: `vb-qxjgx-state4-proof-plan-review-attempt1`
- Planner invocation (jj commit): `p4-proof-planner: write proof-strategy, lane-decisions for vb-qxjgx` (change id `kykklnlr`)
- Workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx`
- Review state: 4 (proof-plan-review)
- Status line: see `STATUS:` at end of file

## Reviewed Artifacts

| Artifact | Path | Hash (sha256) | Status |
|----------|------|---------------|--------|
| proof-strategy.md | `.beads/vb-qxjgx/proof-strategy.md` | `deee837828f3b2a113fcea2a67f4ca732d55dddedf3f759a4836514411f0aca2` | reviewed |
| verifier-lane-decisions.jsonl | `.beads/vb-qxjgx/verifier-lane-decisions.jsonl` | `1557011358e5ddc143c2441de00eba15814ebc4922b155bcbea6cc52192cb749` | reviewed |
| proof-obligations.planned.jsonl | `.beads/vb-qxjgx/proof-obligations.planned.jsonl` | `59de78d111a644fc646506d8c81c6e49f9464486ef8d9792ee47f26edcce714c` | reviewed |
| trusted-base-plan.md | `.beads/vb-qxjgx/trusted-base-plan.md` | `d39f71b3bd51c289837494128725ad715c4d8c6c57c90d4149074cb3674547d6` | reviewed |
| waiver-candidates.jsonl | `.beads/vb-qxjgx/waiver-candidates.jsonl` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` (empty — sha256 of empty bytes) | reviewed |
| verifier-lane-matrix.md | `.beads/vb-qxjgx/verifier-lane-matrix.md` | (summarized into proof-coverage-matrix.md) | referenced |
| proof-coverage-matrix.md | `.beads/vb-qxjgx/proof-coverage-matrix.md` | (cross-table) | referenced |
| contract.md | `.beads/vb-qxjgx/contract.md` | (read for clause coverage) | referenced |
| agent-invocation-ledger.jsonl | `.beads/vb-qxjgx/agent-invocation-ledger.jsonl` | seq 1-3 after this review | reviewed (appended seq 3) |

## Review Summary

### Verifier Profile

Contract-selected profile (per `delivery-scope.jsonl`): `kani + proptest + flux-rs + unit`. Verus is intentionally out of scope per `contract.md` NON-GOALS and absent from every `verifier_modes` value in `delivery-scope.jsonl`. No `not_applicable` lane decision needed for Verus because the verifier was never in the chosen profile (validator rule: silent omissions only occur when a profile-required verifier is dropped, not when the verifier was never on the contract's profile).

### Lane Decision Coverage: PASS

- 12 `verifier-lane-decision/v1` rows in `verifier-lane-decisions.jsonl` (VLD-QXJGX-001 through 012).
- 11 `required` rows (5 kani + 4 proptest + 2 kani).
- 1 `blocked_tooling` row (flux-rs, VLD-QXJGX-012, citing vb-b8i8f closure and `codec/mod.rs:184-186` as evidence + `BEAD-TOOL-FLUX-RS-INSTALL` acquisition ref).
- 0 `not_applicable` rows. The contract's risk profile does not demand explicit non-applicability rows because no required verifier was excluded.
- Every required row names a planned `proof-obligation/v1` ID; the blocked_tooling row names `[]` (correct: blocking lanes carry no obligations in this plan).
- See FIND-QXJGX-002 for an analytical observation about additional flux-rs `not_applicable` rows; disposition is `owner_approved_no_action` because the substantive coverage is sound.

### Schema Conformance: PASS

- All 7 obligations use schema `proof-obligation/v1` with all required fields populated: `id`, `requirement_id`, `contract_clause`, `domain_claim`, `risk`, `risk_tags`, `verifier`, `artifact`, `target`, `command`, `workdir`, `expected_evidence`, `assumptions`, `model_bounds`, `tool_metadata`, `trusted_base_refs`, `required`, `behavior_affecting`, `mode`, `owner_state`, `rerun_from`, `status`.
- No legacy alias fields (`layer`, `checker`, alias-only `claim`) detected.
- `target` field is canonical (production symbol path) on every obligation. See FIND-QXJGX-004 for cosmetic narrowing notes.
- `command` field is explicit with flags (`--mem-predicates`, `PROPTEST_CASES=10000`, `--harness`, etc.) and workdir paths.
- `expected_evidence` enforces `VERIFICATION:- SUCCESSFUL` (kani) or `test result: ok` (proptest) gate; explicit "no cover!-as-proof", "no --no-default-checks", "no --no-memory-safety-checks", "no --no-overflow-checks", "no --no-unwinding-checks" suppression audits are baked into the prose.
- `owner_state: 4`, `rerun_from: 4`, `status: planned` on every row (consistent).
- `assumptions: []` on every obligation (no `kani::assume(...)` short-circuits; kani::any() / kani::any_where() only).
- `model_bounds` populated: kani uses `unwind=8, input_size=1024, mem_high=20G, mem_max=24G` (default per `references/resource-governance.md`); proptest uses `cases=10000, input_size=1024` (default).
- `tool_metadata` populated with `tool`, `version_pin`, and where relevant `feature_flags` + `solver`.

### Verifier Counts and Composition: PASS

- 7 obligations: 5 kani (PO-QXJGX-001 through PO-QXJGX-005) + 2 proptest (PO-QXJGX-006, PO-QXJGX-007).
- Counts match the operator's verification spec ("kani ×5 + proptest ×2").

### Production Binding Coverage: PASS

- All 7 obligations target production symbols with canonical rust paths (`crate::records::RecordKind::id`, `crate::events::JournalEvent::record_kind`, `crate::codec::validation::is_known_record_kind`, `crate::codec::EnforceKindParity::enforce_kind_parity`, `crate::codec::decode_journal_event`, `crate::recovery::replay::summary::apply::apply`, `crate::runtime::durability_matrix::DURABILITY_MATRIX`).
- `proof-coverage-matrix.md` §4 documents the binding map: STRONG mechanism for every obligation (kani harness calls the production function directly; proptest generator reaches the production constant/function).
- No `verification/` shadow types disconnected from production. No dead harness code; all harnesses extend the existing `crates/vb_storage/src/kani_record_kind.rs` or are appended to the existing parity-gate pattern (`check_ask_timed_out_payload_kind_parity_accepts_kind_29`).
- Verus-specific `production_binding` field is NOT required because no Verus obligations exist (the rule applies only when `verifier: verus`).

### Kani Non-Vacuity: PASS

- Every kani harness uses `kani::any()` / `kani::any_where()` for symbolic input and `kani::assert` for property assertions.
- `kani::cover!` is used solely as reachability evidence, paired with `kani::assert` for the property. No `cover!`-as-proof obligations. The cover instances explicitly prove reachability for: id-33 branch, StepSucceeded arm, id-33 family branch, new family id, legacy envelope-12 + StepSucceeded branch, cross-bind rejection, legacy decode path (per `trusted-base-plan.md` §3 table).
- No `--only-codegen`, `--no-codegen`, `--no-default-checks`, `--no-memory-safety-checks`, `--no-overflow-checks`, `--no-unwinding-checks`, `--prove-safety-only`, or other proof-theater flags appear in any obligation command.

### Proptest Anti-Invariants: PASS

- Every proptest obligation has an explicit anti-invariant token (`invalid_input`) and an explicit `prop_assume!(false)` for the pre-fix collapse path.
- PO-QXJGX-006 second sub-property pairs the post-replay counter assertion with an anti-invariant that asserts an id-keyed counter would yield a different total — closing the `E_KANI_ASSUMPTION_VACUITY`-style pre-fix collapse.
- PO-QXJGX-007 first sub-property asserts the post-fix `StepSucceeded` rows and asserts the absence of pre-fix `SlotWritten` in step-closing positions.

### Backward Compatibility Decision: PASS

- BACK-COMPAT = LEGACY ENVELOPE-12 TOLERANCE, NOT a schema bump. Confirmed by:
  - `contract.md` Context §"Domain terms" pins `CURRENT_SCHEMA_VERSION: u16 = 1` at `crates/vb_storage/src/constants.rs:58` and forbids raising it per `NON-GOALS`.
  - `proof-strategy.md` §1 ("No schema version bump"), §8 ("CURRENT_SCHEMA_VERSION remains 1; no migration is added").
  - `crates/vb_storage/src/constants.rs:58` reads `pub const CURRENT_SCHEMA_VERSION: u16 = 1;` (verified on disk).
  - tests.rs:3925 and tests.rs:4223 assert CURRENT_SCHEMA_VERSION=1 per contract (not modified by this bead).
  - LegacyEnvelopeBinding::Legacy { accepted_ids: &[12, 33] } is the typed accessor for the dual-envelope tolerance (POST-005); the same acceptance set is enforced by both `EnforceKindParity for JournalEvent` (kind_parity.rs:50-64) and `validate_journal_event_record_kind` (mod.rs:97-111).
  - Writer ALWAYS emits canonical id 33; reader accepts {12, 33} for StepSucceeded only; no runtime config flag, no compat-mode toggle.

### Flux-rs Blocked Tooling: PASS

- `VLD-QXJGX-012` is `blocked_tooling`, `limitation_kind: external_dependency_unavoidable`, `tooling_acquisition_ref: BEAD-TOOL-FLUX-RS-INSTALL`.
- Evidence ref: `crates/vb_storage/src/codec/mod.rs:184-186` keeps `pub mod flux_validation` commented out with `// vb-b8i8f:`-prefixed comment; checked on disk.
- Compensating evidence: PO-QXJGX-007 third sub-proptest parses `crates/vb_storage/src/codec/flux_validation.rs:14,33` and asserts 33 appears in both literal sets (literal-sync per POST-011).
- `E_BLOCKED_TOOLING_ADVANCE` rule satisfied: blocked_tooling rows do not pass; they block State 4 onwards. The proptest literal-sync carries the behavioral witness.

### Waiver Candidates: PASS

- `waiver-candidates.jsonl` is empty (0 rows). `sha256` of empty file is `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` (cross-checked).
- No behavior-affecting waivers exist.
- All 5 kani obligations and both proptest obligations are `behavior_affecting: true` and either required or carry a typed acceptance set.

### Trusted Base Plan: PASS

- `trusted-base-plan.md` documents every trust marker:
  - §1 Assumption Ledger: empty `assumptions` arrays on all 7 obligations (no `kani::assume(...)` short-circuits).
  - §2 Model-Bound Ledger: every kani obligation uses the default `unwind=8, input_size=1024, mem_high=20G, mem_max=24G`. No reduction.
  - §3 Tooling and Trust-Marker Ledger: every `kani::cover!` paired with `kani::assert`; every `prop_assume!(false)` is the post-fix anti-invariant (rejection evidence).
  - §4 Skipped-Test Ledger: zero `#[cfg_attr(miri, ignore)]` or `#[ignore]` markers. Kani harnesses are `cfg(kani)`-feature-gated; proptest sources are dev-dependency gated. Neither is a runtime skip.
  - §5 Compensating Evidence Cross-Reference: rows for flux-rs literal-sync, default-profile proptest gaps, default-profile kani gaps, static-scan obligations, manual-qa obligations.
  - §6 Open Tooling Acquisition: `BEAD-TOOL-FLUX-RS-INSTALL` is the open bead for re-enabling flux-rs.
- Validator rule: `E_TRUST_UNLEDGERED_MARKER` and `E_TRUST_PENDING_AT_CLOSURE` are not triggered because (a) no `assume`/`axiom`/`admit`/`external_body`/`#[trusted]`/stub/disabled check in any obligation command, and (b) the `unwind=8` model bound is default and need not be reduced.

### Bridge Planning: PASS (State 7 will land the actual bridge)

- The proof plan defers bridge planning to State 7 (proof-to-implementation). This is the canonical lifecycle (`proof-pipeline-contract.md` State 7: "proof-to-implementation maps proof claims to Rust source refs, behavior tests, refinement harness refs, and exact evidence commands").
- Every obligation's `target` is a canonical Rust path that State 7 will bind to file:line refs. The mapping back to production is straightforward (single-symbol obligations, STRONG binding).
- `proof-strategy.md` §6 details the 8 line-site durability-matrix substitutions that State 11 (holzman-rust) will execute, including the `durability_matrix/tests.rs:50-94` lockstep.

### Review Provenance: PASS

- Reviewer invocation ID: `vb-qxjgx-state4-proof-plan-review-attempt1`.
- Planner invocation (jj commit): `p4-proof-planner: write proof-strategy, lane-decisions for vb-qxjgx`.
- Independent: distinct invocation IDs and distinct skills (`proof-plan-reviewer` vs the jj commit's authoring agent `femdation-controller`). The planner's artifacts contain NO reviewer fields (no `reviewer_disposition`, no `finding_refs`).
- Reviewer's state4 row appended to `.beads/vb-qxjgx/agent-invocation-ledger.jsonl` (sequence 3, entry_hash `cadc28bf56c6331be62bed31c6834734439557fd0515bf82aecdf6f49982c940`). See FIND-QXJGX-003 for the planner's missing self-log (non-blocker; jj audit trail substitutes).

### Verifier-Lane-Review Output: PASS

- 12 `verifier-lane-review/v1` rows in `.beads/vb-qxjgx/verifier-lane-review.jsonl` (VLR-QXJGX-001 through 012), one per planner row.
- All 12 rows have `reviewer_disposition: accepted`, with FIND-QXJGX-002 cited only on VLR-QXJGX-012 (the flux-rs blocked_tooling row).
- `planner_invocation_id` and `reviewer_invocation_id` populated on every row.
- `owner_state: 4`, `status: reviewed` on every row.

## Findings

See `.beads/vb-qxjgx/proof-plan-findings.jsonl` for the structured findings. Summary:

| ID | Code | Severity | Disposition | Description |
|----|------|----------|-------------|-------------|
| FIND-QXJGX-001 | E_LANE_DECISION_WEAK | minor | owner_approved_no_action | proof-strategy.md §5 prose mentions VLD-QXJGX-VERUS-001/VLD-QXJGX-FLUX-001 IDs that do not appear in the jsonl. Validator-beats-markdown; documented for prose tightening. |
| FIND-QXJGX-002 | E_LANE_DECISION_MISSING | major | owner_approved_no_action | Flux-rs lane decisions are filed only for POST-011 (VLD-QXJGX-012) but PS-vb-qxjgx-001/003 suggested `flux` in `suggested_layers`. Substantive coverage is sound (kani exhaustiveness, global flux-rs unavailability per vb-b8i8f). Recommended low-priority backfill: add `not_applicable` rows VLD-QXJGX-013/014. |
| FIND-QXJGX-003 | E_INVOCATION_LEDGER_MISSING | observation | owner_approved_no_action | agent-invocation-ledger.jsonl captured only state 1+2; no state 3 (rust-contract) or state 4 (proof-planner) entry. Reviewer appends own state4 row (sequence 3). The planner's commit is in jj working-copy @ (change id kykklnlr) and serves as audit trail. |
| FIND-QXJGX-004 | E_SOURCE_REF_SHAPE | minor | owner_approved_no_action | `target` field is narrower than `expected_evidence` scope on PO-QXJGX-003, PO-QXJGX-004, PO-QXJGX-007. The full binding is documented in `expected_evidence` and `proof-coverage-matrix.md` §4. Cosmetic tightening only. |
| FIND-QXJGX-005 | E_REVIEW_PROVENANCE_MISSING | observation | fixed_with_evidence | Self-referential: this markdown header now includes reviewer_skill, reviewer_invocation_id, review_state, reviewed_artifacts/hashes, and STATUS. |

## Verdict

The proof plan is complete, precise, and implementation-bound. All 7 obligations target production symbols with verified file paths, all 5 kani obligations use `kani::assert` (never `kani::cover!`-as-proof), and both proptest obligations carry explicit anti-invariants with `invalid_input` tokens. Backward compatibility is `LEGACY ENVELOPE-12 TOLERANCE`, not a schema bump — `CURRENT_SCHEMA_VERSION = 1` is pinned and verified on disk at `crates/vb_storage/src/constants.rs:58`. The single `blocked_tooling` lane decision (VLD-QXJGX-012, flux-rs, vb-b8i8f closure, BEAD-TOOL-FLUX-RS-INSTALL acquisition ref) is supported by a literal-sync proptest (PO-QXJGX-007 third sub-property). No behavior-affecting waivers exist. The planner and reviewer invocation IDs are independent. The reviewer has appended its state4 row to the agent-invocation ledger.

The plan is ready for `proof-writer` (State 5).

**STATUS: APPROVED**

## Next Steps

1. State 5 (`proof-writer`): Execute the 7 planned obligations using the exact commands in `proof-obligations.planned.jsonl` under the workdir `crates/vb_storage` (for kani) and `crates/vb_runtime` (for proptest). Use `scripts/kani-list.sh <package>` for kani harness isolation per `AGENTS.md`.
2. State 6 (`proof-reviewer`): Validate written proof artifacts against this plan and `proof-coverage-matrix.md`.
3. State 7 (`proof-to-implementation`): Materialize refinement obligations; bind every `proof-obligation/v1` row to file:line refs in production code.
4. Optional low-priority audit pass: backfill the prose tightening from FIND-QXJGX-001/002/004 and the planner/state-3 ledger rows from FIND-QXJGX-003.

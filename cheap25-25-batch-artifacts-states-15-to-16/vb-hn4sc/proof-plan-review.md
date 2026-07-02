# Proof Plan Review — vb-hn4sc

## Provenance

- bead_id: vb-hn4sc
- review_state: approved
- reviewer_skill: proof-plan-reviewer
- reviewer_invocation_id: proof-plan-reviewer-vb-hn4sc-state4b-attempt1
- planner_invocation_id: proof-planner-vb-hn4sc-state4-attempt1
- host_session_id: femdation-cheap25-batch
- workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-hn4sc
- reviewed_at: 2026-07-01T16:30:00Z
- contract_phase: 3 (rust-contract) — accepted
- plan_phase: 4 (proof-planner) — accepted
- review_phase: 4b (proof-plan-reviewer) — accepted
- next_state: 5 (proof-writer)

## Inputs Reviewed

| Artifact | Path | SHA-256 |
|---|---|---|
| proof-strategy.md | .beads/vb-hn4sc/proof-strategy.md | `354362a25c7c44668953f3f814ce526ef44a25aa64ba8686815d8bea43884699` |
| verifier-lane-decisions.jsonl | .beads/vb-hn4sc/verifier-lane-decisions.jsonl | `973abe4c9ef2238f81f506240d7eaac860e4cb3e8032a4df915d1a737d8b010a` |
| proof-obligations.planned.jsonl | .beads/vb-hn4sc/proof-obligations.planned.jsonl | `6cf1e487f534e63b4e2c62679e3f2f2492752ee181fa87e850f9b77edff9f64c` |
| trusted-base-plan.md | .beads/vb-hn4sc/trusted-base-plan.md | `900e4fca3f18b21c679c777ec4bb6ccb0fd2694e2394fc9de5bd217030be9370` |
| waiver-candidates.jsonl | .beads/vb-hn4sc/waiver-candidates.jsonl | `a5d1bbd06d894ea84e105674ce07033fe957f282d740b53bc8523ddf24977f53` |
| contract.md | .beads/vb-hn4sc/contract.md | `65898b70e7ede6fb71c1e1f5a041623c490d1b5f5fe14f2ed4f7a0e90c4c08f2` |
| domain-model.md | .beads/vb-hn4sc/domain-model.md | `dbba7dda272117f98f22b6335cdd895fa43e6b18c080c55ace61e536968cc908` |
| type-contracts.md | .beads/vb-hn4sc/type-contracts.md | `019616b6c497175ed5eb61c103e33779980902a308b721f6df93218709b89edc` |
| boundary-map.md | .beads/vb-hn4sc/boundary-map.md | `83680a5fb750e018841292303d415cd10837a4791df6c30662772b256a7bbbae` |
| error-taxonomy.md | .beads/vb-hn4sc/error-taxonomy.md | `0fe48b7721d92b20c1cacd24ff22fcbe927fd983f559073a72fa8c86bd50a76a` |
| hazard-analysis.md | .beads/vb-hn4sc/hazard-analysis.md | `abac986ef44f6cbadbe44cf1fb93d81c2293b6296a307c2c7e761d59b0ce6311` |
| workflow-model.md | .beads/vb-hn4sc/workflow-model.md | `b5f11d6787823d7885b8722c846fbb81deadacdbc629be78902999cac1b3b259` |
| proof-seeds.jsonl | .beads/vb-hn4sc/proof-seeds.jsonl | `f6559aa9c085a0b895c45adda13073adb1f0bbf1982a9c3e3bd5f51d70a2fe7d` |
| traceability-matrix.jsonl | .beads/vb-hn4sc/traceability-matrix.jsonl | `fbe3dec1c36cb26f1b6eb14014c95d6ede0e3ee13668ed51254d4ad8cc88ad53` |
| proof-coverage-matrix.md | .beads/vb-hn4sc/proof-coverage-matrix.md | `ee27a686693e889e4b52d5024932a29469ef609334eef452b65dca4a3f45c89a` |
| delivery-scope.jsonl | .beads/vb-hn4sc/delivery-scope.jsonl | (state 2 artifact, referenced) |
| codebase-map.md | .beads/vb-hn4sc/codebase-map.md | (state 2 artifact, referenced) |

## Verifier Lane Decisions (20 total, all accepted)

20 of 20 lane decisions in `verifier-lane-decisions.jsonl` received `verifier-lane-review/v1` rows in
`verifier-lane-review.jsonl` with `reviewer_disposition: accepted`:

| # | Lane | Verifier | Applicability | Disposition | VLR ID |
|---|---|---|---|---|---|
| 1 | LD-vb-hn4sc-001 | rust-local | required | accepted | VLR-vb-hn4sc-001 |
| 2 | LD-vb-hn4sc-002 | rust-local | required | accepted | VLR-vb-hn4sc-002 |
| 3 | LD-vb-hn4sc-003 | rust-local | required | accepted | VLR-vb-hn4sc-003 |
| 4 | LD-vb-hn4sc-004 | rust-local | required | accepted | VLR-vb-hn4sc-004 |
| 5 | LD-vb-hn4sc-005 | rust-local | required | accepted | VLR-vb-hn4sc-005 |
| 6 | LD-vb-hn4sc-006 | rust-local | required | accepted | VLR-vb-hn4sc-006 |
| 7 | LD-vb-hn4sc-007 | rust-local | required | accepted | VLR-vb-hn4sc-007 |
| 8 | LD-vb-hn4sc-008 | persistence | required | accepted | VLR-vb-hn4sc-008 |
| 9 | LD-vb-hn4sc-009 | persistence | required | accepted | VLR-vb-hn4sc-009 |
| 10 | LD-vb-hn4sc-010 | persistence | required | accepted | VLR-vb-hn4sc-010 |
| 11 | LD-vb-hn4sc-011 | kani | required | accepted | VLR-vb-hn4sc-011 |
| 12 | LD-vb-hn4sc-012 | kani | required | accepted | VLR-vb-hn4sc-012 |
| 13 | LD-vb-hn4sc-013 | kani | required | accepted | VLR-vb-hn4sc-013 |
| 14 | LD-vb-hn4sc-014 | proptest | required | accepted | VLR-vb-hn4sc-014 |
| 15 | LD-vb-hn4sc-015 | verus | not_applicable | accepted | VLR-vb-hn4sc-015 |
| 16 | LD-vb-hn4sc-016 | flux-rs | not_applicable | accepted | VLR-vb-hn4sc-016 |
| 17 | LD-vb-hn4sc-017 | loom | not_applicable | accepted | VLR-vb-hn4sc-017 |
| 18 | LD-vb-hn4sc-018 | miri | not_applicable | accepted | VLR-vb-hn4sc-018 |
| 19 | LD-vb-hn4sc-019 | cargo-fuzz | not_applicable | accepted | VLR-vb-hn4sc-019 |
| 20 | LD-vb-hn4sc-020 | tla-plus | not_applicable | accepted | VLR-vb-hn4sc-020 |

### `not_applicable` Validation (proof-theater rejection criteria)

All 6 `not_applicable` decisions include concrete `non_applicability_evidence_refs` (3-4 each):

- **verus**: codebase-map.md §116 + contract.md §2 T-HN4SC-2 + vb-vzcuf/contract.md (queued path explicitly out of Verus scope).
- **flux-rs**: codebase-map.md §116 + vb-hn4sc newtypes over u64 + vb-vzcuf PS-005/PS-008 covers direct-path Flux.
- **loom**: contract.md §9 OI-3 + type-contracts.md §2.2 + codebase-map.md §144 (Loom model for byte accumulator is OUT OF SCOPE per Holzman Rust scope discipline).
- **miri**: error/mod.rs:1, batch/types.rs:1, lib.rs:1 + type-contracts.md §180 + Holzman Rust engineering rules.
- **cargo-fuzz**: codebase-map.md §84 + vb-vzcuf PS-005 fuzz target + vb-hn4sc no new parser/codec boundary.
- **tla-plus**: proof-planner skill SKILL.md + contract.md §3 W-HN4SC-1..9 + codebase-map.md §144.

None of the `not_applicable` decisions use the "too hard", "not practical", or "not needed" prose. All
have 3+ evidence refs, all pointing to existing on-disk artifacts or canonical skill text.

## Proof Obligations (6 total, all schema-valid)

| Obligation | Verifier | Risk | Status |
|---|---|---|---|
| POB-vb-hn4sc-001 | kani | HIGH | planned |
| POB-vb-hn4sc-002 | proptest | MEDIUM | planned |
| POB-vb-hn4sc-003 | rust-local | HIGH | planned |
| POB-vb-hn4sc-004 | rust-local | HIGH | planned |
| POB-vb-hn4sc-005 | persistence | HIGH | planned |
| POB-vb-hn4sc-006 | rust-local | MEDIUM | planned |

### Schema Validation

All 6 obligations verified against `proof-obligation/v1` schema requirements:

- `schema_version: "proof-obligation/v1"` — 6/6 ✓
- `id`, `requirement_id`, `contract_clause`, `domain_claim`, `verifier`, `artifact`, `target`, `command`, `workdir`, `expected_evidence`, `assumptions`, `model_bounds`, `tool_metadata`, `trusted_base_refs`, `required`, `behavior_affecting`, `mode`, `owner_state`, `rerun_from`, `status` — 6/6 ✓
- No legacy alias fields (`layer`, `checker`): 6/6 absent ✓
- `target` present and non-empty: 6/6 ✓
- `command` present and non-empty: 6/6 ✓
- `workdir` present and non-empty: 6/6 ✓
- `expected_evidence` present and non-empty: 6/6 ✓
- `assumptions` array non-empty: 6/6 (2-3 entries each) ✓
- `model_bounds` present: 6/6 ✓
- `trusted_base_refs` non-empty: 6/6 (TBP-001 alone or TBP-001+TBP-002) ✓
- `required: true`, `behavior_affecting: true`: 6/6 ✓
- `owner_state: 4`, `rerun_from: 5`, `status: planned`: 6/6 ✓
- `mapping_status: planned`: 6/6 (correct at State 4; required to be `materialized` at State 12) ✓

### Production Binding (Mandatory)

**No Verus obligations in this plan** — `production_binding` requirement is vacuously satisfied.
The plan uses `verifier: rust-local` (not `verifier: verus`) for the compile-time const assertion
and parity tests. The Kani obligation POB-vb-hn4sc-001 binds to the new
`crates/vb_storage/src/kani_vb_vzcuf_ps010.rs` (NEW harness file) and the existing
`crates/vb_storage/src/queue/writer.rs` (gate call site at pre-commit boundary). The persistence
obligation POB-vb-hn4sc-005 binds to writer.rs:152-231 (flush_batch body) and writer/stage.rs
(stage_queued_event). All bindings are concrete and exist on disk.

### Verus Production-Binding Plan Validation

**Not applicable to this plan** — no `verifier: verus` obligations exist, so the
STRONG/WEAK_MIRROR/WEAK_EXTERN validation does not apply. The plan's `verus` lane decision
is `not_applicable` (LD-vb-hn4sc-015-verus) with 3 evidence refs.

### Trusted Base Validation

The plan enumerates 4 trusted base groups (TBP-001..TBP-004) with 12 facts, all existing:

- TBP-001: `u64::checked_add` (TBP-001.1), 7 production constants (TBP-001.2), `encode_record` determinism (TBP-001.3).
- TBP-002: `JournalWriteBatch::append_event` behavior (TBP-002.1), diagnostic code 0x4022 / `JOURNAL_BATCH_BYTES_EXCEEDED` (TBP-002.2).
- TBP-003: `Mutex<JournalWriterQueueState>` (TBP-003.1).
- TBP-004: `OwnedWriteBatch::commit()` atomicity (TBP-004.1).

All 6 obligations correctly reference TBP-001 (and most also TBP-002). No new trusted base entries
are introduced. The plan says "0 new trusted base entries introduced by this bead" and this is correct.

### Non-Vacuity Plan

- Kani obligation POB-vb-hn4sc-001 uses `kani::any()` for accumulator/next/limit with explicit
  `kani::assume(...)` bounds (GOD RULE 1 satisfied). The harness exercises 5 distinct invariant
  classes: overflow sentinel, exact-fit boundary, over-limit rejection, default-budget accommodation,
  type-system newtypes.
- proptest obligation POB-vb-hn4sc-002 generates 256 cases with a bounded payload byte range,
  filtered to MAX_ENCODED_RECORD_BYTES via `proptest::strategy::AssumeOkBound`.
- The rust-local and persistence obligations assert concrete state changes (no commit called on
  rejection, pending.len() unchanged, drain_all short-circuit) — not model-equivalence smoke.

### Behavior Waivers

Zero behavior-affecting waivers. The two waiver-candidate rows are:

- W-vb-hn4sc-NONE-001: confirms NO behavior-affecting waivers planned; review_status: approved.
- W-vb-hn4sc-OI-001: deferred to proof-to-implementation (RuntimeError classification, contract.md §9 OI-1); behavior_affecting: false; review_status: deferred.

The deferred item does not weaken any of the 6 obligations because none assert anything about
`RuntimeError` shape — they all assert only the typed `JournalError` variant, fields, and
diagnostic codes (TBP-002.1 covers this).

## Hard-Constraint Compliance (from proof-strategy.md §2)

- ✓ **No new `JournalError` variant.** Confirmed at `error/mod.rs:40-41`: variant is reused
  as `JournalBatchBytesExceeded { attempted: u64, limit: u64 }`. Diagnostic code 0x4022 at
  `error/codes.rs:75`. Display string verified at `error/mod.rs:40-41`.
- ✓ **No widening of `RuntimeError`.** Carried to proof-to-implementation (OI-1, H-12).
- ✓ **Compile-time const assertion required.** `proof-strategy.md §7` specifies the exact
  const block with 3 assertions binding `StorageLimits::DEFAULT.max_journal_batch_bytes` to
  the encoded-basis constant (1_048_636) and the latter to the payload limit + header bytes.
  The pattern mirrors `IndexStatusState::_INDEX_STATUS_STATE_EXHAUSTIVE` at `types.rs:324-334`.
- ✓ **No `unsafe`, no `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg!`.** Touched files
  have `#![forbid(unsafe_code)]` at module level (verified at `error/mod.rs:1`, `batch/types.rs:1`,
  `lib.rs:1`). Type-contracts.md §180 enforces the no-`unsafe` invariant on the new types.
- ✓ **Kani harness uses `kani::any()`.** `proof-strategy.md §6` mandates `kani::any()` for
  accumulator, next, and limit with explicit `kani::assume(...)` bounds — no hardcoded shapes
  (GOD RULE 1 satisfied).

## Bridge Plan (State 7 → proof-to-implementation)

The plan is precise enough for the proof-to-implementation agent to produce
`rust-refinement-obligation/v1` rows for all 6 obligations with:

- **POB-vb-hn4sc-001 (kani)**: source_refs = `crates/vb_storage/src/kani_vb_vzcuf_ps010.rs`
  (NEW), `crates/vb_storage/src/queue/writer.rs` (gate call site), `crates/vb_storage/src/batch/append_event.rs:86-102`
  (parity target). Refinement_harness_refs = the new kani_vb_vzcuf_ps010 harness.
- **POB-vb-hn4sc-002 (proptest)**: source_refs = `crates/vb_storage/src/codec.rs::encode_record`,
  `crates/vb_storage/src/queue/writer.rs` (gate consumes `value.len()`). Behavior_test_refs =
  the `length_roundtrip` proptest block at `crates/vb_storage/src/queue/tests.rs`.
- **POB-vb-hn4sc-003 (rust-local const)**: source_refs = `crates/vb_storage/src/types.rs`
  (StorageLimits + const block), `crates/vb_storage/src/storage_constants.rs` (NEW alias).
- **POB-vb-hn4sc-004 (rust-local parity)**: source_refs = `crates/vb_storage/src/queue/tests.rs`
  (parity test), `crates/vb_storage/src/batch/append_event.rs:86-102`, `crates/vb_storage/src/error/{mod,codes}.rs`.
- **POB-vb-hn4sc-005 (persistence)**: source_refs = `crates/vb_storage/src/queue/writer.rs:152-231`
  (flush_batch body), `crates/vb_storage/src/queue/writer/stage.rs` (stage_queued_event). Refinement
  harness is the real Fjall integration at `queue/tests.rs:19` (temp_journal).
- **POB-vb-hn4sc-006 (rust-local negative-space)**: source_refs = `crates/vb_storage/src/queue/writer.rs:67-91`
  (enqueue path), `crates/vb_storage/src/queue/tests.rs` (enqueue_does_not_enforce_byte_budget_only_flush_does).

## Findings (low severity only — all `owner_approved_no_action`)

| Finding Code | Severity | Disposition | Owner |
|---|---|---|---|
| E_SCOPE_DOC_DRIFT | low | owner_approved_no_action | proof-planner |
| E_NEW_MODULE_PLAN | low | owner_approved_no_action | proof-to-implementation |
| E_PROPTEST_VERSION_ASSUMPTION | low | owner_approved_no_action | formal-verifier |
| E_KANI_VERSION_HINT | low | owner_approved_no_action | formal-verifier |
| E_TRUSTED_BASE_LANES_NOT_LEDGERED | low | owner_approved_no_action | formal-verifier |
| E_BEHAVIOR_OBLIGATION_MAPPING | low | owner_approved_no_action | proof-to-implementation |

All 6 findings use the canonical `finding/v1.disposition: owner_approved_no_action` with
`owner`, `approval_ref`, and `rationale` populated. **Zero blocker findings.** No
`blocker_reason`, no `blocker` disposition, no `STATUS: REJECTED`.

## Plan Quality Assessment

### Strengths

1. **Lane profile coverage complete**: 14 required lanes (4 rust-local, 1 persistence,
   3 kani, 1 proptest) + 6 not_applicable lanes (verus, flux-rs, loom, miri, cargo-fuzz,
   tla-plus) = 20 total. Every demanded verifier has a decision with concrete evidence
   for `not_applicable`.

2. **No silent omissions**: Default-profile verifiers (Verus, Flux, Loom, Miri, cargo-fuzz,
   TLA+) are explicitly `not_applicable` with 3-4 evidence refs each. This satisfies the
   `proof-planner` skill rule "no silent omissions".

3. **Kani harness wiring specified**: `proof-strategy.md §6` names the exact file path
   (`crates/vb_storage/src/kani_vb_vzcuf_ps010.rs`), the harness ID
   (`kani_vb_vzcuf_ps010::check_queued_byte_budget_invariants`), the feature gate
   (`kani-vb-vzcuf`), the slot in `lib.rs:76-94` (next to ps001-ps009), and the run command.
   The `kani::any()` discipline is explicit (GOD RULE 1 satisfied).

4. **Compile-time const assertion planned**: `proof-strategy.md §7` provides the exact const
   block with 3 assertions, mirroring the existing `IndexStatusState::_INDEX_STATUS_STATE_EXHAUSTIVE`
   pattern (verified at `types.rs:324-334`). The numeric constants (1_048_636, 1_048_576, 60)
   are individually verified on disk.

5. **Parity claim is concrete**: The plan identifies the exact existing parity test
   (`accounting_uses_full_encoded_length_not_payload_length` at
   `crates/vb_storage/src/batch/t_byte_accounting_part1.rs:76`) and the exact existing
   `JournalWriteBatch::append_event` lines (86-102) that the queued path must match. Both
   verified on disk.

6. **Trust boundaries are minimal and explicit**: 4 TBP groups with 12 facts, all existing,
   all referenced by name from the obligations' `trusted_base_refs`. No new TBP entries
   introduced.

7. **No behavior waivers**: Both waiver-candidate rows are non-behavior-affecting (one is
   "NONE-001" confirming no waivers; the other is a deferred documentation item).

8. **Existing test coverage preserved**: The plan explicitly states that
   `flush_batch_rejects_same_batch_duplicate_key` (verified at `queue/tests.rs:1073`) and
   `flush_batch_across_calls_handles_idempotent_retry` continue to pass unmodified.

### Low-Severity Observations (already dispositioned above)

- Delivery-scope.jsonl clause GROUP-COMMIT-BYTE-GATE-7 still mentions 1_048_576; the resolved
  contract uses 1_048_636 per M-HN4SC-9. The plan applies the resolution and the const
  assertion enforces the binding at compile time, so the runtime is correct. Documentation
  drift only.
- `crates/vb_storage/src/storage_constants.rs` is a new module file referenced by the const
  assertion; the const block cannot compile without it, so the obligation implicitly requires
  the file. The proof-to-implementation bridge will record the new file.
- proptest 1.11.0 / kani "latest" are guidance hints; the formal-verifier at State 12 will
  pin the exact versions and report them in the verification-ledger.

## Compliance Checklist (reviewer perspective)

| Check | Result |
|---|---|
| All 20 lane decisions have verifier-lane-review/v1 rows | ✓ |
| Planner/reviewer invocation IDs are independent | ✓ |
| All 6 obligations have all required schema fields | ✓ |
| No legacy alias fields (`layer`, `checker`) | ✓ |
| All `not_applicable` decisions have 3+ evidence refs | ✓ |
| No behavior-affecting waivers | ✓ |
| Kani harness uses `kani::any()` (GOD RULE 1) | ✓ |
| Compile-time const assertion planned (T-HN4SC-7) | ✓ |
| JournalError::JournalBatchBytesExceeded reused (no new variant) | ✓ |
| Diagnostic code 0x4022 reused (no new code) | ✓ |
| Bridge plan is concrete enough for State 7 | ✓ |
| Trusted base plan enumerates 4 groups, 12 facts, 0 new entries | ✓ |
| All findings use canonical `finding/v1.disposition` values | ✓ |
| No `blocker` findings; no `STATUS: REJECTED` path | ✓ |

## Disposition

The proof plan for vb-hn4sc is **APPROVED**. The plan is precise enough for the
proof-writer (State 5) to produce the `kani_vb_vzcuf_ps010` harness and the
`length_roundtrip` proptest, and for the proof-to-implementation (State 7) to
produce `rust-refinement-obligation/v1` rows for all 6 obligations. The 6
low-severity findings are all `owner_approved_no_action` with non-blocking
dispositions; no `blocker` findings, no `STATUS: REJECTED`.

The bead advances from State 4b (proof-plan-reviewer) to State 5 (proof-writer).
The 6 obligations remain in `mapping_status: planned` and will be transitioned
to `materialized` at State 7 and `verified` at State 12.

## Cross-Reference

- **Approved lane review rows**: 20/20 in `.beads/vb-hn4sc/verifier-lane-review.jsonl`.
- **Findings ledger**: 6/6 in `.beads/vb-hn4sc/proof-plan-findings.jsonl` (all `owner_approved_no_action`).
- **Planned obligations**: 6/6 in `.beads/vb-hn4sc/proof-obligations.planned.jsonl` (all schema-valid).
- **Next state**: 5 (proof-writer).
- **Required handoff**: proof-writer authors `crates/vb_storage/src/kani_vb_vzcuf_ps010.rs`
  with `kani::any()` discipline; registers it in `crates/vb_storage/src/lib.rs:76-94` and
  `crates/vb_storage/Cargo.toml` feature `kani-vb-vzcuf`; documents the run command and
  the trusted-base debt for the const assertion.

STATUS: APPROVED

# Proof Plan Review — vb-cib14

## Review Identity

| Field | Value |
|---|---|
| `bead_id` | vb-cib14 |
| `reviewer_skill` | proof-plan-reviewer |
| `reviewer_invocation_id` | femdation-p4b-proof-plan-reviewer-vb-cib14 |
| `planner_invocation_id` | femdation-p4-proof-planner-vb-cib14 |
| `review_state` | 4b (post-planning pre-proof) |
| `host_session_id` | femdation-cheap25-batch |
| `workdir` | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14` |
| `coupled_bead` | vb-edvbj (STRONG release coupling — deletes the `RunFailedEvent` catch-all at `crates/vb_runtime/src/journal/chunk_002.rs:298–302`) |

## Reviewed Artifacts and Hashes

| Artifact | SHA-256 |
|---|---|
| `.beads/vb-cib14/proof-strategy.md` | `9a3b263a084f5516d28018a7f4b8129429999526d79d9156ea04b635dd138a6b` |
| `.beads/vb-cib14/verifier-lane-decisions.jsonl` | `1803bd022cb942b8186243f8254e5bf1d770f72fee97196dc20348605db08b40` |
| `.beads/vb-cib14/proof-obligations.planned.jsonl` | `365e97393e698e3cc8f0342cea8de3acb35dac0e1ab63120a5946105152a8d80` |
| `.beads/vb-cib14/trusted-base-plan.md` | `b2ee54562057197032c1e70e9b0f80ea2412c15baa153bf96591dcf5ce3f404b` |
| `.beads/vb-cib14/waiver-candidates.jsonl` | `9785f620479e3ae488909726c247c4510fa5809e6d27121d67ff8ea37075759c` |

All five artifacts existed before the review started (`reviewed_artifacts_existed_before_start: true`).

## Workspace Provenance

- `pwd -P` resolves to `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14` — isolated JJ workspace.
- `jj root` resolves to the same path (JJ-initialized; no Git co-checkout).
- This is the agent's dedicated workspace under `~/src/isoloated/`, distinct from the main coordination checkout `/home/lewis/src/velvet-ballistics`.

## Counts and Inventory

- Lane decisions: **20** rows in `verifier-lane-decisions.jsonl` (15 required, 5 not_applicable; 0 blocked_tooling).
- Planned obligations: **7** rows in `proof-obligations.planned.jsonl` (PO-001 .. PO-007; all behavior_affecting=true, required=true, owner_state=4, status="planned").
- Trusted base entries: **12** (TB-001 .. TB-012) plus **4 reductions** with soundness rationale.
- Waiver candidates: **1** (`W-NONE-001`, behavior_affecting=false, all lanes required).
- Proof seeds: **10** in `proof-seeds.jsonl` (every seed has lane decisions).
- Lane-review rows (this artifact): **20** in `verifier-lane-review.jsonl`, one per VLD row.

## Production Binding Validation (Mandatory)

PO-001 is the sole Verus obligation. Its `production_binding` field:

```yaml
mechanism: WEAK_EXTERN
production_path: crates/vb_runtime/src/journal/chunk_002.rs
production_lines: "193-268,270-303"
extern_path: verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs
assume_specification_targets:
  - production::boundary_storage_event::Resumed_arm
  - production::convert_resume_timestamp
exec_wrapper_required: true
drift_detection: mirror-drift-gate
drift_gate_script: scripts/check-verus-production-binding.sh
drift_threshold: zero
```

| Required field | Status | Evidence |
|---|---|---|
| `mechanism ∈ {STRONG, WEAK_MIRROR, WEAK_EXTERN}` | PASS | `WEAK_EXTERN` |
| `production_path` exists on disk | PASS | `crates/vb_runtime/src/journal/chunk_002.rs` (416 lines; mapper site verified at line 193–268 and 270–303) |
| `production_lines` non-empty | PASS | `"193-268,270-303"` (boundary_storage_event body + storage_event dispatch) |
| `extern_path` exists | PASS | `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs` (876 lines) |
| extern uses `#[path]` to production or mirror | PASS | Line 174: `#[path = "production_inner/vb_jnz9_journal_event_seq_valid_production.rs"]` (mirror) |
| `assume_specification_targets` non-empty (WEAK_EXTERN requires) | PASS | Two targets: `boundary_storage_event::Resumed_arm` and `convert_resume_timestamp` |
| `MirrorJournalEvent::RunResumed` shape anchor | PASS | Verified at lines 616–624 of extern file (matches `JournalEvent::RunResumed` shape `{ run, seq, timestamp }`) |
| `drift_gate_script` exists (WEAK_EXTERN discipline) | PASS | `scripts/check-verus-production-binding.sh` (6.3K, exists) |
| No `EXPLICITLY_ALLOWED` / `ALLOWED_EXCEPTIONS` escape | PASS | No such field present; clean WEAK_EXTERN |

Production-binding is sound. The mirror encodes `JournalEvent::RunResumed { run: u64, seq: EventSeq, timestamp: u64 }` and the production `JournalEvent::RunResumed { run: RunId, seq: EventSeq, timestamp: DateTime<Utc> }` shape is unchanged by this bead. The drift gate (`scripts/check-verus-production-binding.sh`) is the canonical enforcement.

## Lane-by-Lane Disposition

| VLD | Verifier | Req | Clause | Seed | Disposition | Notes |
|---|---|---|---|---|---|---|
| VLD-001 | proptest | C1 | contract.md#C1 | seed-001 | accepted | PO-002: 65536 random triples + boundary cases; non-vacuity strategy includes 5 distinct cases |
| VLD-002 | verus | C1 | contract.md#C1 | seed-001 | accepted | PO-001 WEAK_EXTERN binding (verified above) |
| VLD-003 | source-lint | C1 | contract.md#C1 | seed-001 | accepted | PO-006 panic-surface + hot-cold + verus-binding |
| VLD-004 | proptest | C2 | contract.md#C2 | seed-002 | accepted | PO-003 u64 sweep with sentinels [0, 1, 1_700_000_000, i64::MAX, i64::MAX+1, u64::MAX, u64::MAX-1] |
| VLD-005 | verus | C2 | contract.md#C2 | seed-002 | accepted | PO-001 Verus spec companion to proptest |
| VLD-006 | source-lint | C2 | contract.md#C2 | seed-002 | accepted | PO-006 forbids `as i64` cast on `u64` |
| VLD-007 | cargo-test | C3 | contract.md#C3 | seed-003 | accepted | PO-004 16-variant enumeration extension at `chunk_004.rs:1077-1090` |
| VLD-008 | source-lint | C3 | contract.md#C3 | seed-003 | accepted | PO-006 surfaces non-exhaustive match warning until vb-edvbj lands |
| VLD-009 | cargo-test | C4 | contract.md#C4 | seed-004 | accepted | PO-004 single-clone regression at `chunk_002.rs:410-493` extended for Resumed arm |
| VLD-010 | loom+proptest | C5 | contract.md#C5 | seed-005 | accepted | PO-005 loom 2-thread × 4 preemptions × 20000 branches + proptest 4096 replay alphabets |
| VLD-011 | proptest | C6 | contract.md#C6 | seed-006 | accepted | PO-002 asserts `mapped.seq() == seq` and `mapped.run_id() == event.run_id()` |
| VLD-012 | verus | C6 | contract.md#C6 | seed-006 | accepted | PO-001 pass-through refinement at `MirrorJournalEvent::RunResumed` |
| VLD-013 | proptest | C7 | contract.md#C7 | seed-007 | accepted | PO-007 asserts struct-variant shape with `run` and `timestamp: u64` fields |
| VLD-014 | source-lint | C7 | contract.md#C7 | seed-007 | accepted | PO-006 confirms `#[non_exhaustive]` attribute present (verified at `crates/vb_runtime/src/error/mod.rs:8`) |
| VLD-015 | flux-rs | ALL | ALL | ALL | accepted (not_applicable) | `limitation_kind=surface_absent`; 3 evidence_refs SHA-256 verified |
| VLD-016 | miri | ALL | ALL | ALL | accepted (not_applicable) | `limitation_kind=surface_absent`; `crates/vb_runtime` is `#![forbid(unsafe_code)]`; verified at `lib.rs:1` |
| VLD-017 | cargo-fuzz | ALL | ALL | ALL | accepted (not_applicable) | `limitation_kind=surface_absent`; not a parser/codec surface; strongly-typed enum input |
| VLD-018 | kani | ALL | ALL | ALL | accepted (not_applicable) | `limitation_kind=superseded_by_other_lane_with_evidence`; proptest PO-003 + 16-variant enumeration strictly stronger than CBMC bounds for this claim set |
| VLD-019 | tla-plus | REFINEMENT-RRO-RESUME | rust-refinement-obligations.jsonl:6 | seed-009 | accepted (not_applicable) | `limitation_kind=risk_out_of_scope`; TLA+ removed per master declaration; temporal-replay split into loom+proptest (PO-005) |
| VLD-020 | source-lint | VERUS-MIRROR | contract.md#verus-mirror-binding | seed-008 | accepted | PO-006 drift gate `check-verus-production-binding.sh` + mirror drift `check-production-inner-drift.sh` |

Every planner-owned lane has an independent reviewer disposition row. **20 of 20 accepted.**

## Required-Lane Coverage (Per Verification Lane Policy)

| Default lane | Required? | Plan's call | Status |
|---|---|---|---|
| `verus` | Yes (rust-local + arithmetic + typestate) | VLD-002, VLD-005, VLD-012 (required); PO-001 | ACCEPTED |
| `kani` | Yes (bounded state + panic/overflow/index) | VLD-018 (not_applicable) with rationale: superseded by proptest+enumeration | ACCEPTED — reasoning sound |
| `flux-rs` | Yes (refinements / length / ownership) | VLD-015 (not_applicable) with rationale: surface_absent (no refinement types in `vb_runtime`) | ACCEPTED |
| `proptest` | Yes (property pressure) | VLD-001, VLD-004, VLD-011, VLD-013 (required); PO-002, PO-003, PO-007 | ACCEPTED |

| Conditional lane | Required? | Plan's call | Status |
|---|---|---|---|
| `loom` (concurrency) | Required (temporal-replay has interleaving risk on `Shard::handle_resume` ↔ `append_resumed_event` ↔ recovery read) | VLD-010 (loom+proptest); PO-005 | ACCEPTED |
| `cargo-fuzz` (parsers/codecs) | Not applicable (not a parser/codec surface) | VLD-017 (not_applicable) | ACCEPTED |
| `unsafe / FFI / miri` | Not applicable (no unsafe) | VLD-016 (not_applicable) | ACCEPTED |
| `tla+` (removed) | Per master declaration, TLA+ is removed | VLD-019 (not_applicable); risk covered by loom+proptest | ACCEPTED |

No default lane is silently omitted. No conditional lane is silently omitted. Every `not_applicable` row cites concrete evidence_refs (3 per row, SHA-256 verified) and has a typed `limitation_kind`.

## Obligation Field Validation

All 7 obligations (`PO-001` .. `PO-007`) have every required `proof-obligation/v1` schema field:

- `schema_version`, `id`, `requirement_id`, `contract_clause`, `domain_claim`, `risk`, `risk_tags`, `verifier`, `artifact`, `target`, `command`, `workdir`, `expected_evidence`, `assumptions` (≥ 1 each), `model_bounds`, `tool_metadata`, `trusted_base_refs`, `required`, `behavior_affecting`, `mode`, `owner_state`, `rerun_from`, `status` — all present.
- No legacy alias fields (`layer`, `checker`, `claim` only) populated.
- `workdir` matches the agent's isolated workspace in every row.
- `command` is concrete and runnable (no vague placeholders).
- `expected_evidence` describes exact pass criteria (verifier success messages, proptest `ok`, atomic-counter assertion, lint exit codes).
- `tool_metadata.tool` matches `verifier` (verus@0.2024.10, proptest, cargo, loom, bash).

## Non-Vacuity Plan

Every obligation has a non-vacuity plan:

- **PO-001 Verus**: spec fn `convert_resume_timestamp_spec` is total over `u64`; `ensures(mapped.seq == input.seq)` and `ensures(mapped.run_id == input.run_id)` are proved; no `assume(...)`, no `axiom`, no `external_body`, no `#[trusted]` added.
- **PO-002 proptest**: 65536 cases + 5 explicit strategies (`run != RunId(0)`, `seq != EventSeq(0)`, realistic timestamp `1_700_000_000`, boundary `0`, last-legal `i64::MAX as u64`).
- **PO-003 proptest**: 65536 cases + 7 boundary sentinels including `u64::MAX`, `i64::MAX as u64 + 1`, and the original `u64` value preserved in the `Err` variant.
- **PO-004 cargo-test**: 3 distinct regression tests (single-clone, resumed→run_resumed-once, 16-variant enumeration).
- **PO-005 loom+proptest**: 2-thread loom × 4 preemptions × 20000 branches + 4096 random replay alphabets; regression scenario (legacy `RunFailedEvent`) is also exercised.
- **PO-006 source-lint**: 6 lint scripts each with explicit exit-0 criterion.
- **PO-007 proptest**: 4096 cases + explicit `RuntimeError` variant-shape assertion (struct with `run` and `timestamp: u64`).

## Anti-Laundering Guards (No Vacuum / Cover-Only / Trust-Marker Abuse)

- **No vacuum Verus**: PO-001 binds via `assume_specification_targets` to production `boundary_storage_event::Resumed_arm` and `convert_resume_timestamp`; the mirror drift gate (`scripts/check-verus-production-binding.sh`) is the canonical enforcement; `MirrorJournalEvent::RunResumed` shape confirmed at `extern_vb_jnz9_journal_event_seq_valid.rs:616-624`.
- **No `cover!`-as-proof**: All Verus specs require `ensures` post-conditions; proptest asserts concrete equality and variant shape; no `cover!` used.
- **No `assume`/`axiom`/`admit`/`external_body`**: zero occurrences in the plan or trusted-base.
- **No trust-marker abuse**: `RuntimeError` is already `#[non_exhaustive]` (verified at `crates/vb_runtime/src/error/mod.rs:8`); no new `#[trusted]` or `extern_spec` is added by the plan.
- **No `as i64` cast on `u64`**: explicitly forbidden by PO-006 source-lint gate.
- **No `cover!`-as-proof (Kani)**: not_applicable — no Kani harness.

## Behavior-Affecting Waiver Check

`waiver-candidates.jsonl` contains one row `W-NONE-001` with `behavior_affecting=false`. It is a planning-stage commitment that no behavior-affecting waiver is needed. No `formal-waiver/v1` row exists; no behavior-affecting obligation is waived. **PASS.**

## Coupling Verification (vb-edvbj STRONG-coupled)

The bead is STRONG-coupled to `vb-edvbj` (deletes the synthetic `RunFailedEvent` catch-all at `chunk_002.rs:298-302`). The plan honors this coupling:

- PO-007 explicitly requires the 16-variant enumeration test to pass before vb-edvbj lands (no variant reaches the catch-all except where it's an explicit arm of `boundary_storage_event` for `Resumed` — which is removed in this fix).
- PO-004 single-clone regression is extended with a `Resumed` arm sample that asserts dispatch returns the typed `RunResumed` event exactly once (not the catch-all `RunFailedEvent`).
- VLD-008 source-lint surfaces the structural hazard (`_ =>` arm + non-exhaustive match warning) until vb-edvbj lands.
- PO-005 loom+proptest regression scenario exercises the legacy buggy shape (`Resumed` rewritten as `RunFailedEvent`) and asserts it produces `LifecycleState::Failed` (the bug).

## Bridge Plan (proof → implementation)

PO-005 specifies `crates/workspace_tests/tests/vb_test_runtime_resume_replay.rs` for the loom harness (new file in State 5). PO-002 and PO-003 specify extensions to `crates/vb_runtime/src/journal/tests/chunk_002.rs`. PO-007 specifies extensions to `crates/vb_runtime/src/journal/tests/chunk_004.rs` (16-variant enumeration) plus a new test in `chunk_002.rs` for the typed-error variant. PO-001 specifies the new Verus spec at `verification/verus/vb_cib14_resume_storage_map.rs`. PO-006 specifies the source-lint gate set.

All bridge paths exist (existing test files at `chunk_002.rs`, `chunk_004.rs`, `extern_vb_jnz9_journal_event_seq_valid.rs`; new file paths to be authored in State 5 by `proof-writer`).

## Plan-Quality Gates (Final)

| Gate | Status |
|---|---|
| `pwd -P` resolves to isolated workspace | PASS |
| `git rev-parse --show-toplevel` / `jj root` resolves to same path | PASS (JJ-initialized isolated workspace) |
| Every demanded lane has at least one row | PASS (rust-local × 2, temporal-replay × 1, Verus mirror × 1, source-lint × 2, cargo-test × 2) |
| Every `not_applicable` lane has concrete evidence_refs | PASS (3 SHA-256 refs each, hashes verified) |
| Obligation count in 5-8 range | PASS (7 obligations) |
| Required obligations have non-empty `evidence_command` and `expected_evidence` | PASS |
| No behavior-affecting waiver candidate | PASS |
| Source refs in `path::symbol` form, not prose-only | PASS (every source ref includes file path + line range + symbol) |
| `verifier-lane-decisions.jsonl` is one JSON object per line | PASS (`jq -c '.'` parses 20 lines) |
| `proof-obligations.planned.jsonl` is one JSON object per line | PASS (`jq -c '.'` parses 7 lines) |
| `waiver-candidates.jsonl` is one JSON object per line | PASS (`jq -c '.'` parses 1 line) |
| Verus obligation has production-binding mechanism | PASS (PO-001: `WEAK_EXTERN` with full schema) |
| Lane-decision ↔ lane-review 1:1 correspondence | PASS (20 ↔ 20) |
| Independent planner/reviewer invocation IDs | PASS (`femdation-p4-proof-planner-vb-cib14` ≠ `femdation-p4b-proof-plan-reviewer-vb-cib14`) |

## Findings

No `finding/v1` rows required. Every check passes. The plan is precise enough for `proof-writer` (State 5) and `proof-to-implementation` (State 7).

## STATUS: APPROVED
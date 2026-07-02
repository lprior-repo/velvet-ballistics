# Proof Strategy: vb-cn2v4 — Keys reject zero RunId (P1 bug)

## Bead Identity

- bead_id: `vb-cn2v4`
- title: Keys: reject zero RunId in all key encoders (P1 bug)
- state: 4 (proof-planner)
- prior_state: 3 (rust-contract; produces 9 contract artifacts)
- invocation_id: `femdation:vb-cn2v4:p4:planner:v1`
- isolated_workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4`
- jj_workspace: `cheap25-vb-cn2v4`
- controller: `femdation` (parent dispatcher; this agent is the direct child)

## Strategy Summary

The P1 bug is the encoder/decoder asymmetry: `decode_storage_key` already
rejects `RunId(0)` via `KeyDecodeError::InvalidRunId` at
`crates/vb_storage/src/keys.rs:372-374, 381-383, 400-402, 412-414, 423-425`,
but the encoder side (`run_header_key`, `run_event_key`, `run_snapshot_key`,
`index_status_key`, `index_workflow_key`, `index_action_key`) currently
emits Ok bytes for the same `RunId(0)` input. The fix tightens the
encoder side to mirror the decoder's invariant.

The strategy is **mirror-the-decoder-rejection**: a single private helper
`require_non_zero_run(run) -> Result<(), JournalError>` is called at the
top of `run_only_key`, `sequenced_run_key`, `index_status_key`,
`index_workflow_key`, and `index_action_key`. The six public encoders
inherit the rejection through their call-graph. The existing variant
`JournalError::InvalidRunId { run: RunId }` (diagnostic code `0x4021`,
symbolic `INVALID_RUN_ID`) is reused — no new error variant is added.

The proof strategy proves three things:

1. **Rust-local rejection invariant** — for every public run-bearing
   encoder fn, `run.get() == 0` ⇒ `Err(JournalError::InvalidRunId { run })`
   and `run.get() != 0` ⇒ `Ok(bytes)` whose layout is unchanged.
2. **Encoder/decoder symmetry** — every byte sequence the encoder emits
   is acceptable to the decoder, and every byte sequence the decoder
   rejects is never produced by the encoder.
3. **No collateral damage** — `RunId::new` and `RunId::ZERO` invariants
   are preserved; the decoder side is untouched; out-of-scope surfaces
   (recovery `NoRecoveryData` placeholders, TLA+ spec mirror, runtime
   tests that build `RunId::new(0)` without reaching an encoder) are
   unchanged.

The lane profile is **rust-local + kani + verus** (per femdation
directive). `loom`, `miri`, `cargo-fuzz`, and `flux-rs` are explicitly
out of scope for this bead.

## Risk Profile

The seed risk_tags draw from `proof-seeds.jsonl`:

- `vb-cn2v4-seed-001` (C1): `["public-api", "typed-error", "rust-local", "parser-codec"]`
- `vb-cn2v4-seed-002` (C2): `["rust-local", "internal-helper", "refactor"]`
- `vb-cn2v4-seed-003` (C1, C8): `["round-trip", "invariant", "rust-local", "parser-codec"]`
- `vb-cn2v4-seed-004` (C6): `["kani", "proof-harness", "refactor"]`
- `vb-cn2v4-seed-005` (C7): `["verus", "verifier-artifact", "production-binding", "parser-codec"]`
- `vb-cn2v4-seed-006` (C3): `["diagnostic", "symbolic-code", "rust-local"]`
- `vb-cn2v4-seed-007` (C5): `["behavior-test", "test-parity", "rust-local"]`
- `vb-cn2v4-seed-008` (workflow-model): `["public-api", "typed-error", "behaviour-shift", "parser-codec"]`

The dominant risk shape is **Rust-local rejection** with **parser-codec
symmetry**. The contract is a typed-error rejection (every encoder must
surface `Err(JournalError::InvalidRunId)` for `RunId(0)`) and a
round-trip invariant (encoder Ok ⇒ decoder Ok; encoder Err ⇒ decoder Err
for the same `RunId(0)` input).

## Behavior-Affecting Classification

Per femdation directive, every obligation in this plan carries
`behavior_affecting: false`. The justification is the **rejection is
the close-of-gap, not a behavior change**: the decoder already
rejects `RunId(0)`; the encoder's prior emission of `Ok` bytes for
`RunId(0)` was the bug. The fix aligns encoder with decoder
(behavioural invariant that the existing decoder already enforces at
runtime). The contract describes a structural/rejection property, not
a new feature or capability. No `E_BEHAVIOR_WAIVER` concerns arise.
No `rust-refinement-obligation/v1` rows are required (those are
materialized at State 7 by `proof-to-implementation`).

## Lane Selection

| Verifier | Decision | Rationale |
|---|---|---|
| `verus` | required | `rust_local + parser_codec` default profile; mirrors the encoder contract on `SpecKeyEncodeError` (must extend with `InvalidRunId { run: u64 }`); `assume_specification` clauses on run-bearing mirror fns prove the rejection for all `run` values, not just bounded ones. Production-binding gate `scripts/check-verus-production-binding.sh` MUST pass. |
| `kani` | required | `bounded_state + rejection` for `kani_typed_partitioned_ids` split-harness; the current `Err(_) => assert!(false)` arms must be replaced with an explicit rejection-path match (`matches!(..., Err(InvalidRunId { .. }))` when `run_value == 0`) so Kani never spuriously reports a counterexample. C6 is the contract clause. |
| `proptest` | required | `property + property_test` default profile; per-prefix `encoder_rejects_zero_run_id_for_every_prefix` exercises all six public encoder entry points with `RunId::new(0)`; mutation-resistance proptest confirms the guard is not removable. |
| `flux-rs` | not_applicable | The validation is a single `== 0` integer compare inside a private helper; encoding it as a Flux refinement adds the same predicate the helper already enforces. Surface absent — no `refinement`/`ownership`/`index` tag in seed risk_tags. |
| `loom` | not_applicable | Encoders are pure synchronous; no `Send`/`Sync` boundary; no async; no threads. See `boundary-map.md` SHA-256. |
| `miri` | not_applicable | `vb_storage` sets `#![forbid(unsafe_code)]`; no FFI, no raw pointers, no `unsafe` block. See `boundary-map.md` SHA-256. |
| `cargo-fuzz` | not_applicable | Encoders are typed-input (typed `RunId`, `EventSeq`, `StepIdx`, etc.), not byte-stream parsers. The contract explicitly notes fuzz is "optional/friendly-evidence" but not required. proptest covers the rejection space efficiently at 10k cases; Kani provides bounded symbolic coverage. Surface absent — no `parser`/`codec`/`hostile_input` tag in seed risk_tags. |

## Production-Binding Plan (Verus)

Per the proof-planner SKILL.md mandate, every Verus obligation MUST
include a `production_binding` field. The mechanism for this bead is
**WEAK_EXTERN** (the existing `extern_vb_storage_keys.rs` companion
module). Rationale:

- The mirror file is `verification/verus/extern_vb_storage_keys.rs`
  (an `extern_*.rs` companion module under `verification/verus/`).
- It binds to production code via the existing direct
  `#[path = ".../crates/vb_storage/src/..."]` patterns the project
  uses for constants and types. New mirror fns for
  `require_non_zero_run` must bind to the production helper via
  `assume_specification`.
- The `scripts/check-verus-production-binding.sh` gate treats
  `extern_*.rs` files as companion modules and exempts them; the
  new variant + contracts in the spec file
  `vb_storage_keys_spec.rs` are the verified surface.
- `scripts/check-production-inner-drift.sh` MUST pass; the mirror
  body of `run_event_key` / `journal_key` / `encode_key` must
  return `Err(SpecKeyEncodeError::InvalidRunId { run })` for
  `run == 0`, and the production body must return
  `Err(JournalError::InvalidRunId { run })` for the same input.

The full binding map is recorded in `proof-to-implementation-input.md`
(separate artifact owned by State 7 `proof-to-implementation`); this
plan's `proof-obligations.planned.jsonl` row for the Verus obligation
contains the inline `production_binding` JSON object per the schema.

## Obligation Profile (5-7 obligations)

Six obligations across the three required lanes:

1. **PO-001-VERUS-MIRROR** (verifier: verus) — extend
   `SpecKeyEncodeError` with `InvalidRunId { run: u64 }`; add
   `assume_specification` contracts on the run-bearing mirror fns.
2. **PO-002-VERUS-DECODER-SYMMETRY** (verifier: verus) — the mirror
   fn body returns `Err(SpecKeyEncodeError::InvalidRunId { run })`
   for `run == 0`; production-binding gate passes; mirror drift gate
   passes.
3. **PO-003-KANI-SPLIT-HARNESS** (verifier: kani) — reorganise
   `kani_typed_partitioned_ids.rs::assert_key_contracts` to
   distinguish the `run_value == 0` rejection path from the
   `run_value != 0` happy path; `kani::cover` reachability proves
   both arms are reachable.
4. **PO-004-KANI-ORDER-OF-CHECKS** (verifier: kani) — for
   `index_status_key`, the new `InvalidRunId` rejection fires
   before the existing `IndexStatusStateCollision` check; the
   `Other(0..2)` collision path with `RunId(0)` is unreachable.
5. **PO-005-PROPTEST-PER-PREFIX** (verifier: proptest) — per-prefix
   `encoder_rejects_zero_run_id_for_every_prefix` covers all six
   public encoder entry points (`run_header_key`, `run_event_key`,
   `run_snapshot_key`, `index_status_key`, `index_workflow_key`,
   `index_action_key`).
6. **PO-006-PROPTEST-MUTATION** (verifier: proptest) — mutation
   resistance: a paired property test asserts the rejection
   contract holds for an `Arc<Mutex<bool>>` flag-controlled guard;
   flipping the flag (simulating a guard removal) causes the
   proptest to fail.

All six obligations share:
- `behavior_affecting: false` (per femdation directive; rejection
  close-of-gap)
- `mode: verify-proof` (no smoke-only lanes)
- `owner_state: 4` (this planner)
- `rerun_from: 4` (any change to the plan restarts the chain)
- `status: planned` (never `PASS`; reviewer and verifier own
  closure)

## Trusted-Base Plan

The plan introduces two trust markers:

- **TB-001**: Verus mirror `SpecKeyEncodeError` is a hand-written
  shadow enum. The `extern_vb_storage_keys.rs` companion module
  pattern is the project's established production-binding
  mechanism; the production-binding gate exempts `extern_*.rs` and
  `production_inner/*` files. The mirror enum is bound to the
  production `JournalError` via the doc-comment header that cites
  `crates/vb_storage/src/error/mod.rs:140-141`.
- **TB-002**: Kani harness `assert_key_contracts` currently uses
  `match ... { Ok(_) => ..., Err(_) => assert!(false) }`. After
  the patch, the rejection arm is `matches!(..., Err(InvalidRunId
  { .. }))`. The harness's symbolic input (`SymbolicKeyInputs`)
  uses `kani::Arbitrary` and `kani::any()` (no hardcoded structural
  inputs). The `kani::cover` reachability evidence is required to
  prove both arms are reachable for the bounded domain.

Both trust markers are non-behavior (structurally necessary
mirror/harness patterns). The full note bodies are in
`trusted-base-plan.md`.

## Waiver Candidates

Two waiver-candidate rows are required by the lane profile (the
default profile includes `flux-rs` and the universal profile
includes `loom`/`miri`/`cargo-fuzz` for surface_absent reasons):

- **WVR-001**: `flux-rs` not_applicable — see `verifier-lane-decisions.jsonl` VLD-FLUX-CN2V4-001
- **WVR-002**: `loom` not_applicable — see VLD-LOOM-CN2V4-001
- **WVR-003**: `miri` not_applicable — see VLD-MIRI-CN2V4-001
- **WVR-004**: `cargo-fuzz` not_applicable — see VLD-FUZZ-CN2V4-001

All four rows have `behavior_affecting: false` (these are
non-behavior surface-absent waivers). Boundary proofs and
compensating evidence are recorded in `waiver-candidates.jsonl`.

## Out-of-Scope Surfaces (Preservation Invariants)

The contract's C9 lists surfaces that MUST remain unchanged.
The proof plan does not add obligations that touch these surfaces;
the proof obligations are scoped to the encoder files only:

- `RunId::new` and `RunId::ZERO` constructor invariants
- `recovery/replay/summary/{derive,apply,tests}.rs` (use
  `RunId::new(0)` as `NoRecoveryData` placeholder)
- Workspace tests that build `RunId::new(0)` without reaching
  an encoder
- TLA+ spec mirror `RunId::Run(0)` placeholder
- Proptest `all_key_functions_are_deterministic` (already excludes
  zero)
- `tests.rs::symbolic_code_table` (already maps `INVALID_RUN_ID`)

The proof obligations assert preservation of these surfaces
indirectly: Kani and proptest exercise the production
encoders, not the out-of-scope paths.

## Forbidden Implementation Shapes (Guard Rails)

The contract C4, C8, C9 list the forbidden shapes that the proof
plan enforces by negative assertion:

- Do NOT add a new `JournalError` variant. The proof obligations
  assert that the rejection is the existing `InvalidRunId { run }`.
- Do NOT touch the decoder. The proof obligations bind to the
  encoder only; the decoder is the unchanged source of truth.
- Do NOT touch `RunId::new` or `RunId::ZERO`. The proof obligations
  pass `RunId::new(0)` to encoders; they do not modify the
  constructor.
- Do NOT remove the manual `if run.get() == 0` check from
  `headers.rs::run_header` (per C4, the manual check may stay as
  defence-in-depth; the proof obligations tolerate both shapes).
- Do NOT add `unwrap`, `expect`, `panic`, `todo`, `unimplemented`,
  or `dbg!` in the encoder paths (per the project engineering
  rules; Kani harnesses naturally catch these via `assert!(false)`).

## Anti-Laundering Discipline

- No `assume(`, `axiom`, `admit`, `sorry`, or
  `#[verifier::external_body]` is permitted in the
  `expected_evidence` or `command` of any Verus obligation. The
  Verus mirror `extern_*.rs` is the documented companion-module
  pattern; the spec file's `assume_specification` clauses are the
  verified surface.
- No `kani::cover!` is the sole property evidence. The
  `expected_evidence` for every Kani obligation cites
  `kani::assert` (or a function contract postcondition) on the
  property claim, with `kani::cover` only as reachability evidence
  for the bounded domain.
- No hardcoded structural inputs in Kani. The harness MUST use
  `kani::Arbitrary` and `kani::any()` (GOD RULE 1). The
  `SymbolicKeyInputs` struct in the existing harness already
  satisfies this; the patch only changes the `match` arms.
- No proptest with `is_ok()` only. Every proptest asserts the
  exact rejection error variant via
  `assert!(matches!(result, Err(JournalError::InvalidRunId { .. })))`.

## Handoff

- `proof-plan-reviewer` at State 4b: review
  `verifier-lane-decisions.jsonl` and `proof-obligations.planned.jsonl`
  for schema and binding discipline.
- `proof-writer` at State 5: author the Verus spec file changes
  and Kani harness edits.
- `proof-to-implementation` at State 7: materialize
  `proof-to-implementation-input.md` into
  `rust-refinement-obligation/v1` rows. (For this bead with
  `behavior_affecting: false`, the bridge may be lightweight or
  empty; the planner does not pre-empt the bridge.)
- `formal-verifier` at State 12: execute the six obligations and
  close the ledger.

## Non-Goals

- No implementation in this plan (proof-planner owns planning
  artifacts only).
- No tests authored in this plan (test-writer / test-planner own
  test artifacts; the contract's 18 test flips are the test-writer's
  scope).
- No production code edits in this plan.
- No CI config changes.
- No waiver that hides behavior (E_BEHAVIOR_WAIVER forbidden).
- No claim of `STATUS: APPROVED` or `PASS`; reviewer and verifier
  own disposition and closure.

## Self-Audit

This plan satisfies the proof-planner skill's EARS contract:

- One `verifier-lane-decision/v1` row per required
  (requirement_id, contract_clause, proof_seed_id, verifier)
  tuple (8 rows total: 2 verus, 2 kani, 2 proptest, 4
  not_applicable for flux/loom/miri/fuzz).
- All `applicability: required` rows have paired
  `proof-obligation/v1` rows with matching `verifier`.
- All `applicability: not_applicable` rows for default-profile
  verifiers (flux-rs) have `non_applicability_evidence_refs`
  with concrete artifact hashes; universal-profile verifiers
  (loom, miri, fuzz) have `non_applicability_evidence_refs` for
  surface_absent reasons.
- No `behavior_affecting: true` rows in
  `waiver-candidates.jsonl` (E_BEHAVIOR_WAIVER forbidden).
- `target` fields parse as `path::symbol` form (e.g.,
  `crates/vb_storage/src/keys.rs::require_non_zero_run`).
- `verifier` enum is in the allowed set.
- `command` has no placeholders and is a single runnable
  shell invocation per obligation.
- `model_bounds` populated for `kani` and `proptest`
  obligations; `verus` obligations have empty `model_bounds`
  (unbounded invariant).
- `tool_metadata.version_pin` set for all six obligations.
- Six obligations total (within the 5-7 femdation envelope).
- Two trust markers (TB-001, TB-002) in `trusted-base-plan.md`.

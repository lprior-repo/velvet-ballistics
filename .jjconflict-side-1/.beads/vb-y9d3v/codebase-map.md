# Codebase Map — vb-y9d3v State 2 Explore

## Scope

- Bead: `vb-y9d3v`
- Goal: fresh replacement scope for ActionTicket generation fencing, retries, stale authority, verifier lanes, and prior 36-obligation evidence.
- Isolated workspace: `/home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-y9d3v`
- Prior capped context only: `/home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.5`
- Production Rust edited by this explore pass: **none**.

## Primary production files and symbols

| Path | Symbols / lines read | Why it is in scope | Risk tags |
| --- | --- | --- | --- |
| `crates/vb_core/src/action.rs` | `ActionTicket` lines 136-153; `compute_action_idempotency_key` lines 155-167; `action_ticket_has_valid_key` lines 169-173; `ActionContract` lines 84-107; retry/idempotency enums lines 12-64 | Defines the public ticket authority fields (`run`, `step`, `seq`, `action`, `attempt`, `idempotency_key`, `capacity`) and canonical key contract. | public-api; idempotency; arithmetic |
| `crates/vb_runtime/src/engine/action.rs` | `execute_do` lines 20-74; `resume_action_outcome` lines 138-190; `compute_idempotency_key` lines 202-208 | Generates first ActionTicket with attempt=1/capacity from retry policy and computes retry tickets with checked `seq`/`attempt` increments. | generation-fence; retry; arithmetic; idempotency |
| `crates/vb_runtime/src/shard/transitions.rs` | `await_action` lines 88-120; `finish_run` lines 69-85; `apply` lines 16-61 | Runtime scheduling fence: normalizes scheduled tickets, records scheduled attempts, journals `ActionScheduledTicket`, removes terminal run state. | state-machine; terminal-fence; journaling |
| `crates/vb_runtime/src/shard/helpers.rs` | `validate_action_completion` lines 28-44; `validate_ticket_attempt` lines 72-94; `normalize_scheduled_ticket` lines 96-114; `record_scheduled_attempt` lines 188-198; `retry_policy_after_action` lines 224-270; `record_retry_attempt` lines 273-294 | Core pure-ish logic for stale attempt rejection, zero/over-capacity rejection, scheduled attempt promotion, retry policy extraction, and checked retry generation. | stale-authority; retry; arithmetic; state-machine |
| `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` | `handle_action_completion` lines 369-408; `handle_action_failure` lines 433-465; `ticket_with_retry_capacity` lines 467-485; `apply_action_failure_to_state` lines 487-505 | Mutation boundary for completion/failure journaling. Preflight happens before completion journal; failure path validates before retry/error handling then journals `ActionFailed`. | journaling; stale-authority; retry; terminal-fence |
| `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | `apply_drive_result` lines 181-205; `apply_awaiting_action` lines 207-215 | Dispatches engine `AwaitingAction` to shard scheduling. | state-machine; generation-fence |
| `crates/vb_runtime/src/shard/lifecycle/chunk_003.rs` | `preflight_action_completion` lines 48-78; `reject_invalid_ticket_key` lines 80-91; `retry_is_available` lines 183-195; `apply_error_handler` lines 197-215 | Completion preflight validates ticket, canonical key, contract/output/taint/size before mutation; retry helper delegates to `record_retry_attempt`. | idempotency; retry; output-boundary; private-api |
| `crates/vb_runtime/src/shard/timer_wheel.rs` | referenced by current Kani artifact; not fully read in this pass | In scope for stale timer authority and generation monotonicity if bead retains timer side of original vb-8mdp.5 scope. | temporal; stale-authority |

## Current tests and executable behavior evidence targets

| Path | Evidence found | Notes / gaps |
| --- | --- | --- |
| `crates/vb_runtime/src/shard/helpers/tests.rs` | Lines 1306-1436 cover matching attempt acceptance, stale attempt rejection without state change, zero attempt, zero capacity, attempt beyond max. Lines 1438-1523 cover scheduled-ticket normalization/promotion and over-capacity rejection. | Strong unit coverage around helper functions; not named `vb_8mdp_5`, so prior planned property filter would still run zero tests unless changed. |
| `crates/vb_runtime/src/shard/lifecycle_tests/chunk_004.rs` | Lines 2-41 show future attempt completion is currently accepted when within capacity; lines 43-79 reject beyond max; lines 81-161 assert stale completion leaves run/counters/journal/frame/trace unchanged. | Important domain decision: current freshness is lower-bound (`attempt >= current`), not exact equality. Downstream contract must either preserve or explicitly change this. |
| `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs` | Lines 2-32 and 62-95 assert `ActionFailed` journal ordering before run failure / handler step; lines 111-123 cover retry exhaustion journaling. | Useful for failure mutation order, but not a generated property suite for the 36 prior obligations. |
| `crates/vb_runtime/src/verification/proptest/mod.rs` | Property tests cover `IdempotencyTracker` duplicate and eviction behavior, not shard ActionTicket generation fence. | Existing proptest lane does not close prior `vb_8mdp_5` property obligations. |
| `crates/workspace_tests/tests/*` | Grep found many public `ActionTicket` integration tests (e.g. `vb_vt2f_direct_runtime_api_acceptance.rs`, `vb_test_runtime_lifecycle_state_behavior.rs`, `vb_test_runtime_ipc_resource_behavior.rs`). | Candidate downstream behavior-test scope; not exhaustively read in State 2. |

## Verification/proof artifacts in fresh workspace

| Path | Status in fresh workspace | Consequence |
| --- | --- | --- |
| `crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs` | Exists, read. Header says vb-8mdp.5 obligations. It imports `reject_invalid_ticket_key` from `crate::shard::helpers`, but the read implementation has `reject_invalid_ticket_key` as a private function in `lifecycle/chunk_003.rs`, not in helpers. It also uses `WorkflowParts::try_from_parts(...).unwrap()` and a hardcoded minimal workflow shape despite the no-hardcoded-shapes rule. | Treat as suspect/unclosed until wired, compiled, and repaired to current APIs and generator requirements. |
| `crates/vb_runtime/Cargo.toml` | Features read lines 25-35. No `vb-8mdp-5` feature exists. | Prior planned command `KANI_FEATURES=vb-8mdp-5 cargo kani ...` has no matching feature in this fresh main snapshot. |
| `crates/vb_runtime/src/lib.rs` | Read lines 62-76 and 95-98. No module wiring for `src/verification/kani/...`; only test `verification::proptest` is wired. | Kani/Flux/Verus artifacts under `src/verification` are not automatically compiled by package lanes unless wired. |
| `scripts/flux-check-package.sh` | Exists now; read lines 1-21. | Fresh main fixed the prior missing script blocker, but package-level Flux still only proves wired refinements. |
| `contracts/proof_obligations.yaml` | Grep found registry Verus targets, but no `vb_8mdp_5` targets. | `scripts/verify-verus.sh` will not close vb-y9d3v/vb-8mdp.5-specific Verus obligations without registry updates or approved single-file bridge. |
| `crates/vb_runtime/src/verification/flux/vb_8mdp_5_001.rs` | Missing in fresh workspace. | Prior Flux sketches are not present on clean main. Must be recreated or replaced if this bead repeats those obligations. |
| `crates/vb_runtime/src/verification/verus/vb_8mdp_5_001.rs` | Missing in fresh workspace. | Prior detached Verus sketches are not present on clean main. |
| `verification/tla/vb_8mdp_5_001.tla` | Missing in fresh workspace. | Prior five passing TLA+ models are not present on clean main; prior evidence may be context only, not reusable closure. |
| `crates/vb_runtime/src/models/loom/timer_fired_cancel.rs` | Exists; not read deeply. | Candidate Loom scope for timer stale authority; original Loom lane failed before model execution in prior evidence. |

## Prior 36-obligation evidence context from vb-8mdp.5

Read prior artifacts under `/home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.5/.beads/vb-8mdp.5/`:

- `proof-review.md` lines 15-18: 31 required implementation-bound obligations remained unclosed; only 5 TLA statuses were `verified_temporal_model_alt_command`.
- `proof-review.md` lines 20-23: five behavior-affecting trusted-base blocker rows remained open and pending review.
- `proof-review.md` lines 25-43: Verus detached smoke, Flux package/sketch smoke, Kani failed before harness execution, Loom failed before model execution, property command ran zero matching tests.
- `proof-review.md` lines 45-48: TLA evidence acceptable only as bounded temporal-model evidence, not Rust implementation proof.
- `proof-evidence.md` lines 245-288: prior TLC command via installed `tlc` passed `vb_8mdp_5_001`, `002`, `005`, `006`, `007` as temporal models only.
- `proof-repair-guide.md` lines 7-14: required repairs are Flux wiring, Kani isolation/fix, Verus production binding, Loom cfg/dependency repair, non-zero property tests, trust-ledger closure, and TLA retention only as model evidence.

## Downstream contract/proof/test owners

- `rust-contract`: decide whether ticket freshness must remain lower-bound (`attempt >= current`, current code/tests) or become exact equality. This is a domain-critical fork.
- `proof-planner`: split obligations by current fresh-main reality, not prior branch artifacts. Default lanes: Kani + Flux + Verus + proptest; add Loom/TLA only where temporal/interleaving risk remains.
- `proof-writer`: do not reuse prior missing/unwired sketches as closure. Recreate or wire artifacts under current repo conventions; avoid hardcoded Kani shapes and detached Verus models.
- `test-planner` / `test-writer`: target helper and shard lifecycle tests with non-zero named filters; add public API/integration assertions for stale completions/failures, retry capacity, canonical key rejection, terminal run rejection, and journal non-mutation.

## Key risks / open questions

1. **Fresh-main divergence:** prior TLA/Flux/Verus artifacts are missing in this fresh worktree; only the suspect Kani file remains.
2. **Verifier wiring gap:** no `vb-8mdp-5` feature in `vb_runtime/Cargo.toml`; Kani artifact appears unwired and references a private/wrong-path helper.
3. **Domain decision gap:** code accepts future attempts within capacity (`future_attempt_completion_rejected_when_current_attempt_exists` name contradicts asserted `Ok(true)` behavior). Decide if future attempts are valid authority or stale/future-authority rejection is required.
4. **Mutation-order hazard:** completion path preflights before journaling; failure path validates and computes retry/error outcome before journaling. Tests should prove stale/invalid failure does not append `ActionFailed`.
5. **Prior evidence cannot be copied as approval:** vb-8mdp.5 ended rejected; its five TLA passes are context-only temporal evidence.

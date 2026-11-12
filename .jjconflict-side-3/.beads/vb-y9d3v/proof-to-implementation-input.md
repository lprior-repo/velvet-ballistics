# Proof-to-Implementation Input — vb-y9d3v

Bridge planning artifact mapping every proof obligation to production Rust source refs, behavior test expectations, refinement harness refs, and exact evidence commands. This is input for `proof-to-implementation` (State 7), `test-planner` (State 8), and `holzman-rust` (State 11).

## Obligation-to-Source Mapping

| Obligation ID | Verifier | Production Target | Source File(s) | Symbol(s) | Implementation Impact |
|---|---|---|---|---|---|
| PO-001 | kani | validate_ticket_attempt | crates/vb_runtime/src/shard/helpers.rs:72-94 | `validate_ticket_attempt` | Must reject stale lower attempts with StaleAttempt error; current fresh-main already does this. |
| PO-002 | kani | validate_ticket_attempt | crates/vb_runtime/src/shard/helpers.rs:72-94 | `validate_ticket_attempt` | **IMPLEMENTATION GAP**: Must add future-attempt rejection (currently accepts `attempt > current` within capacity). Needs new condition after line 92 or a new error variant `FutureAttempt`. |
| PO-003 | kani | record_retry_attempt, retry_policy_after_action | crates/vb_runtime/src/shard/helpers.rs:224-294 | `record_retry_attempt`, `retry_policy_after_action` | Current code uses checked arithmetic; behavior must be verified unchanged. |
| PO-004 | kani | preflight_action_completion, reject_invalid_ticket_key | crates/vb_runtime/src/shard/lifecycle/chunk_003.rs:48-91 | `preflight_action_completion`, `reject_invalid_ticket_key` | `reject_invalid_ticket_key` is private; harness must either use public preflight path or extract a pure helper. |
| PO-005 | kani | finish_run, handle_action_completion | crates/vb_runtime/src/shard/transitions.rs:69-85, lifecycle/chunk_001.rs:369-408 | `finish_run`, `handle_action_completion` | Terminal fence must prevent later completions. Fresh-main already returns RunNotFound. |
| PO-006 | kani | cancel_run, kill_run | crates/vb_runtime/src/shard/transitions.rs:69-85 (via Runtime) | `cancel_run`, `kill_run` | Must return RunNotFound for absent runs. Fresh-main behavior must be confirmed. |
| PO-007 | kani | record_retry_attempt, timer replacement | crates/vb_runtime/src/shard/helpers.rs:273-294, timer_wheel.rs:80-88 | `record_retry_attempt`, timer replacement logic | Checked arithmetic must reject overflow. |
| PO-008 | kani | fire_expired, timer wheel generation | crates/vb_runtime/src/shard/timer_wheel.rs:106-128 | `fire_expired` | Stale generation entries must be silently ignored. |
| PO-009 | kani | handle_action_completion, preflight_action_completion | crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:369-505, chunk_003.rs:48-91 | Integration of all preflight checks | Non-mutation integration harness. |
| PO-010 | kani | Fuzz target compilation | fuzz/fuzz_targets/fuzz_retry_codec.rs | `fuzz_retry_codec` | New fuzz target scaffold. |
| PO-011 | verus | validate_action_completion, validate_ticket_attempt | crates/vb_runtime/src/shard/helpers.rs:28-94 | `validate_action_completion`, `validate_ticket_attempt` | Verus `requires/ensures` on exec fn or extracted pure helper proving attempt authority correctness. |
| PO-012 | verus | record_retry_attempt, retry_policy_after_action | crates/vb_runtime/src/shard/helpers.rs:224-294 | `record_retry_attempt` | Verus proof of monotonic retry advancement and capacity bound semantics. |
| PO-013 | verus | handle_action_completion, preflight_action_completion, finish_run | crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:369-505, chunk_003.rs:48-91 | `handle_action_completion`, `preflight_action_completion` | Verus proof of non-mutation for invalid authority. |
| PO-014 | verus | cancel_run, kill_run | crates/vb_runtime/src/shard/transitions.rs:69-85 | `cancel_run`, `kill_run` | Verus proof of typed error for missing runs. |
| PO-015 | verus | fire_expired | crates/vb_runtime/src/shard/timer_wheel.rs:106-128 | `fire_expired` | Verus proof of timer generation freshness check. |
| PO-016 | verus | timer replacement (generation increment) | crates/vb_runtime/src/shard/timer_wheel.rs:80-88 | timer generation increment | Verus proof of checked arithmetic for timer generation overflow. |
| PO-017 | flux-rs | ActionTicket type, validate_ticket_attempt | crates/vb_core/src/action.rs:136-153, crates/vb_runtime/src/shard/helpers.rs:72-94 | `ActionTicket`, `validate_ticket_attempt` | Flux `#[refined_by]` on ActionTicket fields; `#[sig]` on validate_ticket_attempt requiring nonzero attempt/capacity. |
| PO-018 | flux-rs | validate_ticket_attempt attempt comparison | crates/vb_runtime/src/shard/helpers.rs:72-94 | `validate_ticket_attempt` | Flux refinement distinguishing `attempt == current` from `attempt > current`. **Requires implementation fix from PO-002.** |
| PO-019 | flux-rs | record_retry_attempt, retry_policy_after_action | crates/vb_runtime/src/shard/helpers.rs:224-294 | `record_retry_attempt` | Flux `#[sig]` with `ensures` that attempt and seq are incremented by exactly 1. |
| PO-020 | flux-rs | RunState, handle_action_completion | crates/vb_runtime/src/shard/types.rs, lifecycle/chunk_001.rs:369-408 | `RunState`, `handle_action_completion` | Flux refinement on RunState to distinguish live/terminal runs; `#[sig]` on handler requiring live precondition. |
| PO-021 | flux-rs | TimerEntry, fire_expired | crates/vb_runtime/src/shard/timer_wheel.rs:19-37, 106-128 | `TimerEntry`, `fire_expired` | Flux refinement requiring `fired_generation == current_generation` at fire_expired. |
| PO-022 | proptest | validate_ticket_attempt | crates/vb_runtime/src/shard/helpers.rs:72-94 | `validate_ticket_attempt` | proptest strategies for Arbitrary ActionTicket with hostile attempt/capacity values. |
| PO-023 | proptest | record_retry_attempt, retry_policy_after_action | crates/vb_runtime/src/shard/helpers.rs:224-294 | `record_retry_attempt` | proptest properties for retry capacity fence. |
| PO-024 | proptest | handle_action_completion, preflight_action_completion after terminal | crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:369-505 | `handle_action_completion` | proptest property for stale completions after cancel/kill. |
| PO-025 | proptest | cancel_run, kill_run | crates/vb_runtime/src/shard/transitions.rs:69-85 | `cancel_run`, `kill_run` | proptest property for RunNotFound on missing runs. |
| PO-026 | proptest | preflight_action_completion with invalid key | crates/vb_runtime/src/shard/lifecycle/chunk_003.rs:48-91 | `preflight_action_completion` | proptest property for noncanonical key rejection without mutation. |
| PO-0041 | cargo-fuzz | Retry counter encode/decode | crates/vb_runtime/src/shard/helpers.rs:224-294 | Retry counter serialization path | Fuzz harness for arbitrary byte sequences into retry counter decode. |

**Note: No TLA+ obligations exist.** TLA+ has been globally removed from the verifier whitelist. Seed 012 temporal claims remain as design context only; Rust-local invariants are covered by seeds 001-010.

## Implementation Changes Required

Based on the contract and proof plan, the following implementation changes are needed before State 11 (holzman-rust):

### 1. Future-Attempt Rejection (Critical)
- **File**: `crates/vb_runtime/src/shard/helpers.rs:87-93`
- **Change**: After the stale-attempt check (`attempt < current`), add a future-attempt check:
  ```rust
  if ticket.attempt > current {
      return Err(RuntimeError::InvalidActionCompletion);
      // Or prefer: RuntimeError::FutureAttempt { incoming: ticket.attempt, current }
  }
  ```
- **Impact**: Breaking behavioral change; existing tests that accept future attempts must be updated.
- **Affected obligations**: PO-002, PO-011, PO-018, PO-022

### 2. Public `reject_invalid_ticket_key` or Extracted Pure Helper
- **File**: `crates/vb_runtime/src/shard/lifecycle/chunk_003.rs:80-91`
- **Change**: Either make `reject_invalid_ticket_key` `pub(crate)` so Kani harnesses can call it directly, OR extract the pure key comparison logic into `helpers.rs` as a public pure function.
- **Impact**: Module visibility change only; no behavioral change.
- **Affected obligations**: PO-004, PO-009, PO-026

### 3. Verifier Module Wiring
- **File**: `crates/vb_runtime/src/lib.rs`
- **Change**: Add `#[cfg(kani)] mod verification { mod kani; }` wiring so Kani harnesses compile with the crate. Add `vb-8mdp-5` feature flag in `Cargo.toml` for harness isolation.
- **Impact**: Build configuration only.
- **Affected obligations**: PO-001 through PO-010

### 4. Verus Registry Update
- **File**: `contracts/proof_obligations.yaml`
- **Change**: Register `vb-y9d3v-action-fence`, `vb-y9d3v-retry-bounds`, `vb-y9d3v-terminal-fence`, `vb-y9d3v-timer-fence` targets.
- **Impact**: Configuration only.
- **Affected obligations**: PO-011 through PO-016

## Behavior Test Mapping

Each proof obligation requires independent behavior tests (State 8-10) that exercise the same invariants. Key test areas:

| Test Area | Source Files | Covering Proof Obligations |
|---|---|---|
| Attempt fence unit tests (helpers) | crates/vb_runtime/src/shard/helpers/tests.rs | PO-001, PO-002, PO-022 |
| Retry fence unit tests | crates/vb_runtime/src/shard/helpers/tests.rs | PO-003, PO-007, PO-023 |
| Completion preflight lifecycle tests | crates/vb_runtime/src/shard/lifecycle_tests/chunk_004.rs | PO-004, PO-009, PO-024 |
| Failure journal tests | crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs | PO-004, PO-009 |
| Terminal fence integration tests | crates/workspace_tests/tests/vb_test_runtime_lifecycle_state_behavior.rs | PO-005, PO-006, PO-009, PO-024, PO-025 |
| Timer wheel tests | crates/vb_runtime/src/shard/timer_wheel tests | PO-008, PO-015, PO-016, PO-021 |
| Public API/integration ActionTicket tests | crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs | PO-022, PO-024, PO-025 |

## Refinement Harness Mapping (State 7-12)

For each Verus proof obligation, the bridge must produce:

| Proof Obligation | Refinement Harness | Mapped Production Symbol | Behavior Test | Evidence Command |
|---|---|---|---|---|
| PO-011 | `verus_action_fence_refinement.rs` | `validate_action_completion` | `helpers/tests.rs` stale/future attempt cases | `bash scripts/verify-verus.sh --target vb-y9d3v-action-fence` |
| PO-012 | `verus_retry_bounds_refinement.rs` | `record_retry_attempt` | `helpers/tests.rs` retry advancement cases | `bash scripts/verify-verus.sh --target vb-y9d3v-retry-bounds` |
| PO-013 | `verus_terminal_fence_refinement.rs` | `handle_action_completion` | `lifecycle_tests/chunk_004.rs` terminal run cases | `bash scripts/verify-verus.sh --target vb-y9d3v-terminal-fence` |
| PO-014 | `verus_missing_run_refinement.rs` | `cancel_run` | Workspace integration tests | `bash scripts/verify-verus.sh --target vb-y9d3v-missing-run` |
| PO-015 | `verus_timer_fence_refinement.rs` | `fire_expired` | Timer wheel unit tests | `bash scripts/verify-verus.sh --target vb-y9d3v-timer-fence` |
| PO-016 | `verus_timer_overflow_refinement.rs` | Timer generation increment | Timer wheel unit tests | `bash scripts/verify-verus.sh --target vb-y9d3v-timer-gen-overflow` |

## Proof-Verifier-Source Alignment Rules

1. Kani harnesses must use `kani::Arbitrary` or `kani::any()` for core structures; no hardcoded workflow shapes (GOD RULE 1).
2. Verus `proof fn` must bind to production `exec fn` via `requires`/`ensures` (GOD RULE 2).
3. TLA+ must model bounded integers (MAX_U64), not unbounded Nat (GOD RULE 3).
4. If a harness exposes a production code flaw, fix the implementation, not the harness (GOD RULE 4).
5. Verification scope must be trimmed to the call-graph blast radius of this bead (GOD RULE 5).
6. Prior vb-8mdp.5 evidence is context only; no reuse as closure (contract VER-002).
7. Kani `cover!` is non-vacuity evidence only, not satisfaction evidence (lane policy).

# Runtime, Action, and Durability Defects

## P0: Action completion mutates frame before full validation and durable evidence

Evidence:

- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:364-377` validates only ticket shape, writes the output slot, marks the step succeeded, advances the frame, then encodes the value.
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:387-404` appends `SlotWritten`, `StepSucceeded`, and `ActionCompleted` after mutation.

Master violated:

- Section 19: action completion decode must validate ticket/run/step/action equality, output slot bounds, payload length bounds, idempotency policy, and duplicate completion before mutating a frame.
- Section 18: persistence/replay invariants.
- Section 44 points 13-14 and 19.

Impact: If encoding or journal append fails after mutation, in-memory state advances without durable evidence. Recovery can diverge.

Suggested bead: `P0 make action completion two-phase validate-persist-mutate`

## P0: Durable action events lose the ActionTicket and idempotency key

Evidence:

- `crates/vb_runtime/src/journal/chunk_001.rs:47-64` records `ActionScheduled` and `ActionCompleted` with only `run`, `step`, and `action`.
- `crates/vb_runtime/src/journal/chunk_002.rs:96-115` maps scheduled/completed actions to storage events with `attempt: 1` hardcoded.
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:400-404` appends `RuntimeJournalEvent::ActionCompleted` without the ticket.

Master violated:

- Section 19: `ActionTicket` includes `seq`, `attempt`, and `idempotency_key`.
- Section 19: completion payload contains `ActionTicket`, output slot, outcome discriminant, value/failure, taint, encoded length.
- Section 44 point 14: replay mismatch and idempotency policy must be enforceable.

Impact: Recovery cannot enforce duplicate completion, idempotency key equality, real retry attempts, or stale completion rejection from durable evidence.

Suggested bead: `P0 persist full action ticket and completion payload metadata`

## P0: Action output size and taint policy are not enforced on completion

Evidence:

- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:359-404` writes `ActionOutputReady.value` and `ActionOutputReady.taint` directly.
- No checked evidence in that path for `encoded_len <= ActionContract.max_output_bytes`, `ResourceContract.max_blob_bytes`, input-taint comparison, idempotency-class propagation, declassification, or duplicate completion.

Master violated:

- Section 19 action ABI and taint propagation rules.
- Section 44 points 11, 14, and 19.

Impact: A caller can complete an action with oversized output metadata or downgrade taint to `Clean` before the runtime persists the lie.

Suggested bead: `P0 enforce action output bounds and taint policy before completion mutation`

## P0: Runtime taint lattice diverges from the normative three-level lattice

Evidence:

- `crates/vb_core/src/value.rs:10-25` defines `Taint::{Clean, DerivedFromSecret, Secret, Random, TimeDependent}`.
- `crates/vb_core/src/value.rs:27-45` joins taint by ordinal and treats `Random`/`TimeDependent` as more restrictive than `Secret`.

Master violated:

- Section 14 requires exactly `Clean < DerivedFromSecret < Secret`.
- Section 47 reinforces the three-level lattice.

Impact: Runtime taint behavior can diverge from validation, action, and finish semantics.

Suggested bead: `P0 restore normative three-level taint lattice or update master with proof`

## P0: Terminal step state can become pending again

Evidence:

- `crates/vb_core/src/frame.rs:38-57` includes `(StepState::Succeeded, StepState::Pending)` as a valid transition.

Master violated:

- Section 45: `Succeeded`, `Failed`, `Cancelled`, and `Skipped` are terminal; only idempotent re-mark is valid.

Impact: Terminal state invariants and replay/state-machine properties are false for `Succeeded`.

Suggested bead: `P0 remove Succeeded-to-Pending transition and repair loop re-entry model`

## P0: ResourceContract shape/defaults violate Section 13 and Phase 45

Evidence:

- `crates/vb_core/src/workflow/mod.rs:189-228` has extra fields `max_transitions_per_tick` and `allows_secret_results`, making the shape differ from the 16-field master contract.
- `crates/vb_core/src/workflow/mod.rs:232-235` defaults `max_steps: 10_000` and `max_constants: u16::MAX`.

Master violated:

- Section 13: exact resource contract fields and limits.
- Phase 45: tightened defaults, no `u16::MAX`.

Impact: Accepted artifacts can carry resource envelopes outside the master contract.

Suggested bead: `P0 reconcile ResourceContract fields and defaults with master`

## P1: `Runtime::new` defaults to dropping all journal events

Evidence:

- `crates/vb_runtime/src/runtime.rs:41-46` constructs `Runtime` with `NoopRuntimeJournal::shared()`.
- `crates/vb_runtime/src/journal/chunk_001.rs:207-225` says `NoopRuntimeJournal` intentionally drops all events and returns `Ok(())`.

Master violated:

- Section 18: Fjall persistence and recovery are product requirements.
- Section 18: volatile/no-persist behavior is valid only for explicit benchmark/test mode.

Impact: The most obvious runtime constructor creates a non-durable engine with successful acknowledgements and no recovery evidence.

Suggested bead: `P1 make non-durable runtime constructors explicit test-benchmark-only paths`

## P1: Collect primitive reads wall-clock time inside runtime primitive logic

Evidence:

- Subagent inspection found `crates/vb_runtime/src/primitives/collect.rs` importing and using `SystemTime::now()` for timeout checks.

Master violated:

- Section 20: deterministic steps run inside shard until suspension.
- Section 45: exact replayable node semantics.

Impact: Collect timeout behavior is ambient and not replayable unless the time source is journaled/capability-gated.

Suggested bead: `P1 replace collect wall-clock reads with shard timer authority`

## P1: Frame pool allocates on empty pool

Evidence:

- Subagent inspection found `FramePool::take` allocating a new `RunFrame` when the pool is empty.

Master violated:

- Section 44 point 12: turbo admission preallocates or reserves hot resources before acceptance.

Impact: Admission can allocate after acceptance instead of failing/reserving explicitly.

Suggested bead: `P1 preallocate or reserve shard frame pools at admission`

## Closure Status (reconciled by vb-ovgnb / vb-o5zb.6, 2026-06-12)

The umbrella bead `vb-o5zb` (P0: reconcile core taint step-state and resource
contracts with master) and all 5 of its direct children are CLOSED. The
following STATUS: REJECTED proof obligations remain deferred — they are
captured here per the differential-verification doctrine to prevent
evidence laundering, NOT scrubbed.

### Closed direct children of vb-o5zb

| Bead ID | Title | Status |
|---------|-------|--------|
| vb-o5zb.1 | restore normative Clean/DerivedFromSecret/Secret taint lattice | CLOSED |
| vb-o5zb.2 | make terminal step states absorbing | CLOSED |
| vb-o5zb.3 | reconcile ResourceContract shape and defaults with master | CLOSED |
| vb-o5zb.4 | route collect timeout semantics through replayable shard timer authority | CLOSED |
| vb-o5zb.5 | audit vb-o5zb child closure evidence | CLOSED |

### STATUS: REJECTED sub-beads (proof rejected, bead closed; obligations deferred)

| Bead ID | Title | Status | Deferred obligation |
|---------|-------|--------|---------------------|
| vb-53k3r | remove Succeeded to Running exception from StepState VALID_TRANSITIONS (vb-o5zb.2 follow-up) | CLOSED | `crates/vb_core/src/frame.rs:54` still contains `(StepState::Succeeded, StepState::Running)` in `VALID_TRANSITIONS`; `vb_proof_kernels/src/step_state.rs:48,105-115` retains the Succeeded special case. CL-TERM-01 unsatisfied. Repair requires removing those entries and updating 4 stale Kani harnesses (`frame.rs:1101`, `frame/tests_and_verification.rs:1627`, `vb_proof_kernels/src/step_state.rs:558-561`, `vb_runtime/src/primitives/reentry_proofs.rs:775-776`) plus `integration_step_behavior.rs:1324-1333` and the `mark_pending` doc comment at `frame.rs:393-394`. |
| vb-izu26 | add test for `CollectPaginationState::from_journal` preservation path (vb-o5zb.4 follow-up) | CLOSED | preservation-path test for `CollectPaginationState` not yet added; collect-timeout replay correctness depends on it. |
| vb-yurs3 | repair stale 5-variant `SpecTaint` in Verus `run_frame_invariant.rs` (vb-o5zb.1 follow-up) | CLOSED | Verus model still uses 5-variant `SpecTaint`; needs repair to match the 4-variant master lattice. |

### Stale citations verified absent from the database

`vb-y8tnv` and `vb-f4x2u` are referenced in the original `vb-ywhqi`
bead description as STATUS: REJECTED sub-beads of `vb-o5zb`, but
`bd show vb-y8tnv` and `bd show vb-f4x2u` both return "no issue found".
They are stale text and do not represent hidden obligations.


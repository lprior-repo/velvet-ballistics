# RunFrame Full Public State and Taint Surface Qualification Contract

Bead: vb-eirmk.15.1 Status: contract (unverified implementation) Scope: exhaustive qualification of all RunFrame public API, state invariants, and taint behavior per velvet-ballistics-MASTER.md Section 42 and master requirements function:24:3.

## Source Evidence

crates/vb_core/src/frame.rs — RunFrame struct, StepState enum, is_valid_step_state_transition, module re-exports, Kani includes.

crates/vb_core/src/frame/parts/impl_001_construct.rs — new and reinitialize.

crates/vb_core/src/frame/parts/impl_002_accessors.rs — run_id, pc, executed, step_count, slot_count, max_parallel_in_flight, set_max_parallel_in_flight, parallel_in_flight, add_parallel_in_flight, sub_parallel_in_flight, set_pc, increment_executed.

crates/vb_core/src/frame/parts/impl_003_slots_taints.rs — read_slot, write_slot, write_slot_with_taint, initialized_slots, slots_snapshot, written_slot_count, taint_snapshot, states_snapshot, read_taint, write_taint, find_handle_taint.

crates/vb_core/src/frame/parts/impl_004_state_machines.rs — mark_running, mark_pending, mark_succeeded, mark_failed, mark_skipped, mark_waiting, mark_asking, mark_cancelled, step_state, write_step_state, validate_transition.

crates/vb_core/src/frame/parts/initialized_slot_entry.rs — free function used by initialized_slots.

crates/vb_core/src/frame/tests_and_verification.rs — existing module tests.

crates/vb_core/src/frame/checkpoint.rs — checkpoint roundtrip tests.

## Step State Enum

StepState variants: Pending, Running, Succeeded, Failed, Skipped, Waiting, Asking, Cancelled. Eight variants total. #[non_exhaustive].

is_valid_step_state_transition(current, new) — pure predicate shared by runtime validation and proof harnesses. Returns true for identical states and the enumerated valid transitions. Runtime mark methods use this via validate_transition which returns CoreResult with InternalInvariantViolation("invalid_state_transition") on failure.

## Constructor

new(run_id, first_step, step_count, slot_count) -> CoreResult<Self> Allocates exactly three boxed arrays: states (step_count x StepState::Pending), slots (slot_count x None), taint (slot_count x Taint::Clean). Initializes run_id, pc=first_step, executed=0, max_parallel_in_flight=u16::MAX, parallel_in_flight=0.

Failure cases:
- step_count == 0 -> CoreError::InvalidCompiledWorkflow { reason: "step_count_zero" }
- first_step >= step_count -> CoreError::InvalidProgramCounter { step }
- slot_count may be 0 (empty slot arrays permitted)

Given a valid RunId and first_step within bounds, new returns Ok with all step states Pending, all slots None, all taints Clean. Given first_step == 0, step_count == 1, slot_count == 0, new returns Ok with zero slot arrays. Given step_count == 0, new returns InvalidCompiledWorkflow error. Given first_step >= step_count, new returns InvalidProgramCounter error.

## Engine Wrapper

new_run_frame(run_id: RunId, workflow: &CompiledWorkflow) -> Result<RunFrame, EngineError> Public wrapper in crates/vb_core/src/engine.rs delegating to RunFrame::new with workflow.entry(), workflow.node_count(), workflow.slot_count(). EngineError aliases CoreError; the wrapper returns RunFrame::new directly and propagates errors unchanged, without mapping or wrapping. Workflow structure is not mutated or validated beyond the dimension and entry checks performed by new.

## Reinitialize

reinitialize(run_id, first_step, step_count, slot_count) -> CoreResult<()> Resets a previously released frame for a new run with identical dimensions. Validates step_count and first_step. Rejects dimension mismatch. Resets run_id, pc, executed=0, max_parallel_in_flight=u16::MAX, parallel_in_flight=0. Iterates states to Pending, slots to None, taints to Clean.

Reinitialize validates in exact order: zero requested step_count -> CoreError::InvalidCompiledWorkflow { reason: "step_count_zero" }; requested first_step outside requested step_count -> CoreError::InvalidProgramCounter { step: first_step }; only after both checks pass, different dimensions -> CoreError::InvalidCompiledWorkflow { reason: "frame_dimension_mismatch" }. Identical dimensions and valid requested entry succeed and reset all fields. Every rejection preserves the complete original frame. Retained paired-invalid cases: zero requested steps plus invalid entry/changed dimensions must return step_count_zero; nonzero changed dimensions plus entry at the requested count must return InvalidProgramCounter before dimension mismatch.

## Read Accessors

run_id() -> RunId — returns the immutable run identifier. pc() -> StepIdx — returns the current program counter. executed() -> u64 — returns the transition counter. step_count() -> u16 — returns the allocated step count. slot_count() -> u16 — returns the allocated slot count. max_parallel_in_flight() -> u16 — returns the configured parallel limit. parallel_in_flight() -> u16 — returns the current in-flight count.

All accessors are const and preserve frame state. Given a frame with run_id R, run_id() returns R. Given a frame with pc P, pc() returns P. Given a frame with executed count N, executed() returns N.

## Setters and Counters

set_max_parallel_in_flight(limit: u16) — sets the parallel limit. No validation; caller sets any u16.

add_parallel_in_flight(count: u16) -> CoreResult<()> Adds count to parallel_in_flight with overflow check and limit check. Failure: overflow -> InternalInvariantViolation("parallel_in_flight overflow"). Failure: exceeds limit -> ParallelLimitExceeded { limit }. Given count that would overflow u16, returns InternalInvariantViolation. Given count that would exceed max_parallel_in_flight, returns ParallelLimitExceeded. Given a valid count within limit, returns Ok and increments counter.

sub_parallel_in_flight(count: u16) -> CoreResult<()> Subtracts count with underflow check. Failure: underflow -> InternalInvariantViolation("parallel_in_flight underflow"). Given count > current parallel_in_flight, returns InternalInvariantViolation. Given count <= current parallel_in_flight, returns Ok and decrements.

set_pc(pc: StepIdx) -> CoreResult<()> Moves program counter after bounds validation. Failure: pc >= step_count -> CoreError::InvalidProgramCounter { step }. Given a valid pc within bounds, returns Ok and updates program counter. Given a pc beyond step_count, returns InvalidProgramCounter.

increment_executed() -> CoreResult<()> Increments executed counter with overflow check. Failure: overflow -> CoreError::StepCounterOverflow. Given a frame not at u64::MAX executed, returns Ok and increments. Given a frame at u64::MAX executed, returns StepCounterOverflow.

## Slot Read/Write

read_slot(slot: SlotIdx) -> CoreResult<&SlotValue> Returns reference to value at index. Failure: slot >= slot_count -> SlotOutOfBounds { slot }. Failure: slot value is None -> SlotUninitialized { slot }. Given a written slot at index, returns reference to the written value. Given an unwritten slot at index, returns SlotUninitialized. Given a slot index beyond slot_count, returns SlotOutOfBounds.

write_slot(slot: SlotIdx, value: SlotValue) -> CoreResult<()> Writes value at index with Clean taint. Delegates to write_slot_with_taint(slot, value, Taint::Clean). Failure: slot >= slot_count -> SlotOutOfBounds { slot }. Given a valid slot index and value, returns Ok and stores the value with Clean taint. Given a slot index beyond slot_count, returns SlotOutOfBounds.

write_slot_with_taint(slot, value, taint) -> CoreResult<()> Writes value and explicit taint at index. Failure: slot >= slot_count -> SlotOutOfBounds { slot }. Given a valid slot index, value, and taint, returns Ok and stores both. Given a slot index beyond slot_count, returns SlotOutOfBounds.

Given a slot written with value V and taint Clean, read_slot returns reference to V and read_taint returns Clean. Given a slot written with write_slot_with_taint and taint Secret, read_taint returns Secret. write_slot implicitly writes Clean taint via write_slot_with_taint.

## Taint Read/Write

read_taint(slot: SlotIdx) -> CoreResult<Taint> Returns taint marker at index. Failure: slot >= slot_count -> SlotOutOfBounds { slot }. Failure: slot value is None -> SlotUninitialized { slot }. Given a written slot with taint T, returns T. Given an unwritten slot, returns SlotUninitialized.

write_taint(slot: SlotIdx, taint: Taint) -> CoreResult<()> Writes taint marker without changing value. Failure: slot >= slot_count -> SlotOutOfBounds { slot }. Failure: slot value is None -> SlotUninitialized { slot }. Given a written slot, returns Ok and updates taint. Given an unwritten slot, returns SlotUninitialized.

Given a slot with value V and taint Clean, write_taint(slot, Secret) preserves V and sets taint to Secret. write_taint preserves slot value and modifies only taint.

## Snapshot and Enumeration

slots_snapshot() -> Vec<Option<SlotValue>> Returns copy of all slot values. Given a frame with N slots, returns Vec of length N with written/unwritten values.

taint_snapshot() -> Vec<Taint> Returns copy of all taint markers. Given a frame with N slots, returns Vec of length N with taint values.

states_snapshot() -> Vec<StepState> Returns copy of all step states. Given a frame with N steps, returns Vec of length N with current states.

initialized_slots() -> CoreResult<Vec<(SlotIdx, SlotValue, Taint)>> Returns compact copy of only initialized slots with their values and taints. Zips slots and taint arrays by index and iterates in ascending order. Failure: u16 index conversion error from slot index (slot count exceeding u16::MAX would produce this; valid frame construction prevents this in practice). Given a frame with some written slots, returns only initialized entries. Given a frame with no written slots, returns empty Vec.

written_slot_count() -> u64 Returns count of slots with Some values. Given a frame with K written slots, returns K. Given a frame with no written slots, returns 0.

find_handle_taint(value: &SlotValue) -> CoreResult<Taint> Looks up taint for object/list handles by scanning slots. Returns Clean for non-handle values. Failure: InternalInvariantViolation("taint_slots_diverged") if slot/taint arrays diverge. Given an object handle that exists in a slot, returns that slot's taint. Given an object handle not in any slot, returns Clean. Given a scalar value, returns Clean.

## State Machine Transitions

step_state(step: StepIdx) -> CoreResult<StepState> Returns current state at step index. Failure: step >= step_count -> StepStateOutOfBounds { step }. Given a valid step index, returns the current StepState. Given a step index beyond step_count, returns StepStateOutOfBounds.

mark_running(step) -> CoreResult<()> Transition to Running. Valid from: Pending, Waiting, Asking (and identical Running). Failure: invalid current state -> InternalInvariantViolation. Failure: step out of bounds -> StepStateOutOfBounds. Given a Pending step, mark_running transitions to Running. Given a Running step, mark_running is valid (idempotent).

mark_succeeded(step) -> CoreResult<()> Transition to Succeeded. Valid from: Pending, Running, and identical Succeeded. Failure: invalid current state -> InternalInvariantViolation. Failure: step out of bounds -> StepStateOutOfBounds. Given a Running step, mark_succeeded transitions to Succeeded. Given a Pending step, mark_succeeded transitions to Succeeded.

mark_failed(step) -> CoreResult<()> Transition to Failed. Valid from: Pending, Running, and identical Failed. Given a Running step, mark_failed transitions to Failed. Given a Succeeded step, mark_failed is invalid.

mark_skipped(step) -> CoreResult<()> Transition to Skipped. Valid from: Pending, Running, and identical Skipped. Given a Pending step, mark_skipped transitions to Skipped. Given a Succeeded step, mark_skipped is invalid.

mark_waiting(step) -> CoreResult<()> Transition to Waiting. Valid from: Running, and identical Waiting (idempotent). Given a Running step, mark_waiting transitions to Waiting. Given a Waiting step, mark_waiting is valid (idempotent). Given a Pending step, mark_waiting is invalid.

mark_asking(step) -> CoreResult<()> Transition to Asking. Valid from: Running, and identical Asking (idempotent). Given a Running step, mark_asking transitions to Asking. Given an Asking step, mark_asking is valid (idempotent). Given a Waiting step, mark_asking is invalid.

mark_cancelled(step) -> CoreResult<()> Transition to Cancelled. Valid from: Pending, Running, and identical Cancelled. Given a Running step, mark_cancelled transitions to Cancelled. Given a Waiting step, mark_cancelled is invalid.

mark_pending(step) -> CoreResult<()> Transition to Pending. Valid from: Succeeded (loop re-entry) and identical Pending. Given a Succeeded step, mark_pending resets to Pending. Given a Running step, mark_pending is invalid.

is_valid_step_state_transition(current, new) returns true for:
- Identical states (all 8)
- Pending -> Running, Succeeded, Failed, Cancelled, Skipped
- Running -> Succeeded, Failed, Waiting, Asking, Cancelled, Skipped
- Waiting -> Running
- Asking -> Running
- Succeeded -> Pending
Note: Running -> Waiting and Running -> Asking are the only suspension transitions. Waiting/Asking can only resume to Running, not directly to terminal states.

## Checkpoint Public API

`checkpoint_v1()` copies every scalar, state, slot and taint into `RunFrameCheckpointV1`; the DTO has no version field. `from_checkpoint_v1(checkpoint)` consumes that DTO, validates it, and constructs a frame. Both methods are bound to `crates/vb_core/src/frame/checkpoint.rs:75` and `:94`; no new persisted representation is proposed.

Restoration validation order and exact `RunFrameCheckpointV1Error` outcomes: states length exceeding u16 -> `StepCountOverflow`; slots length exceeding u16 -> `SlotCountOverflow`; zero states -> `EmptySteps`; unequal slot/taint lengths -> `SlotTaintLengthMismatch`; an uninitialized slot carrying non-clean taint -> `TaintWithoutValue`; parallel count above its configured maximum -> `ParallelCountOutOfBounds`; failure of `RunFrame::new` (including PC outside states) -> `InvalidFrame`. There is no `InvalidVersion`, `DimensionMismatch` or `InvalidStepState` checkpoint variant.

Existing native candidates are `vb_core / frame::checkpoint::tests::frame_checkpoint_roundtrips_all_frame_state` and `vb_core / frame::checkpoint::tests::frame_checkpoint_rejects_taint_for_an_uninitialized_slot`. Their names do not close the remaining validation branches. The exact seven rejection branches, pairwise-invalid precedence cases, copied DTO independence, and whole-state roundtrip remain retained qualification obligations until their assertions are individually reviewed.

## Method Failure Atomicity

For a publicly reachable valid frame, rejected `reinitialize`, `set_pc`, `increment_executed`, `add_parallel_in_flight`, `sub_parallel_in_flight`, every mark method, and slot/taint write must return the exact error specified by their actual source and preserve the entire prior frame. Each planned negative test clones the populated frame before the call and compares the complete frame afterward. This is a required oracle, not a claim that every existing test already checks it.

`set_max_parallel_in_flight` accepts any u16, including a limit below the current count; it has no rejection contract. `from_checkpoint_v1` consumes a DTO and either returns a new frame or a typed error; it does not mutate an existing frame.

`new_run_frame(run_id, workflow)` is bound to `crates/vb_core/src/engine.rs:40`: it calls `RunFrame::new(run_id, workflow.entry(), workflow.node_count(), workflow.slot_count())`. `EngineError` aliases `CoreError`; errors propagate unchanged, with no wrapping/conversion layer. Positive qualification compares frame run ID, entry, dimensions, default state and taint with the accepted workflow and verifies that workflow remains unchanged. Any proposed negative wrapper fixture must first demonstrate that it is constructible through current compiled-workflow APIs; impossible invalid workflows are not fabricated or counted as exercised failure coverage.

## Existing Test Registration

nextest catalog discovers 41 frame tests via rust-suites['vb_core']['testcases'] with exact names:

- 39 tests in vb_core::frame::tests_and_verification::tests (frame_new, frame_reinitialize, frame_read_slot, frame_write_slot, frame_mark_running, frame_step_state, frame_taint, parallel_in_flight, reinitialize, security, ut_terminal)
- 2 tests in vb_core::frame::checkpoint::tests (frame_checkpoint_roundtrips, frame_checkpoint_rejects_taint)

The remaining frame::tests::* entries (91 tests) are IPC command wire protocol tests for frame::ipc, not RunFrame state qualification. They are a separate subsystem and not in scope for this bead.

Existing test names from catalog (exact 41 RunFrame tests):
- frame_new_first_step_at_exact_step_count_returns_invalid_program_counter
- frame_new_first_step_out_of_bounds_returns_invalid_program_counter
- frame_new_slot_count_zero_creates_valid_frame_with_empty_slots
- frame_new_step_count_zero_returns_invalid_compiled_workflow
- frame_reinitialize_first_step_out_of_bounds_returns_invalid_program_counter
- frame_reinitialize_resets_all_step_states_to_pending
- frame_reinitialize_step_count_zero_returns_invalid_compiled_workflow
- frame_read_slot_out_of_bounds_returns_slot_out_of_bounds
- frame_read_taint_out_of_bounds_returns_slot_out_of_bounds
- frame_read_uninitialized_slot_returns_slot_uninitialized
- frame_set_pc_accepts_valid_step
- frame_set_pc_rejects_out_of_bounds_step
- frame_step_state_out_of_bounds_returns_error
- frame_taint_roundtrip_write_then_read
- frame_write_slot_out_of_bounds_returns_slot_out_of_bounds
- frame_write_slot_with_taint_sets_both_value_and_taint
- frame_all_step_state_variants_are_reachable
- frame_increment_executed_overflow_returns_step_counter_overflow
- frame_mark_running_on_cancelled_step_allows_overwrite
- frame_mark_running_out_of_bounds_returns_step_state_out_of_bounds
- frame_mark_succeeded_on_failed_step_allows_overwrite
- frame_mark_succeeded_on_pending_step_allows_overwrite
- parallel_in_flight_overflow_returns_error
- parallel_in_flight_rejects_configured_limit_without_mutation
- parallel_in_flight_together_start_join_lifecycle
- parallel_in_flight_tracks_current_after_join_and_respawn
- parallel_in_flight_tracks_peak_when_spawning_branches
- parallel_in_flight_underflow_returns_error
- reinitialize_rejects_dimension_mismatch_without_mutating_frame
- reinitialize_resets_all_hot_state_for_new_run
- security_overwrite_initialized_slot_succeeds
- security_read_slot_distinguishes_uninitialized_from_out_of_bounds
- security_read_taint_on_uninitialized_slot_returns_slot_uninitialized
- security_read_taint_out_of_bounds_returns_slot_out_of_bounds
- security_write_taint_after_slot_cleared_returns_uninitialized
- security_write_taint_on_initialized_slot_succeeds
- security_write_taint_on_uninitialized_slot_returns_slot_uninitialized
- security_write_taint_out_of_bounds_returns_slot_out_of_bounds
- ut_terminal_state_blocks_transitions
- frame_checkpoint_rejects_taint_for_an_uninitialized_slot
- frame_checkpoint_roundtrips_all_frame_state

## Per-Operation Existing Test Mapping

The preceding 41 exact native names are registration candidates. No operation receives full-scope credit from a name, a successful compile, or a historical status. Existing assertions must be inspected against the row below before a new test is added; equivalent existing coverage is reused. The named executed-overflow test currently checks 100 successful increments and fresh-frame getters; it does not reach MAX or assert an overflow error. The named pending-to-succeeded test first calls `mark_running`, so it does not demonstrate direct Pending -> Succeeded. No direct `mark_pending` call was found in those 39 module test functions. Reachability examples do not establish idempotence.

Every `PLANNED` row is a precise retained obligation of vb-eirmk.15.1, not an implementation-defect or missing-coverage claim beyond the inspected source. Planned functions belong in a bounded test module under `crates/vb_core/src/frame/`; names below are proposed names, never claimed native registrations. Source behavior stays unchanged unless a separately reproduced defect is assigned a bead.

| Operation / proposed check | Given / When / required Then assertions | Status |
|---|---|---|
| `new`, `new_run_frame`; `frame_constructor_and_wrapper_contract` | Valid nonzero dimensions and accepted workflow -> exact run_id, pc, step_count, slot_count, executed=0, all Pending, all slots absent and taints Clean; constructor zero steps and first PC at count -> exact source errors. Wrapper delegates exact accepted-workflow dimensions, leaves workflow unchanged; no fabricated invalid workflow. | PLANNED assertion reconciliation; reuse exact constructor registrations above |
| `reinitialize`; `frame_reinitialize_full_state_contract` | Populate every mutable field, then same dimensions/new run/entry -> exact reset defaults and new identity; zero steps, out-of-range entry and dimension mismatch -> exact source error and whole original frame equality. Paired-invalid cases enforce zero-step rejection before entry/dimension checks and requested-entry rejection before dimension mismatch. | PLANNED reconciliation; reuse existing reinitialize assertions first |
| `run_id`, `pc`, `executed`, `step_count`, `slot_count`, `max_parallel_in_flight`, `parallel_in_flight`; `frame_scalar_getter_contract` | Construct/populate known frame and perform valid changes -> each of the seven public getters equals its independent expected value; getters leave whole frame unchanged. | PLANNED assertion reconciliation for all seven getters |
| `set_pc`; `frame_pc_failure_atomicity` | Valid last step -> getter matches; PC at count and maximum StepIdx outside the frame -> exact InvalidProgramCounter payload and full-frame equality. | PLANNED; existing valid/out-of-bounds names are candidates |
| `increment_executed`; `runframe_executed_counter_exact_boundary` | Valid populated frame seeded with checked MAX-1 counter -> public increment succeeds at MAX; next call -> exact StepCounterOverflow and complete frame equality, including nondefault slot/value/taint/state. | Local3090 child vb-eirmk.15.1.1 candidate; registration/execution/reviews tracked by that child, not accepted here |
| `set_max_parallel_in_flight`, `add_parallel_in_flight`, `sub_parallel_in_flight`; `frame_parallel_counter_contract` | Accept any u16 configured maximum; exact within-bound add/sub results; checked overflow, configured-limit rejection and underflow -> exact source error and complete unchanged frame. Setting maximum below current count succeeds; no invented setter rejection. | PLANNED reconciliation; retain existing parallel-counter candidates |
| `read_slot`, `read_taint`, `write_slot`, `write_slot_with_taint`, `write_taint`; `frame_slot_taint_contract` | Independent initialized/uninitialized and last-valid/out-of-range slots -> exact value/taint/error payloads. write_slot resets taint Clean; explicit writes store requested taint; write_taint preserves value. Rejected writes leave complete frame unchanged. | PLANNED reconciliation; existing security and slot tests are candidates |
| `initialized_slots`; `frame_initialized_entry_contract` | Empty and sparsely initialized valid frame -> only initialized (SlotIdx,value,taint) entries in ascending index order; overwrite changes value without duplicate entry. Source uses zipped arrays; helper conversion error is `InternalInvariantViolation { reason: "slot index exceeds SlotIdx range" }`. Valid constructor/restoration bounds keep indexes representable; no invented array-divergence detector or unreachable corruption fixture. | PLANNED exact ordering/value/taint oracle |
| `written_slot_count`; `frame_written_slot_count_contract` | Valid frames with 0, 1, multiple and maximum u16 slots -> exact initialized-entry counts; overwriting an initialized slot does not increase count. Maximum reachable count is u16::MAX (65535), not u64::MAX. | PLANNED reachable bounded-count oracle |
| `slots_snapshot`, `taint_snapshot`, `states_snapshot`; `frame_snapshot_contract` | Populate all arrays including absent slots and nondefault taint/state -> exact full contents/order/length; modifying returned Vec copies leaves original frame unchanged. | PLANNED snapshot completeness and independence |
| All eight marks and `step_state`; `frame_all_state_pairs_contract` | Every pair from the actual eight-state domain -> corresponding mark method produces exact permitted target or exact rejection with complete frame unchanged; include all eight same-state pairs and all eight mark_pending sources. Every method also rejects out-of-range step with exact payload. Expected matrix is bound to the actual predicate, with master disagreements explicitly escalated rather than silently waived. | PLANNED 64-pair matrix and bounds; existing reachability/terminal names are partial candidates |
| `checkpoint_v1`, `from_checkpoint_v1`; `frame_checkpoint_validation_contract` | Exact full populated-state copy/roundtrip and independent DTO Vec copies; each of the seven actual validation branches and paired-invalid precedence -> exact error in source order, with no fabricated version field. | Two exact registrations retained; remaining branch/assertion reconciliation PLANNED |

Native candidate registration is distinct from execution and acceptance. Full vb-eirmk.15.1 stays open until every row has reviewed exact assertions, scoped raw results, independent CODE and TEST review, and the applicable integrated master gates. Approval of this contract or of a bounded child does not close that parent.

## Verification Commands

Proposed scoped command for this bead:
```bash
CARGO_BUILD_JOBS=2 RUSTC_WRAPPER= RUSTFLAGS=-Dwarnings rustup run nightly-2026-04-28 cargo nextest run --cargo-quiet -p vb_core --all-features -E 'test(frame::)'
```

Canonical proving command copied from actual Moon config:
```bash
BEADS_DOLT_SERVER_PORT=37225 moon run :fmt :check :test
```

Expected: all 41 existing RunFrame tests pass with zero failures and zero panics. New tests from the Planned Behavioral Tests section are expected to be written, registered, and pass in future beads. This contract and its mappings remain WIP until independent review; full parent acceptance is not claimed.

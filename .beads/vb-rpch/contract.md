# Contract Specification — vb-rpch

## Context

- **Bead**: vb-rpch — bdd: Durability and recovery acceptance scenarios
- **Domain terms**: Recovery, journal replay, Fjall persistence, durability profiles, RecoveryError, RecoveryFrameSeed, RecoveryHydration, RecoveryRuntimeSummary, UnsupportedRecoveryState, ActionReplayTracker, DigestCheck
- **Assumptions**: Fjall journal provides ordered durable events; RecoveryFrameSeed is sufficient to rebuild a live RunFrame; journal replay is deterministic
- **Open questions**: None

---

## Preconditions

- **PRE-001**: `hydrate_run_frame(snapshot, tail_events, run_id)` — `snapshot.run == run_id`; every `tail_event.run_id() == run_id`; every `tail_event.seq() > snapshot.seq`; snapshot bytes are decodable; derived `step_count > 0` and fits in `u16`
- **PRE-002**: `hydrate_run_frame_from_events(events, run_id)` — `events` is non-empty; derived `step_count > 0` and fits in `u16`
- **PRE-003**: `check_workflow_source_digest(journal, run, expected)` — journal contains a `RunAccepted` event for `run`; otherwise returns `NoRecoveryData`
- **PRE-004**: `recover_runtime_summary(journal, run)` / `recover_runtime_frame_seed(journal, run)` — journal contains events for `run`; otherwise returns `NoRecoveryData`
- **PRE-005**: `recover_full_journal` / `recover_snapshot_plus_tail` — events are non-empty; tracker is non-null; snapshot seq is strictly less than all tail event seqs

---

## Postconditions

- **POST-001**: `check_workflow_source_digest` returns `Ok(())` iff the stored `RunAccepted.workflow == expected`; returns `WorkflowSourceDigestMismatch` on mismatch; returns `NoRecoveryData` when no acceptance event found
- **POST-002**: `check_compiled_ir_digest` returns `Ok(())` iff digests are equal; otherwise `CompiledIrDigestMismatch`
- **POST-003**: `verify_digests` returns `Ok(())` only when ALL required digests match at the given `DigestCheck` level (workflow, IR); `ActionAbiMismatch` and `PolicyDigestMismatch` verification is GAP-3 deferred (not reachable via public API at `DigestCheck::Full`)
- **POST-004**: `recover_runtime_summary` returns `RecoveryHydration::Summary` with accurate counts for steps_started, steps_succeeded, actions_scheduled, actions_resolved, suspensions, slots_written, and correct `terminal` derived from latest terminal event of max attempt
- **POST-005**: `recover_runtime_frame_seed` returns `RecoveryFrameSeed` with exact `step_count`, `slot_count`, `first_step`, `pc`, `steps`, `slots`, `pending_actions`, and `unsupported` markers derived from events
- **POST-006**: `hydrate_run_frame` returns `RunFrame` whose slot values and taint match `snapshot` plus all `tail_events` effects; frame PC and executed count reflect replay; `max_parallel_in_flight` reflects observed peak
- **POST-007**: `hydrate_run_frame_from_events` returns `RunFrame` with step states, slots, PC, and executed count reconstructed from events-only; `unsupported` field correctly marks any missing slot_values, slot_taint, action_payloads, or pending_actions
- **POST-008**: `recover_all_incomplete_runs` returns `Vec<RecoveryHydration>` for every run header whose journal has no terminal event from the latest attempt; never returns a run that has already terminated
- **POST-009**: `replay_events` skips all state-affecting events from attempts older than `max_attempt`; marks actions as completed/failed in tracker; blocks re-execution of already-resolved non-idempotent actions with `NonIdempotentActionBlocked`; detects out-of-order step execution with `ReplayDivergence`
- **POST-010**: `ActionReplayTracker::is_resolved` returns `true` iff the `(action, step)` pair was previously marked completed or failed; `false` otherwise

---

## Invariants

- **INV-001**: Every `RecoveryError` variant is semantically distinct and maps to exactly one failure mode in the recovery domain
- **INV-002**: `UnsupportedRecoveryState::SUPPORTED` has all four boolean fields as `false`; `union` never creates contradictory state
- **INV-003**: `RecoveryFrameSeed.step_count > 0` and `RecoveryFrameSeed.slot_count > 0` when events are non-empty and replay succeeds
- **INV-004**: `ActionReplayTracker` is monotonically sealed: once `(action, step)` is marked completed or failed, `is_resolved` always returns `true`
- **INV-005**: `DigestCheck` variants form a strict hierarchy: `WorkflowSourceOnly ⊂ WorkflowAndIr ⊂ Full` (each level adds one more digest check)
- **INV-006**: `recover_all_incomplete_runs` never includes a run whose latest attempt has a terminal event (RunFinished, RunCancelled, RunFailedEvent)

---

## Error Taxonomy

| Error Variant | When Raised | Semantic |
|---|---|---|
| `RecoveryError::Journal` | Journal operation fails during recovery | Journal infrastructure error |
| `RecoveryError::WorkflowSourceDigestMismatch` | Stored workflow digest ≠ expected | Trust verification failure |
| `RecoveryError::CompiledIrDigestMismatch` | Stored IR digest ≠ expected | Trust verification failure |
| `RecoveryError::ActionAbiMismatch` | Action ABI digest mismatch (GAP-3: not reachable) | ABI verification failure |
| `RecoveryError::PolicyDigestMismatch` | Policy digest mismatch (GAP-3: not reachable) | Policy verification failure |
| `RecoveryError::NonIdempotentActionBlocked` | Action already completed/failed encountered during replay | Non-idempotent re-execution guard |
| `RecoveryError::ReplayDivergence` | Step ordering violation, slot write failure, dimension overflow | Deterministic replay broken |
| `RecoveryError::NoRecoveryData` | No events found for run | Empty journal / missing data |
| `RecoveryError::CorruptSnapshot` | Snapshot undecodable or run_id mismatch | Snapshot integrity failure |
| `RecoveryError::TerminalStateMismatch` | Recovered terminal ≠ expected (deferred: no public API parameter) | Terminal state verification failure |
| `RecoveryError::FrameDimensionOverflow` | Derived dimensions exceed u16 | Model capacity overflow |

---

## Contract Signatures

```rust
// vb_storage::recovery::recover
pub fn check_workflow_source_digest(journal: &FjallJournal, run: RunId, expected: WorkflowDigest) -> RecoveryResult<()>;
pub fn check_compiled_ir_digest(expected: WorkflowDigest, found: WorkflowDigest) -> RecoveryResult<()>;
pub fn verify_digests(journal: &FjallJournal, run: RunId, workflow_digest: WorkflowDigest, ir_digest: WorkflowDigest, found_ir_digest: WorkflowDigest, level: DigestCheck) -> RecoveryResult<()>;
pub fn recover_runtime_summary(journal: &FjallJournal, run: RunId) -> RecoveryResult<RecoveryHydration>;
pub fn recover_runtime_frame_seed(journal: &FjallJournal, run: RunId) -> RecoveryResult<RecoveryFrameSeed>;
pub fn recover_run_admission(journal: &FjallJournal, run: RunId) -> RecoveryResult<Option<RecoveredRunAdmission>>;
pub fn recover_all_incomplete_runs(journal: &FjallJournal) -> RecoveryResult<Vec<RecoveryHydration>>;

// vb_storage::recovery::hydrate
pub fn hydrate_run_frame(snapshot: &RunSnapshot, tail_events: &[JournalEvent], run_id: RunId) -> RecoveryResult<vb_core::RunFrame>;
pub fn hydrate_run_frame_from_events(events: &[JournalEvent], run_id: RunId) -> RecoveryResult<vb_core::RunFrame>;

// vb_storage::recovery::replay::core
pub fn replay_events(events: &[JournalEvent], tracker: &mut ActionReplayTracker, expected_action_abi_digests: &[(ActionId, WorkflowDigest)]) -> RecoveryResult<Vec<JournalEvent>>;
pub fn recover_full_journal(journal: &FjallJournal, run: RunId, tracker: &mut ActionReplayTracker, expected_action_abi_digests: &[(ActionId, WorkflowDigest)], expected_policy_digests: &[(StepIdx, WorkflowDigest)]) -> RecoveryResult<Vec<JournalEvent>>;
pub fn recover_snapshot_plus_tail(snapshot: &RunSnapshot, tail_events: &[JournalEvent], tracker: &mut ActionReplayTracker) -> RecoveryResult<Vec<JournalEvent>>;
pub fn load_snapshot(journal: &FjallJournal, run: RunId, seq: EventSeq) -> RecoveryResult<RunSnapshot>;
pub fn is_terminal_event(event: &JournalEvent) -> bool;
pub fn extract_terminal(events: &[JournalEvent]) -> Option<&JournalEvent>;
```

---

## Verus-Owned Clauses

- **INV-002**: `UnsupportedRecoveryState::union` is commutative, associative, idempotent, and never produces contradictory state — Verus `proof fn union_preserves_invariants`
- **INV-004**: `ActionReplayTracker::is_resolved` is monotonic — Verus `proof fn tracker_monotonic`
- **INV-005**: `DigestCheck` hierarchy strictness — Verus `proof fn digest_check_hierarchy`
- **PRE-001 / PRE-002 / PRE-003 / PRE-005**: Dimension bound checks (`step_count > 0`, `slot_count > 0`, `seq` ordering) — Verus `requires` clauses on `hydrate_run_frame`, `hydrate_run_frame_from_events`, `replay_events`
- **POST-004 / POST-005**: `RecoveryFrameSeed` field invariants from events — Verus `spec fn` model of seed construction
- **POST-009**: `replay_events` attempt filtering and ordering — Verus `proof fn replay_preserves_attempt_invariant`

---

## TLA+-Owned Clauses

- **TLA-001**: Journal replay event ordering — `specs/RecoveryReplay.tla` — invariant: `ReplaySeqOrder` (events replayed in ascending seq; steps monotonic increasing per attempt)
- **TLA-002**: Snapshot-plus-tail causal consistency — `specs/RecoveryReplay.tla` — invariant: `TailCausalAfterSnapshot` (all tail seq > snapshot seq)
- **TLA-003**: Incomplete run discovery — `specs/RecoveryReplay.tla` — invariant: `OnlyIncompleteRuns` (only runs without terminal event of max attempt are returned)
- **TLA-004**: Non-idempotent action blocking — `specs/RecoveryReplay.tla` — invariant: `NoResolvedReExecution` (resolved action+step never appears in replay output)
- **TLA-005**: RecoveryError state machine exhaustiveness — `specs/RecoveryReplay.tla` — every error variant reachable from defined inputs
- **TLA-006**: Digest verification stage ordering — `specs/RecoveryReplay.tla` — workflow digest verified before IR digest

---

## Theorem-Owned Clauses

- **THM-001**: `UnsupportedRecoveryState::union` algebraic properties (commutativity, associativity, idempotency, no contradiction) — this is a tiny kernel, lean-contract.md projects it if needed, but Verus proof fn is primary
- No Lean/Aeneas/Hax required for this bead — Verus is sufficient for all Rust-local pure obligations

---

## Non-goals

- ActionAbiMismatch and PolicyDigestMismatch runtime lookup (GAP-3, deferred to vb-ty9)
- TerminalStateMismatch public API parameter (deferred DEFERRED_GLOBAL)
- Fjall journal internal consistency (covered by storage integration tests)
- Async scheduling, I/O, networking surfaces

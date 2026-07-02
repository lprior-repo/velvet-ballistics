# Contract Specification

## Context
- Bead: `vb-qi37.1.6` - runtime/recovery crash restart integration evidence.
- Source of truth read: `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.1.6 --json`, plus State2 `codebase-map.md` and `delivery-scope.jsonl`.
- Feature: prove that durable runtime recovery can restart from mid-run crash cuts using persisted run headers, journal events, snapshots, live-frame hydration, waits, asks, actions, retries, and collect pagination state.
- Domain terms: run header, journal event sequence, snapshot base, tail events, latest attempt, RunFrame, slot value, taint, wait, ask, action ticket, collect pagination extra, terminal event, typed recovery error.

## Assumptions
- This bead is evidence-first: production behavior may be changed later only if tests expose a real acceptance gap.
- Crate-level integration evidence is acceptable because State2 did not confirm a stable public restart CLI.
- `RunResumed`, `RunRetried`, and `RunAnswered` lifecycle events are not ordered recovery facts unless a later implementation gives them ordered journal sequence semantics.
- Pending action live-frame hydration is allowed to fail closed when exact idempotent continuation cannot be reconstructed.

## Open questions
- Does acceptance require a command-level restart path, or is the `vb_storage` plus `vb_runtime` integration boundary sufficient?
- Should `SlotWrittenEvent` gain an explicit durable taint field, or is taint derivation from slot value and `extra` the intended contract?
- Should lifecycle events become sequenced recovery facts, or remain diagnostic-only events outside live-frame recovery?

## Preconditions
- PRE-001: A recoverable run has a persisted run header with matching run id, workflow source digest, compiled IR digest, and accepted artifact digest before restart recovery begins.
- PRE-002: Recovery input events for a run are read in durable journal order with strictly increasing sequence numbers inside each replay segment.
- PRE-003: Snapshot-plus-tail recovery receives a snapshot whose run id matches the target run and tail events whose sequence numbers are strictly greater than the snapshot sequence watermark.
- PRE-004: Every slot required for exact live-frame hydration has a durable slot value and enough durable metadata to reconstruct taint and collect pagination extra.
- PRE-005: Wait, ask, retry, action, and collect restart evidence uses persisted journal/snapshot state only; no in-memory-only state may satisfy acceptance.
- PRE-006: Recovery callers treat unsupported or incomplete state as fallible and consume `Result<_, RecoveryError>` or `Result<_, RuntimeError>` rather than constructing a partial successful frame.

## Postconditions
- POST-001: Restart from persisted header/admission reconstructs the target run identity and never fabricates an empty successful frame for a non-empty run.
- POST-002: Full-journal replay reconstructs pc, step states, executed counts, slot values, slot taint, terminal status, and latest-attempt state exactly from durable events.
- POST-003: Snapshot-plus-tail replay preserves snapshot facts and applies only tail events after the snapshot watermark, with no tail-before-snapshot or sequence-gap acceptance.
- POST-004: Wait recovery preserves the waiting state and resumes only from the durable wait event identity.
- POST-005: Ask recovery preserves the asking state, answer slot value, and answer taint across restart.
- POST-006: Action recovery preserves pending/resolved action ticket identity; resolved tickets are not re-executed, and non-idempotent or unsupported pending actions fail closed.
- POST-007: Collect pagination recovery preserves cursor, current page, ordering, and identity from durable `SlotWrittenEvent.extra`; corrupt or wrong-identity extra returns a typed error.
- POST-008: Digest mismatch, corrupt snapshot, missing recovery data, corrupt slot value, corrupt collect extra, sequence gap, tail-before-snapshot, and unsupported live-frame state each return a precise typed failure.

## Invariants
- INV-001: Recovery never reports success with less durable state than the corresponding live frame requires.
- INV-002: Replaying the same persisted journal and snapshot twice yields equivalent recovery summaries and frame seeds.
- INV-003: Latest-attempt filtering never mixes stale attempt terminal state, slot state, waits, asks, or actions into the active attempt.
- INV-004: Snapshot facts plus tail facts are monotonic: tail events may advance state but may not erase persisted slot value, taint, ticket, wait, ask, or collect facts without an ordered replacing event.
- INV-005: Taint is exact: a recovered secret slot remains secret; missing durable taint evidence cannot silently default a required secret fact to clean.
- INV-006: Recovery failure is fail-closed: unsupported pending actions, missing event sets, corrupt encodings, digest mismatch, and unsupported variants cannot produce runnable state.
- INV-007: Journal event sequence is the temporal authority for recovery ordering; unordered lifecycle diagnostics cannot alter recovered state.

## Error Taxonomy
- `RecoveryError::NoRecoveryData` - persisted header exists but no usable recovery events/snapshot facts exist.
- `RecoveryError::CorruptSnapshot` - snapshot bytes, run id, dimensions, slot payload, or snapshot metadata cannot be trusted.
- `RecoveryError::ReplayDivergence` - event order, step state, latest-attempt, sequence gap, or tail watermark rules are violated.
- `RecoveryError::WorkflowSourceDigestMismatch` - recovered header/source digest does not match requested workflow source digest.
- `RecoveryError::CompiledIrDigestMismatch` - recovered header/compiled artifact digest does not match requested compiled IR digest.
- `RecoveryError::NonIdempotentActionBlocked` - pending action cannot be replayed without possible duplicate effects.
- `RecoveryError::FrameDimensionOverflow` - recovered dimensions exceed representable or configured frame bounds.
- `RuntimeError::InvalidRecoveryHydration` - storage hydration is incomplete or unsupported for a runnable `RunFrame`.
- `EngineError::CollectExtraHydrationFailed` - collect extra is missing, corrupt, or bound to the wrong collect identity.

## Contract Signatures
- `fn recover_runtime_summary(journal: &FjallJournal, run_id: RunId, expected_digests: ExpectedDigests) -> Result<RecoveryRuntimeSummary, RecoveryError>`
- `fn recover_runtime_frame_seed(journal: &FjallJournal, run_id: RunId, workflow: &CompiledWorkflow, expected_digests: ExpectedDigests) -> Result<RecoveryFrameSeed, RecoveryError>`
- `fn recover_full_journal(events: &[JournalEvent], expected_digests: ExpectedDigests) -> Result<RecoveryHydration, RecoveryError>`
- `fn recover_snapshot_plus_tail(snapshot: RunSnapshot, tail: &[JournalEvent], expected_digests: ExpectedDigests) -> Result<RecoveryHydration, RecoveryError>`
- `fn hydrate_run_frame(hydration: RecoveryHydration, workflow: &CompiledWorkflow) -> Result<RunFrame, RecoveryError>`
- `fn recovery_boundary_from_hydration(hydration: RecoveryHydration, workflow: &CompiledWorkflow) -> Result<RuntimeRecoveryBoundary, RuntimeError>`
- `fn hydrate_collect_state(events: &[JournalEvent], collect_identity: CollectIdentity) -> Result<CollectStates, EngineError>`

## TLA+-Owned Clauses
- PRE-002, PRE-003, POST-002, POST-003, POST-004, POST-005, POST-006, POST-007, INV-002, INV-003, INV-004, INV-007.

## Verus-Owned Clauses
- PRE-004, PRE-006, POST-001, POST-008, INV-001, INV-005, INV-006, plus bounded dimension, fallible recovery-boundary construction, and monotonic summary/frame-seed construction.

## Theorem-Owned Clauses
- None for State3. The recovery state machine and data invariants are expected to be expressible through TLA+ plus Verus; Lean/Aeneas/Hax is a non-goal unless State4 discovers a tiny algebraic kernel beyond Verus.

## Non-goals
- YAML parse/compile recovery chain; covered by dependent `vb-core-yaml-e2e-chain`.
- UI or Makepad recovery evidence.
- Performance improvement claims. This bead requires no speedup; any benchmark is regression evidence only.
- Writing implementation, tests, or proof/model code in State3.

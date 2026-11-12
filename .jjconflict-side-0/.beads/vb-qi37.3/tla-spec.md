# TLA+ Temporal Model Plan: collect pagination recovery/resume

## Boundary
- Temporal behavior in scope: `CollectStart -> CollectPage/body -> wait/ask/suspend -> recovery/replay -> resume -> CollectNext* -> CollectFinish`, including journal ordering, durable collect extras, page-order rejection, and per-run/per-node isolation.
- Rust core excluded from TLA+ and assigned to Verus/Kani/proptest: page slicing, cursor arithmetic, decode identity checks, typed error classification, and value-store bounds.
- External systems abstracted: Fjall durability is an append-only ordered event log; wall clock is a bounded monotonic elapsed-time variable; value store is an abstract map from `ListId` to finite sequences.
- Existing repo state: no collect-specific TLA module/config was found under `specs/tla/`. Existing modules (`RecoveryReplay`, `JournalBeforeDispatch`, `BoundedAdmission`, `AttemptTracking`, `ShardOwnership`) do not cover collect cursor/source stability, collect-extra decode identity, or recovery resume exactness. This artifact records a temporary scoped TLA+ waiver rather than inventing model files.

## TLA+-Owned Clauses
- TLA-COLLECT-001 maps INV-001/INV-002/INV-004/INV-005: per-run/per-slot state isolation, page-order safety, cursor monotonicity, and source list/id item-count stability across wait/ask/replay/resume.
- TLA-COLLECT-002 maps POST-006/INV-006: every resumable active collect page has a durable slot write plus matching collect extra before resume/recovery can observe it.
- TLA-COLLECT-003 maps POST-007/INV-008: recovery hydrates frame and collect side table from the same event prefix and resumes without page loss or duplication.
- TLA-COLLECT-004 maps ERR-004/ERR-005/ERR-006: stale, duplicate, and out-of-order page completions are rejected and do not mutate state.
- TLA-COLLECT-005 maps INV-007: non-collect extras are never decoded as collect state.

## Model Shape (waived target; required future model shape)
- Module/model path: no executable collect-specific module exists in this workspace. Future target should be `specs/tla/CollectPagination.tla` plus `specs/tla/CollectPagination.cfg` or an equivalent exact path added by the proof implementer.
- Variables:
  - `runs`: finite set of run ids.
  - `slots`: finite set of collector slot ids.
  - `state`: partial function `(run, slot) -> CollectState`.
  - `sourceId`: `(run, slot) -> ListId` for active collect source identity.
  - `sourceCount`: `(run, slot) -> Nat` for active collect source item count.
  - `framePage`: `(run, slot) -> PageId` for slots containing collect pages.
  - `journal`: sequence of events with `run`, `seq`, `slot`, `value`, `extraKind`, `extraState`.
  - `pc`: run program counter abstraction: `Start`, `Body`, `Waiting`, `Asking`, `Next`, `Done`, `Failed`.
  - `recoveredFrame` and `recoveredState`: hydration outputs.
  - `errors`: sequence of typed error observations.
- Init action: `InitNoCollectState` with empty state, empty journal, valid finite source lists, and accepted resource bounds.
- Actions:
  - `CollectStartNonEmpty`, `CollectStartEmptyOrFinal`, `CollectPageBody`, `SuspendWaitOrAsk`, `AskExternal`, `ReplayJournalPrefix`, `JournalSlotWithCollectExtra`, `JournalSlotWithNonCollectExtra`, `RecoverFrame`, `HydrateCollectState`, `ResumeCollect`, `CollectNextValid`, `RejectDuplicatePage`, `RejectStalePage`, `RejectOutOfOrderPage`, `CollectFinish`, `CapacityFailure`.
- State constraints:
  - `Cardinality(runs) <= 2`, `Cardinality(slots) <= 2`, `MaxItems <= 5`, `MaxPages <= 5`, `MaxSeq <= 12` for TLC exploration.
  - All lists finite; all cursors bounded by `MaxItems`; journal seq strictly increases per run.
- Symmetry sets: runs and collector slots can be symmetric when no fixed test identity is needed.

## Properties
- Safety invariants:
  - `NoCrossRunStateRead`: actions for run `r` never read/write/remove state for `r2 # r`.
  - `NoCrossSlotStateRead`: actions for slot `s` never read/write/remove state for `s2 # s`.
  - `CurrentPageMatches`: valid next transition requires `framePage[r,s] = state[r,s].current_page`.
  - `CursorMonotonicBounded`: cursor is monotone and never exceeds item_count/limit.
  - `SourceStableAcrossSuspendReplayResume`: active collect source list id and source item count are unchanged by wait, ask, replay, recovery hydration, and resume.
  - `DurableExtraBeforeRecoverableResume`: if a run can resume mid-collect after recovery, journal contains matching collect extra for current page.
  - `NonCollectExtraNotDecoded`: journal entries tagged or proven non-collect do not enter `HydrateCollectState`.
  - `RejectsDoNotMutateState`: duplicate/stale/out-of-order actions append typed error but leave `state` unchanged.
- Liveness/eventuality:
  - Under weak fairness for valid `CollectNextValid`, every active finite collect eventually reaches `Done` or a typed terminal failure (`CollectTimeLimitExceeded` or capacity/recovery failure).
  - Under recovery fairness, every durable active collect state is eventually hydrated before resume.
- Fairness assumptions:
  - Weak fairness on `RecoverFrame`, `HydrateCollectState`, and `CollectNextValid` when enabled.
  - No fairness on invalid duplicate/stale/out-of-order completions; they may occur adversarially but must not mutate state.
- Deadlock freedom: the model must have no deadlock except terminal `Done` or `Failed` states.
- Refinement to Rust/runtime behavior:
  - `CollectStates` entries refine `state[(RunId, SlotIdx)]`.
  - `RunFrame` collector slot list ids refine `framePage`.
  - `RuntimeJournalEvent::SlotWritten` and `JournalEvent::SlotWrittenEvent` refine `journal`.
  - `hydrate_run_frame_from_events` refines `RecoverFrame`; `hydrate_collect_states_from_recovered_journal` refines `HydrateCollectState` after collect-extra filtering.
  - `EngineError` variants refine entries in `errors`.

## Evidence Command
- No executable collect-specific TLA command exists in this workspace.
- Temporary release-scoped waiver command/evidence: `env VERIFY_BEAD_ID=vb-qi37.3 ALLOW_BEAD_LOCKBUD_WAIVER=1 bash scripts/rust-verification-gauntlet.sh all` must record the TLA waiver and all compensating collect runtime/storage commands as PASS or WAIVED.
- Future non-waived command shape: `tlc -config specs/tla/CollectPagination.cfg specs/tla/CollectPagination.tla` or an exact equivalent added by the proof implementer.

## Waivers
- TLA-WAIVER-COLLECT-001: collect pagination temporal model target is missing. Owner: State 6 implementer; approval owner: State 4 contract-verification reviewer. Expiry: before release-critical acceptance of `vb-qi37.3` or 2026-05-18, whichever comes first. Reason/limitation: existing TLA specs model recovery replay, journal-before-dispatch, admission, attempt tracking, and shard ownership, but not collect pagination source stability, cursor/page identity, collect-extra filtering, or recovery resume exactness. Compensating evidence: exact nextest commands for non-list source, time limit, duplicate/stale rejection/state preservation, recovery round trips, corrupt/identity mismatch rejection, source stability/time-limit storage; plus direct all-mode gauntlet `env VERIFY_BEAD_ID=vb-qi37.3 ALLOW_BEAD_LOCKBUD_WAIVER=1 bash scripts/rust-verification-gauntlet.sh all`. This waiver is temporary and must be retired or renewed with reviewer approval before release.

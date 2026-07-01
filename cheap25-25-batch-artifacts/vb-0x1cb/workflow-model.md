# Workflow Model — vb-0x1cb

- bead_id: vb-0x1cb
- phase: 3 (contract)
- attempt: 1-of-1
- captured_at: 2026-07-01T15:55:00Z
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
- source_checkout: /home/lewis/src/velvet-ballistics
- controller: femdation
- scope_kind: workflow_model
- lane_profile: rust_local_concurrency_empty
- status: contract drafted

This document captures the legal-state machine for the dual-failure shape introduced by the repair of `Shard::finish_run` and `Shard::fail_run_state`. It is intentionally narrow: this bead covers only the rollback branches (`transitions.rs:100` and `:202`), not the broader run lifecycle.

## 1. Workflow: `Shard::finish_run(run, state)`

### Pre-conditions (legal entry)

| Pre-condition | Source of truth |
|---------------|-----------------|
| `state.frame.executed()` matches the in-memory delta being committed. | `state.frame.executed()` field. |
| `Shard::counters.inc_completed()` has NOT yet been called for this `run`. | imperative fact. |
| `Shard::terminal_runs` does NOT yet contain `run`. | `terminal_runs: BTreeSet<RunId>`. |
| `Shard::trace_ring` does NOT yet contain `TraceEvent::RunFinished { run }` for this call. | observable fact via `trace_ring`. |

### States (legal while in-flight)

```
                                  ┌──────────────────────────────────────┐
                                  │  S0: Entry (pre-journal-append)      │
                                  │     terminal_runs.count == N0        │
                                  │     trace_ring.last != RunFinished{run}
                                  └────────────────┬─────────────────────┘
                                                   │
                       append_journal_event(RunFinished { run, result })
                                                   │
                       ┌───────────────────────────┴─────────────────────────┐
                       │                                                  │
                  Ok(_)|Err(p)                                        Err(p)         <-- primary
                       │                                                  │
                       ▼                                                  │
                ┌──────────────────┐                          ┌────────────▼────────────┐
                │  S1: Journal OK  │                          │  S2: Journal Failed     │
                │  (happy path)    │                          │  primary = p            │
                └────────┬─────────┘                          └────────────┬────────────┘
                         │                                                │
                         ▼                                                ▼
              (terminal fence mutation,                            observe_run_state_rollback(
               counters.inc_completed,                               run, state,
               release_frame,                                        site = FinishRun,
               discard_journal_sequence,                             primary = Arc::new(p))
               trace_ring.push(RunFinished{run}),                    │
               return Ok(()))                                        │
                                                                          ▼
                                                            ┌─────────────────────────────┐
                                                            │  S3: Rollback succeeded     │
                                                            │  trace_ring unchanged       │
                                                            │  return Err(p)              │
                                                            └─────────────────────────────┘
                                                                          │
                                                                          ▼   Err(s)
                                                            ┌─────────────────────────────┐
                                                            │  S4: Dual failed            │
                                                            │  trace_ring.last ==         │
                                                            │   RunRollbackFailed {       │
                                                            │     run, FinishRun,         │
                                                            │     primary = p,            │
                                                            │     secondary = s           │
                                                            │   }                         │
                                                            │  return Err(p)              │
                                                            └─────────────────────────────┘
```

### State transitions (legal only)

| From | Guard / Command | To | Notes |
|------|-----------------|----|-------|
| S0 | `append_journal_event(RunFinished { run, result }) → Ok(_)` | S1 | normal happy close. |
| S0 | `append_journal_event(RunFinished { run, result }) → Err(p)` | S2 | primary surfaced. |
| S2 | `observe_run_state_rollback(run, state, FinishRun, Arc::new(p)) → RollbackRecovered` | S3 | recovered: primary is the only surfaced error. |
| S2 | `observe_run_state_rollback(run, state, FinishRun, Arc::new(p)) → DualFailed { primary, secondary }` | S4 | dual failed: trace event pushed; `Err(p)` returned. |
| S1 | terminal fence mutation sequence (release_frame, discard_journal_sequence, etc.) | `→ return Ok(())` | exit; this bead does NOT cover happy-path after-effects. |
| S3 / S4 | `return Err(p)` (primary error returned to caller) | exit | terminal. |

### Terminal states (legal exit)

- **`Ok(())`** — only reachable from `S1`. Frame released; `terminal_runs ⊇ {run}`; `trace_ring.last == RunFinished { run }`.
- **`Err(primary)`** — reachable from `S3` and `S4`. `runtime_states` invariant depends on the rollback site (`finish_run` does NOT call `runtime_state_remove`; pre-call state must be re-asserted by the rollback).

### Forbidden transitions

- **S0 → S1 → Drop-counter-before-journal**: counters MUST NOT be incremented before `append_journal_event` succeeds. (`counters.inc_completed` lives at `transitions.rs:105`, AFTER the journal append branch — preserved by the existing ordering.)
- **S2 → suppressed-secondary**: `S2` → `S3`/`S4` MUST go through `observe_run_state_rollback`. Direct `let _ = self.run_state_insert(...)` calls are forbidden.
- **S3 → silent-secondary**: even when the rollback recovers, `observe_run_state_rollback` MUST be invoked (so the helper carries the trace-ring push); dropping back to a direct call would silence the future dual-failure record.
- **S4 → masking**: S4 MUST return `Err(p)`, never `Err(s)`. Verifying this is a black-hat-reviewer obligation on `contract.md` clause C-1.

## 2. Workflow: `Shard::fail_run_state(run, state)`

Identical shape to §1, with two changes:

- The durable event is `RuntimeJournalEvent::RunFailed { run }` (not `RunFinished`).
- The `RollbackSite` payload is `FailRunState` (not `FinishRun`).
- After `S1` (happy path) the function calls `runtime_state_remove(run)` and `trace_ring.push(TraceEvent::RunFailed { run })` at `transitions.rs:208,210` — these are preserved.

State graph for the dual-failure branches (mirror image):

```
S0 → append_journal_event(RunFailed { run })        ┬─ Ok(_) → S1 (happy path)
                                                  └─ Err(p)  → S2' (primary = p)
                                                                  │
                                                  observe_run_state_rollback(
                                                    run, state,
                                                    site = FailRunState,
                                                    primary = Arc::new(p))
                                                                  │
                                                  ┌───────────────┴──────────────┐
                                                  │                              │
                                            RollbackRecovered              DualFailed
                                                  │                              │
                                                  ▼                              ▼
                                                S3'                           S4'
                                          trace_ring unchanged         trace_ring.last ==
                                                                       RunRollbackFailed {
                                                                         run, FailRunState,
                                                                         primary = p,
                                                                         secondary = s
                                                                       }
                                          return Err(p)                return Err(p)
```

This dual shape is **identical** to `finish_run` in observable terms; the only difference is the durable event semantic (failed vs. finished) and the `RollbackSite` payload used for diagnostics. The contract therefore treats both sites uniformly — they share `observe_run_state_rollback`.

## 3. Idempotence & cancellation

### Idempotence

- `Shard::finish_run` and `Shard::fail_run_state` are **not** idempotent at the API surface. Two successful calls on the same `(run, state)` would double-increment `counters`, double-push `trace_ring`, and double-insert `terminal_runs`. The terminal fence pre-conditions (§1 Pre-conditions) make double invocation unreachable at the current call sites.
- `TraceEvent::RunRollbackFailed { .. }` IS push-safe: if the same `ObservedRollbackOutcome::DualFailed` is observed twice in independent rollback branches (e.g. one retry), the trace ring contains two distinct events with distinct `Arc`-cloned errors.

### Cancellation / shutdown

- The repair does NOT change cancellation/ shutdown semantics. `ShutdownInProgress` is a pre-existing `RuntimeError` variant for `apply` to surface, not for `finish_run` / `fail_run_state`.
- `trace_ring.push` is allocation-fallible at the underlying ring level; if the trace ring is full, push DOES NOT panic (per the existing `trace_ring` contract) but instead silently drops. The dual-failure event MAY be lost under a saturated ring — this is an inherited limit, **not** a regression from this bead. The contract forbids a NEW saturation risk by ensuring `Arc::clone` allocation is bounded (I5).

## 4. Out-of-scope transitions

The bead explicitly does NOT cover:

| Transition | Why |
|------------|-----|
| `Shard::apply(SUBMIT)` | already clean (no DISCARD-006 row) |
| `Shard::keep_run` | already clean (uses `?`) |
| `Shard::await_action` line 146 rollback | uses `?` already; not on the allow list |
| `Shard::await_timer` line 180 rollback | uses `?` already; not on the allow list |
| `Shard::finish_run` / `Shard::fail_run_state` happy path | unchanged; this contract covers only the rollback branches |

## 5. Cross-references

- `domain-model.md` — entities, invariants, forbidden states.
- `type-contracts.md` — `Observe_RunStateRollback` and `ObservedRollbackOutcome` types.
- `error-taxonomy.md` — `RuntimeError` and `TraceEvent` variants referenced.
- `hazard-analysis.md` — temporal / observability hazards.
- `contract.md` — clauses C-1 through C-7.

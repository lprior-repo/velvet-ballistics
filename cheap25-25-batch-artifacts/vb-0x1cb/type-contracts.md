# Type Contract — vb-0x1cb

- bead_id: vb-0x1cb
- phase: 3 (contract)
- attempt: 1-of-1
- captured_at: 2026-07-01T15:55:00Z
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
- source_checkout: /home/lewis/src/velvet-ballistics
- controller: femdation
- scope_kind: type_contracts
- lane_profile: rust_local_concurrency_empty
- status: contract drafted

This document defines the type-level contract for the secondary-error surface introduced by `vb-0x1cb`. It pins newtypes, smart constructors, parsers, and the typestate that the repair at `transitions.rs:100` and `:202` MUST satisfy. No `Result`-discrimination is exposed to callers; observability flows through `TraceEvent`.

## 1. New types

### 1.1 `RollbackSite` (newtype enum, `non_exhaustive`)

```rust
//! Identifies the rollback site that produced a `RunRollbackFailed`
//! trace event. Used by diagnostics, log redaction, and proof
//! harnesses to scope failures without stringly branching on
//! source-line ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RollbackSite {
    /// Source: `Shard::finish_run` (`crates/vb_runtime/src/shard/transitions.rs`).
    FinishRun,
    /// Source: `Shard::fail_run_state` (`crates/vb_runtime/src/shard/transitions.rs`).
    FailRunState,
}
```

- **Justification**: a typed `enum` (not `&'static str`) removes the boolean-behavior-flag drift risk and proof-ladder stringly heuristics. `non_exhaustive` so future rollback additions (e.g. `await_action` line 146) don't ripple into matching sites elsewhere.
- **Smart constructor**: none — discriminant is the value, no parse step.
- **Property**: each variant is `Copy`, `Eq`, `Hash`. A `BTreeSet<RollbackSite>` can group traces per shard.

### 1.2 `TraceEvent::RunRollbackFailed` (new `#[non_exhaustive]` variant)

```rust
//! ... inside `pub enum TraceEvent` (already `#[non_exhaustive]`) ...
RunRollbackFailed {
    /// Run identifier whose rollback path failed.
    run: RunId,
    /// Which rollback produced this event.
    site: RollbackSite,
    /// The primary error from `append_journal_event`.
    /// Carried by `Arc` to keep `TraceEvent` size bounded.
    primary: std::sync::Arc<RuntimeError>,
    /// The secondary error from `run_state_insert`.
    /// Carried by `Arc` for the same reason.
    secondary: std::sync::Arc<RuntimeError>,
}
```

- **Justification**:
  - `Arc<RuntimeError>` is what `RuntimeError::StorageJournalAppend { source }` and `RuntimeError::AdmissionHeaderPersistenceFailed { source }` already use; matching that pattern avoids introducing a new `Box`-heap per trace.
  - `RunId` is `Copy` so no `Arc` needed.
  - `RollbackSite` is `Copy + Eq` so partial-match discriminators are exhaustive.
  - **Invariant on field order**: `primary` precedes `secondary` to mirror the temporal ordering of the dual failure (primary lands first in time, secondary is observed after the rollback), aiding debugging UX and proptest ordering.

### 1.3 Helper struct: `ObservedRollbackOutcome`

```rust
//! Outcome of a best-effort rollback. Pure data; not allocated on
//! the trace ring — emitted iff both journal append and rollback
//! fail in the same call.
#[derive(Debug, PartialEq, Eq)]
pub enum ObservedRollbackOutcome {
    /// Rollback succeeded; the in-memory state mirrors the journal.
    /// (Only the primary error is surfaced via `Result::Err`.)
    RollbackRecovered,
    /// Both journal append and rollback failed.
    /// The caller MUST see the primary error in `Err`; this outcome
    /// is recorded as `TraceEvent::RunRollbackFailed`.
    DualFailed {
        primary: std::sync::Arc<RuntimeError>,
        secondary: std::sync::Arc<RuntimeError>,
    },
}
```

- **Justification**: gives the implementation a single returned shape from the rollback helper (see §3) without leaking `Arc` allocation to the caller. This is the *railway-style* return that distinguishes recovered vs. dual-failed at the boundary of the pure core.

## 2. Refined existing types

The repair does NOT add a new `RuntimeError` variant. It does refine one item:

### 2.1 `Shard::finish_run` / `Shard::fail_run_state` return type remains `RuntimeResult<()>`

The repair maintains the existing return type. The semantic of `Err(_)` is unchanged: the caller sees the primary error. The secondary is bound into a `TraceEvent` instead of being dropped. **No new `RuntimeError` variant is required.** This refines:

```rust
// Before (DISCARD-006):
let _ = self.run_state_insert(run, state);  // dropped
return Err(error);

// After (typed-and-bound):
let outcome = self.observe_run_state_rollback(run, state, site);
if matches!(outcome, ObservedRollbackOutcome::DualFailed { .. }) {
    // outcome is already pushed to the trace ring inside the helper.
}
return Err(error);
```

### 2.2 `Shard::observe_run_state_rollback` (new inherent method)

```rust
impl Shard {
    /// Wraps `run_state_insert` so that the secondary `RuntimeError` is
    /// always bound into an `ObservedRollbackOutcome` and any dual
    /// failure is pushed as `TraceEvent::RunRollbackFailed`.
    ///
    /// Pre-conditions:
    /// - `site` MUST be `RollbackSite::FinishRun` when called from
    ///   `Shard::finish_run`, `RollbackSite::FailRunState` when called
    ///   from `Shard::fail_run_state`. Misuse is a value-object
    ///   identity leak; the contract pins it as a `#[track_caller]` +
    ///   `debug_assert!(matches!(site, …))` precondition.
    /// - `run` MUST be the run whose `append_journal_event` just failed
    ///   at the call site.
    ///
    /// Post-conditions:
    /// - `ObservedRollbackOutcome::RollbackRecovered` ⇒ the rollback
    ///   `run_state_insert` returned `Ok(_)`; the prior in-memory run
    ///   state has been restored.
    /// - `ObservedRollbackOutcome::DualFailed { primary, secondary }`
    ///   ⇒ both journal append and rollback returned `Err(_)`. The
    ///   `TraceEvent::RunRollbackFailed { run, site, primary, secondary }`
    ///   has already been pushed to `self.trace_ring`.
    #[must_use]
    pub(crate) fn observe_run_state_rollback(
        &mut self,
        run: RunId,
        state: RunState,
        site: RollbackSite,
        primary: std::sync::Arc<RuntimeError>,
    ) -> ObservedRollbackOutcome {
        match self.run_state_insert(run, state) {
            Ok(_) => ObservedRollbackOutcome::RollbackRecovered,
            Err(secondary) => {
                let secondary = std::sync::Arc::new(secondary);
                self.trace_ring
                    .push(TraceEvent::RunRollbackFailed {
                        run,
                        site,
                        primary,
                        secondary: std::sync::Arc::clone(&secondary),
                    });
                ObservedRollbackOutcome::DualFailed { primary, secondary }
            }
        }
    }
}
```

- **Justification**: a single chokepoint for both rollback sites (`transitions.rs:100` and `:202`). This removes the duplicate `let _ = ...` shape and concentrates the contract in one method.
- **Typestate**: `observe_run_state_rollback` accepts a `primary: Arc<RuntimeError>` so the helper is type-driven (the primary is required, not optional). It is `#[must_use]` because dropping the outcome would itself be a DISCARD-006 hazard on `ObservedRollbackOutcome::DualFailed`.

### 2.3 Equality / Display / Debug completeness

| Type | `Debug` | `Display` | `PartialEq` | `Eq` | `Hash` | `Clone` |
|------|--------|-----------|-------------|------|--------|---------|
| `RollbackSite` | derive | manual `&'static str` form | derive | derive | derive | derive |
| `TraceEvent::RunRollbackFailed { .. }` | derive (per enum) | derive (per enum) | derive (per enum) | derive (per enum) | — | derive (per enum) |
| `ObservedRollbackOutcome` | derive | manual | derive | derive | — | manual (`Arc::clone`) |

`TraceEvent`'s existing `Debug + Clone + PartialEq + Eq` derive on the enum adds these impls to the new variant automatically because `Arc<T>` is `PartialEq + Eq + Hash` only via `PartialEq` for `Arc<T>: PartialEq` (`Arc<T>: Eq` when `T: Eq + ?Sized` already via `Arc`); `RuntimeError` already implements `PartialEq + Eq` via `error/equality.rs`. The new variant rides on those existing impls.

## 3. Parser / Boundary position

`RollbackSite` is constructed only inside `transitions.rs:100` and `:202` (and the future `:146` if a third site is added). It is NOT parsed at any IO/parser boundary. There is no `From<&str, TryFrom<&str, FromStr>` impl because there is no external input that names a rollback site.

`Arc<RuntimeError>` is constructed only by:
- existing call sites in `error/conversions.rs` (`From<vb_storage::JournalError> for RuntimeError` → `StorageJournalAppend { source: Arc<…> }`), and
- the new `observe_run_state_rollback` (which wraps its `Err(secondary)` once).

No external boundary exists for this secondary wrap; `Arc::new` is reached only when the rollback itself fails.

## 4. Typestate summary

| State | Reachable | Proof obligation |
|-------|-----------|------------------|
| Primary journal-append returned `Err(p)` only (rollback succeeded) | Yes | `Err(p)` returned; no `RunRollbackFailed` event. |
| Primary journal-append returned `Err(p)` and rollback also returned `Err(s)` | Yes (rare but explicit) | `Err(p)` returned; `RunRollbackFailed { run, site, primary: Arc(p), secondary: Arc(s) }` pushed. |
| Primary journal-append returned `Ok(_)` | Yes (happy path) | Not on this code path; terminal fence mutation proceeds. |
| Rollback returned `Err(s)` while primary returned `Ok(_)` | **Unreachable** under this contract — `observe_run_state_rollback` is only invoked when the journal-append branch failed. | proof-writer must verify the unreachable-edge property via Flux/Verus refinement. |

## 5. Cross-references

- `domain-model.md` §Invariants — I1–I6.
- `workflow-model.md` — the dual-failure state machine.
- `error-taxonomy.md` — `RuntimeError` and `TraceEvent` variants used.
- `boundary-map.md` — the `observe_run_state_rollback` chokepoint.
- `contract.md` — the canonical surface statement.

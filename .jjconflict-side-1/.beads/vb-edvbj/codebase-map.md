# Codebase Map — vb-edvbj

- bead_id: vb-edvbj
- bead_title: Runtime: delete fallback that maps unmapped journal events to run failure (P0 bug)
- source_checkout: /home/lewis/src/velvet-ballistics
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-edvbj
- controller: femdation
- scope_lane: p2-explore (artifact-writing only)
- captured_at: 2026-07-01T15:22:00Z
- jj_root: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-edvbj
- jj_workspace_name: cheap25-vb-edvbj
- jj_parent_commit: rsvywymkwnqx (AGENTS.md round10 forward-port)
- baseline_branch: origin/main @ 2c8ea33c9

## Bead Summary (literal)

> Remove the catch-all fallback in journal event handling that silently maps unknown `RuntimeJournalEvent` variants to run failure. Locate the match arm in `crates/vb_runtime` that handles `_ =>` or wildcards for journal events and remove it.

This is a bug-hunt tracking bead for the previously diagnosed issue `vb-2gxqo / RE-019`
(see `to-fix/wave1/agent-01-holzman-rust-B.md`,
`to-fix/wave2/agent-01-holzman-rust-B.md`,
`to-fix/11-validation-wave-1.md`). The fix must:

1. Remove the silent `Ok(JournalEvent::RunFailedEvent { run, seq, attempt: 1 })` fabrication
   that runs when `run_storage_event` / `action_storage_event` / `boundary_storage_event`
   all return `None` for a given `RuntimeJournalEvent`.
2. Decide a replacement (typed error vs explicit mapping); see Open Questions.

## Bug Site (exact)

File: `crates/vb_runtime/src/journal/chunk_002.rs`

| Line   | Element | Role |
| ------ | ------- | ---- |
| 270-303 | `StorageRuntimeJournal::storage_event` | function signature + body |
| 274-294 | top-level dispatcher `match &event { ... }` | routes each variant to one of three helpers |
| 283-292 | explicit named-group arm | calls `run_storage_event` / `action_storage_event` |
| **293** | **`_ => Self::boundary_storage_event(...)`** | dispatcher wildcard (not the bug) |
| **295-302** | **BUGGY FALLBACK** | synthesises a `RunFailed` for every variant whose layer helper returns `None` |
| 343 | call site in `StorageRuntimeJournal::append_sequenced` | `let storage_event = Self::storage_event(event, seq)?;` |
| 12 | call site in `QueuedStorageRuntimeJournal::append_sequenced` | `let storage_event = StorageRuntimeJournal::storage_event(event, seq)?;` |

The wildcards in `run_storage_event` (lines 89-101), `action_storage_event` (175-189),
and `boundary_storage_event` (252-266) returning `None` are **NOT** bugs — they are correct
per-layer domain filters. The bug is the post-dispatch fallback at lines 295-302 that
synthesises a `RunFailedEvent` whenever all three helpers return `None`.

### Currently mis-handled (current `RuntimeJournalEvent` shape)

`Resumed { run, timestamp }` (defined at `crates/vb_runtime/src/journal/chunk_001.rs:189-194`):

- `run_storage_event` returns `None` (line 101)
- `action_storage_event` returns `None` (line 189)
- `boundary_storage_event` returns `Ok(None)` (line 266)
- falls through to bug site, becomes `JournalEvent::RunFailedEvent { run, seq, attempt: 1 }`

A successful resume therefore silently writes a fabricated run failure to Fjall when a
storage-backed journal is wired.

### Future variants

Any future addition to `RuntimeJournalEvent` that is not added to all three helpers will
silently become a fabricated `RunFailedEvent`. That is the original RE-019 forward-port hazard.

## Variant / Symbol Inventory

### `RuntimeJournalEvent` (declared at `crates/vb_runtime/src/journal/chunk_001.rs:15`)

21 variants (lines 17-194): RunSubmitted, RunAdmission, RunFinished, RunFailed, RunCancelled,
RunKilled, ActionScheduled, ActionCompleted, ActionScheduledTicket, ActionCompletedEnvelope,
ActionFailed, ActionAbandoned, WaitScheduled, WaitResolved, AskScheduled, AskAnswered,
AskTimedOut, SlotWritten, StepStarted, StepSucceeded, Resumed.

`#[derive(...)]` carries `serde::{Serialize, Deserialize}` so adding a new variant is a
breaking record-format change. `#[non_exhaustive]` blocks external exhaustive matches from
forcing compile-time updates.

### `RuntimeJournalEvent::run_id` accessor (`chunk_001.rs:200-224`)

Single match that already covers all variants. No code change required here.

### Layer helpers

| Helper | File / Line | Returns | Bug exposure |
| ------ | ----------- | ------- | ------------ |
| `run_storage_event` | `chunk_002.rs:41-103` | `Option<JournalEvent>` | indirectly via post-dispatch fallback |
| `action_storage_event` | `chunk_002.rs:105-191` | `Option<JournalEvent>` | indirectly via post-dispatch fallback |
| `boundary_storage_event` | `chunk_002.rs:193-268` | `RuntimeResult<Option<JournalEvent>>` | indirectly via post-dispatch fallback |
| `storage_event` (top-level dispatcher) | `chunk_002.rs:270-303` | `RuntimeResult<JournalEvent>` | holds the buggy fallback |

### `StorageRuntimeJournal::append_sequenced` (`chunk_002.rs:342-346`)

```rust
fn append_sequenced(&self, event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<()> {
    let storage_event = Self::storage_event(event, seq)?;
    self.append_storage_event(&storage_event)?;
    Ok(())
}
```

Propagates `?`. Any change to the `RuntimeResult<JournalEvent>` return type ripples into
`QueuedStorageRuntimeJournal::append_sequenced` (`chunk_003.rs:8-16`) and any downstream reporter.

### `QueuedStorageRuntimeJournal::append_sequenced` (`chunk_003.rs:8-16`)

```rust
fn append_sequenced(&self, event: RuntimeJournalEvent, seq: EventSeq) -> RuntimeResult<()> {
    if self.profile == DurabilityProfile::Strict {
        return Err(RuntimeError::UnsupportedAsyncStrictAck);
    }
    let storage_event = StorageRuntimeJournal::storage_event(event, seq)?;
    ...
}
```

`Strict` profile is gated by a separate `UnsupportedAsyncStrictAck` error.

### Call sites upstream of `storage_event`

| Caller | File / Line | Notes |
| ------ | ----------- | ----- |
| `RuntimeShard::append_journal_event` | `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:194-199` | wraps `journal.append_sequenced(event, seq)` and bumps sequence |
| `VolatileRuntimeJournal::append` | `crates/vb_runtime/src/journal/chunk_001.rs:281-288` | bypasses `storage_event` entirely; only used in Volatile profile tests/embeddings |
| Test mocks | `lifecycle_tests/chunk_004.rs:244,346,478`; `tests/chunk_015.rs:265` | mocks of `RuntimeJournal::append_sequenced` |

Note: `VolatileRuntimeJournal` does NOT exercise the bug because it has its own `append`
impl that pushes directly into a `Vec`. The bug is provoked only by `StorageRuntimeJournal`
or `QueuedStorageRuntimeJournal` profiles.

### `RuntimeError` (no `UnsupportedEvent` variant currently exists)

File: `crates/vb_runtime/src/error/mod.rs`. 51 variants enumerated. None of them is
`UnsupportedEvent`, `UnmappedRuntimeJournalEvent`, or similar. Adding a new variant requires:

1. variant declaration in `mod.rs`,
2. arm in `equality.rs::runtime_error_unit_tag` or `runtime_error_field_eq`,
3. arm in `display.rs::runtime_error_static_message` or `write_runtime_error_dynamic`
   (and `source()` in `display.rs::std::error::Error::source`),
4. `DiagnosticCode` constant + arm in `diagnostics.rs::diagnostic_code`,
5. arm in `diagnostics.rs::runtime_code`,
6. arm in `conversions.rs` only if `From` semantics apply.

Precedent for VB-NOORE pattern of typed-error-on-wildcard:
`RuntimeJournalConfig::shared_journal` (`chunk_001.rs:377-393`) returns
`Err(RuntimeError::UnsupportedDurabilityProfile { profile_debug: String })` when its
`_ =>` arm fires (`chunk_001.rs:388-391`). That is the mirror template the fix should follow.

## Existing Tests That Touch The Surface

### Tests that exercise the buggy path (none currently)

No test in the repository currently invokes `StorageRuntimeJournal::storage_event` with
`RuntimeJournalEvent::Resumed` (or any other variant that maps to `None` across all three
helpers). The three regression tests at
`crates/vb_runtime/src/journal/tests/chunk_002.rs:411-492`
(`storage_event_clones_the_event_exactly_once_per_dispatch`) only exercise `RunAdmission`,
`ActionScheduled`, and `SlotWritten` — none of which hits the bug.

### Tests that exercise the non-buggy `None` paths

`crates/vb_runtime/src/journal/tests/chunk_001.rs:188-260`
(`storage_runtime_journal_maps_cancelled_and_failed_events`) exercises the legitimate
`RunFailed -> JournalEvent::RunFailedEvent` mapping. Removing the fallback must preserve
this test (it goes through the explicit `RuntimeJournalEvent::RunFailed` arm in
`run_storage_event`, not through the fallback).

### Test inventory (in-scope verification)

| Test file | Status relative to bead |
| --------- | ----------------------- |
| `crates/vb_runtime/src/journal/tests/chunk_001.rs` | MUST continue to pass (`RunCancelled`, `RunFailed` mappings). |
| `crates/vb_runtime/src/journal/tests/chunk_002.rs` | MUST continue to pass. Several tests assert mappings for `RunSubmitted`, `RunFinished`, `RunCancelled`, `ActionScheduled`, `ActionCompleted`, all `Wait*`, all `Ask*`, `SlotWritten`, `RunAdmission`, `RunFailed`. None hits the fallback. |
| `crates/vb_runtime/src/journal/tests/chunk_003.rs` | MUST continue to pass. |
| `crates/vb_runtime/src/journal/tests/chunk_004.rs` | MUST continue to pass. |
| `crates/vb_runtime/tests/durable_resume_red_phase.rs` | Hits `Resumed` against `VolatileRuntimeJournal`, not storage. Bug is NOT visible there. |
| `crates/vb_runtime/tests/recovery_bdd_tests.rs` | Storage-driven flows; verify after fix. |
| `crates/vb_runtime/tests/recovery_hydration_tests.rs` | Storage-driven flows; verify after fix. |
| `crates/vb_runtime/tests/vb_h6ix_integration.rs` | Storage-driven flows; verify after fix. |
| `crates/vb_runtime/tests/vb_jggy_*.rs`, `vb_jggy_journal_event_tests.rs` | Codec/serde tests for `RuntimeJournalEvent`; should not be affected. |
| `crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs` | Volatile-only. |
| `crates/vb_runtime/tests/durability_matrix_integration.rs` | Storage-driven. |

### Test plans downstream (not yet authored)

- RE-019 regression test: must assert that `Resumed` round-trip through
  `StorageRuntimeJournal::append_sequenced` produces a `JournalEvent` mapped to a real
  variant (either the existing `JournalEvent::RunResumed` — see
  `crates/vb_storage/src/events.rs:290-297` — or a typed error), never a fabricated
  `RunFailedEvent`. Must assert the in-memory `VolatileRuntimeJournal` *and* the on-disk
  `events_for_run(run)` agree on the run's terminal-classification-free state.
- New test: dispatch an unknown / future variant to a typed error (when Option A is chosen)
  OR pin the explicit `Resumed -> RunResumed` mapping (when Option B is chosen).

## Open Questions for Downstream Agents

1. **Replacement strategy.** The bead is literal: "Locate the match arm that handles `_ =>`
   and remove it." If we delete the fallback arms only, we need a typed replacement so the
   function still resolves `RuntimeResult<JournalEvent>`. Two viable options:

   **Option A — typed error (matches VB-NOORE precedent for `UnsupportedDurabilityProfile`):**
   - Add `RuntimeError::UnmappedRuntimeJournalEvent { event_kind: &'static str }` (or
     similar name) to `crates/vb_runtime/src/error/mod.rs`.
   - Wire `equality.rs`, `display.rs`, `diagnostics.rs`, `conversions.rs` per the precedent
     of `UnsupportedDurabilityProfile` (see `to-fix/wave1/agent-01-holzman-rust-B.md:35`).
   - In `storage_event`, replace the synthetic fallback with
     `Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind: "..." })`.
   - Update the three layer helpers to return `RuntimeResult<Option<JournalEvent>>` OR keep
     `Option<JournalEvent>` and let `storage_event` introspect `event` via a `match` to
     determine the kind.

   **Option B — explicit mapping:**
   - Add `Resumed { run, timestamp }` arm to `boundary_storage_event` returning
     `Ok(Some(JournalEvent::RunResumed { run, seq, timestamp: DateTime::from_timestamp(timestamp, 0).single() }))`
     (or `Utc.timestamp_opt(timestamp, 0).single()` depending on chrono version).
   - Convert the dispatcher's wildcards to exhaustive alternations (no more `_ =>`).
   - Leave `RuntimeError::UnmappedRuntimeJournalEvent` as a backstop ONLY if a helper still
     returns `None`. (Optional.)

   The downstream contract/proof/test planners must pick one and encode it as the typed Rust
   contract.

2. **`RuntimeJournalEvent::Resumed` timestamp type.** `Resumed { timestamp: u64 }` (epoch
   seconds) vs. `JournalEvent::RunResumed { timestamp: DateTime<Utc> }`. Any Option B mapping
   must convert; this requires confirming which `chrono` API is available in the workspace
   (see `crates/vb_runtime/Cargo.toml:7` — `chrono.workspace = true`).

3. **Behavior expectation when `Resumed` flows through storage.** The Recovery semantics
   currently observe `JournalEvent::RunResumed` in
   `crates/vb_storage/src/recovery/replay/observation/normalize.rs:126-127` and
   `crates/vb_storage/src/journal/incident.rs:203`. The buggy fallback synthesises
   `JournalEvent::RunFailedEvent`, which mis-classifies the run as terminated. Today the
   recovery path may be silently re-classifying `Resumed` flows as failures.

4. **Strict profile path.** `QueuedStorageRuntimeJournal::append_sequenced` separately
   rejects the Strict profile with `Err(RuntimeError::UnsupportedAsyncStrictAck)`
   (`chunk_003.rs:9-11`). That gate is independent of the bug being fixed and must remain.

5. **Kani harnesses.** The `append_journal_event` function has a `#[cfg(kani)]` stub
   (`crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:201-211`) that always returns `Ok(())`.
   No Kani harness currently exercises `storage_event` directly. A future Kani harness would
   need `#[kani::proof_for_contract(StorageRuntimeJournal::storage_event)]` or similar —
   out of scope for vb-edvbj.

## Risk Tags

- `persistence`: high. Durable storage layer corrupts run state silently. Fix must preserve
  crash-consistency rules of master §49.
- `temporal`: medium. Resume-after-suspend is the user-visible behavior that fails today.
- `parser/codec`: medium. New typed-error variant (Option A) must follow the established
  `RuntimeError` contracts.
- `public API`: medium. `RuntimeError::UnmappedRuntimeJournalEvent` (Option A) is a new
  public variant. Existing callers must handle it via the `?` propagation already in place.
- `test-coverage`: high. No existing regression test detects the bug. Downstream MUST add a
  fresh regression test that pins `Resumed -> run-failure-fabrication` is impossible.
- `concurrency`: low. `storage_event` is synchronous and `&event`-borrowing; no shared state.
- `unsafe/UB`: none. Source is `#![forbid(unsafe_code)]`.
- `dependency`: low. No third-party change; only workspace crate `vb_runtime` touched.

## Excluded Paths

- `crates/vb_storage/**` is the receiving layer; no source change expected. If Option B is
  chosen, helpers may add a `chrono` DateTime conversion that imports from `vb_storage::JournalEvent`.
  `vb_storage` requires no edits.
- `crates/vb_cli/**`, `crates/vb_compile/**`, `crates/vb_core/**`, `crates/vb_ipc/**`,
  `crates/vb_queue_semantics/**`, `crates/vb_validate/**`, `crates/workspace_tests/**`,
  `fuzz/**`, `kani/**`, `verification/**`, `xtask/**`: out of scope.

## Open Sequencing Notes

The bug fix touches a single file (`crates/vb_runtime/src/journal/chunk_002.rs`) and (under
Option A) up to six files in `crates/vb_runtime/src/error/**`. The blast radius is small
enough that a single State 3 -> State 7 cycle is sufficient, pending rust-contract /
proof-plan / test-plan approval.

## DISCOVERY_BLOCKED

None. All scope inputs are present and grepable.

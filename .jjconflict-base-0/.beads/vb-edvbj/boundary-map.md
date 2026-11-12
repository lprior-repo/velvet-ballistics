# Boundary Map — vb-edvbj

## Functional core / imperative shell

| Layer | Boundary | Goes through | Notes |
| ----- | -------- | ------------ | ----- |
| **Functional core** | `run_storage_event`, `action_storage_event`, `boundary_storage_event` return pure `Option<JournalEvent>` / `RuntimeResult<Option<JournalEvent>>` from an event value. No side effects. | `StorageRuntimeJournal::storage_event` | Pure; safe for property-testing. Keep functional shape. |
| **Imperative shell** | `storage_event` consumes the variant + `seq`, dispatches, returns `RuntimeResult<JournalEvent>` to its imperative caller `append_storage_event` / `append_sequenced`. | same file | Side-effecting only at append time. |
| **Storage sink** | `append_storage_event` → `FjallJournal::append_journaled` / `append_strict` | `StorageRuntimeJournal` impl block, lines 32-39 | Stays untouched. |
| **Caller (upstream of `storage_event`)** | `StorageRuntimeJournal::append_sequenced` `?`-propagates, then `append_storage_event`. Two-line body; no edit required. | `chunk_002.rs:342-346` | `?` carries the new error verbatim. |
| **Sister-adapter caller** | `QueuedStorageRuntimeJournal::append_sequenced` has its own Strict-profile guard and then `?`-propagates the same `storage_event` call. | `chunk_003.rs:8-16` | `?` carries the new error verbatim. Strict guard stays. |
| **Shard caller** | `RuntimeShard::append_journal_event` wraps the journal adapter and bumps sequence. | `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:194-199` | Surfaces error to whatever called it (e.g. `handle_resume`, `handle_cancel`, etc.). |

## Boundary classifications

| Boundary | Kind | Affected by this bead? | Why |
| -------- | ---- | ---------------------- | --- |
| **Pure core** | `run_storage_event` / `action_storage_event` / `boundary_storage_event` | No body change. Signature preserved. | New error originates in imperative dispatcher only. |
| **Imperative shell (sync)** | `StorageRuntimeJournal::storage_event` | **Yes** — post-dispatch fallback replaced with typed error. | This is the fix. |
| **Imperative shell (sync, queue)** | `QueuedStorageRuntimeJournal::append_sequenced` | No body change. `?` picks up new error. | Strict guard retained. |
| **Async shell** | `tokio`-driven callers in shard / runtime | No. `storage_event` is sync; `?` is plain. | No await added. |
| **Storage** | `vb_storage::FjallJournal::{append_journaled, append_strict}` | No. | Bug does not reach storage; fix happens upstream. |
| **Network** | n/a — no IPC on this codepath | No. | — |
| **Time** | `Resumed { timestamp: u64 }` carries a monotonic timestamp | No edit (only the dispatch path changes; `Resumed` is unchanged). | `Resumed` timestamp type unchanged. |
| **FFI** | n/a | No. | Rust-only. |
| **Unsafe** | n/a — `#![forbid(unsafe_code)]` in this crate | No. | — |
| **Parser / codec** | `serde::{Serialize, Deserialize}` on `RuntimeJournalEvent` (chunk_001.rs:14). Add-variant is a breaking record format. | No edit. | Variant shape is preserved. `#[non_exhaustive]` is preserved. |
| **Display / Diagnostic** | `equality.rs`, `display.rs`, `diagnostics.rs` — see `error-taxonomy.md` | **Yes** — error variant edition is mandatory to wire the new variant through the equality / display / diagnostic pipeline. | New arm in three modules: `equality.rs::runtime_error_field_eq`, `display.rs::runtime_error_static_message`, `display.rs::write_runtime_error_dynamic`, `diagnostics.rs::diagnostic_code`, `diagnostics.rs::runtime_code`. |

## Async / sync cross-check

`storage_event` is a synchronous function — it returns `RuntimeResult<JournalEvent>`, takes its inputs by value, and never `.await`s. No async context switches are introduced.

## Storage cross-check

The fix removes one site of disk writes (the synthetic `RunFailedEvent`) but does not touch the storage layer directly. The `FjallJournal::append_*` calls remain the only disk-write surface; they receive only verified `JournalEvent` values.

## Display / Diagnostic cross-check

`Display` and `DiagnosticCode` paths are independent of the dispatch layer. The contract requires that the new variant is reachable from:
- `RuntimeError::eq` (PartialEq via `runtime_error_field_eq`),
- `RuntimeError::fmt` (Display via static-or-dynamic message path),
- `RuntimeError::diagnostic_code` (0x2020),
- `RuntimeError::runtime_code` (None),
- `RuntimeError::symbolic_code` (INTERNAL_INVARIANT),
- `RuntimeError::source` (None).

None of these require cross-crate wiring.

## Ownership / aliasing cross-check

The dispatcher matches `&event` (today) for first-dispatch, then `clone_for_dispatch(&event)` clones into the helper. The fix does not change this pattern. The new error path does not own or alias any external data — `event_kind: &'static str` is a literal reference.

## Hostile-input cross-check

`storage_event` does not accept external input directly. `RuntimeJournalEvent` values are constructed by `vb_runtime` itself in shard paths. The only externally-driven source is reading them back via `serde::Deserialize` (chunk_001.rs:14), and that boundary is orthogonal to the dispatch fix — additive variants in the on-disk format are still routed to `UnmappedRuntimeJournalEvent { event_kind: "<future_variant>" }` if not added to the helpers.

## Verus mirror binding (GOD RULE 2)

Reference mirror: `verification/verus/extern_storage_kind_family.rs`. The mirror already binds:

- `MirrorJournalEvent::RunResumed { run: MirrorRunId, seq: MirrorEventSeq }` (mirror of `crates/vb_storage/src/events.rs:290-297`),
- `MirrorRecordKind::RunResumed => 25` (mirror of `crates/vb_storage/src/records.rs`).

This mirror is preserved. No new `MirrorRuntimeJournalEvent` type is required for this bead because the fix is at the dispatcher, not the storage-record-shape layer. The bead's contract REQUIRES that any drift-detection helper in this mirror file remain compiling; no method renames are introduced on the production side (`RunResumed` is unchanged), so the drift gate continues to bind. See `hazard-analysis.md` H-3.

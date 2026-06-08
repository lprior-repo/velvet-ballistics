# Section 50 ArrayQueue Migration — Implementation Plan

**Bead ID candidates:** `vb-section50-1` (scanner), `vb-section50-2` (action queue), `vb-section50-3` (IPC ingress), `vb-section50-4` (contract test)
**Scope:** Resolve the two Round-4 LETHAL Section 50 violations and the two accompanying defects.
**Authoritative contract:** `velvet-ballistics-MASTER.md` §5 (Library Choices), §12 (Forbidden Hot-Path APIs), §20 (Runtime and Shard Design).
**Hot crates in scope:** `vb_ipc`, `vb_runtime`.

---

## Defect Summary

| # | Defect | File:line | Severity |
|---|--------|-----------|----------|
| D1 | `MemoryIngress::bounded` uses `crossbeam_channel::bounded` (forbidden) | `crates/vb_ipc/src/ingress.rs:77` | LETHAL |
| D1b | `MemoryIngressSender` wraps `crossbeam_channel::Sender` (cascade of D1) | `crates/vb_ipc/src/ingress.rs:55-65, 122-127` | LETHAL (derived) |
| D1c | `disconnect_sender` test helper uses `crossbeam_channel::bounded(1)` | `crates/vb_ipc/src/ingress.rs:117` | LETHAL (derived) |
| D2 | `BoundedActionCompletionQueue::inner` uses `std::sync::Mutex<VecDeque>` | `crates/vb_runtime/src/action_queue/queue.rs:19, 39` | LETHAL |
| D3 | Scanner only catches `crossbeam_channel::unbounded(` | `scripts/check-hot-cold-forbidden-apis.rs:87` | HIGH (allows D1/D1b to re-occur) |
| D4 | `array_queue_tests.rs` covers behavior but not backend identity | `crates/vb_ipc/src/queue/tests/array_queue_tests.rs` | HIGH (no regression fence) |

Reference shard command queue as the canonical ArrayQueue pattern: `crates/vb_runtime/src/shard/queue.rs:28-120` (`ShardCommandQueue` wrapping `crossbeam_queue::ArrayQueue<ShardCommand>`).

---

## Work Item 1 — `vb-section50-1` (Scanner Extension)

**Defect (D3):** `scripts/check-hot-cold-forbidden-apis.rs` declares a single `CHANNEL-UNBOUNDED-001` class at line 82-88 that only fires on `std::sync::mpsc::channel(`, `mpsc::channel(`, `unbounded_channel(`, and `crossbeam_channel::unbounded(`. The bounded variant `crossbeam_channel::bounded(` (and any unqualified `bounded(` that resolves to it) passes the scanner, which is exactly how the `vb_ipc` violation slipped in.

**Fix:**
1. In `scripts/check-hot-cold-forbidden-apis.rs:82-88`, split `CHANNEL-UNBOUNDED-001` into two related classes:
   - `CHANNEL-UNBOUNDED-001` (unchanged) — only fires on `unbounded` and `mpsc::channel(`.
   - `CHANNEL-BOUNDED-001` (new) — fires on `crossbeam_channel::bounded(` and `std::sync::mpsc::sync_channel(`.
2. Add the new class to the `required` set in `self_test` at line 273 with a positive fixture (`crates/vb_runtime/src/engine.rs` containing `let _c = crossbeam_channel::bounded(1);`) and assert it appears in the violation set.
3. The fix file remains `scripts/check-hot-cold-forbidden-apis.rs`. No `Cargo.toml` change required. The companion shell wrapper at `scripts/check-hot-cold-forbidden-apis.sh:1-50` does not require edits.

**Test plan:**
- `bash scripts/check-hot-cold-forbidden-apis.sh --self-test` — must print `FixturePass: hot/cold forbidden API scanner` and exit 0.
- A synthetic repo fixture where `crates/vb_runtime/src/engine.rs` contains `let _q = crossbeam_channel::bounded(4);` must produce a `CHANNEL-BOUNDED-001` finding. Add this fixture to `self_test()` and assert it.
- The existing `CHANNEL-UNBOUNDED-001` self-test fixture (line 254-257) must continue to pass.

**Acceptance criteria:**
- `bash scripts/check-hot-cold-forbidden-apis.sh` exits 0 only when the working tree has no `crossbeam_channel::bounded(` or `std::sync::mpsc::sync_channel(` in `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc` (after the other work items land).
- `bash scripts/check-hot-cold-forbidden-apis.sh --self-test` exits 0 in CI.

**Public API impact:** None. Scanner is a build-time tool, not a library export.

**Risk:** Low. The scanner is read-only; widening the regex set cannot regress existing passing trees (the only false positive would be text in a string literal, which the existing scanner already accepts).

**Hours:** 0.5 (the scanner change is ~10 lines plus self-test fixture).

---

## Work Item 2 — `vb-section50-2` (`BoundedActionCompletionQueue` → `ArrayQueue`)

**Defect (D2):** `BoundedActionCompletionQueue::new` (line 16-25) and `with_backpressure` (line 31-46) construct `std::sync::Mutex<Inner>` where `Inner { items: VecDeque<ActionTicket> }`. Master §5/§20 mandates bounded MPMC storage using `crossbeam-queue::ArrayQueue`. The `Mutex` is also redundant: `ArrayQueue<T>` is already `Send + Sync` with lock-free push/pop.

**Fix:**
1. Add `crossbeam-queue::ArrayQueue` to the existing `vb_runtime` dependency list (already present in `crates/vb_runtime/Cargo.toml:18`).
2. In `crates/vb_runtime/src/action_queue/types.rs`:
   - Replace `pub(crate) inner: std::sync::Mutex<Inner>` with `pub(crate) inner: std::sync::Arc<ArrayQueue<ActionTicket>>` (line 66).
   - Delete the `Inner` struct at line 71-74 and the `use std::collections::VecDeque` import (line 3).
3. In `crates/vb_runtime/src/action_queue/queue.rs`:
   - `new` (line 16-25): construct `Arc::new(ArrayQueue::new(capacity.get()))` directly. No `Mutex::new(...)` wrapper.
   - `with_backpressure` (line 31-46): same — `Arc::new(ArrayQueue::new(...))`.
   - `enqueue` (line 55-82): replace `let mut inner = ...lock()` with `let inner = &self.inner;`; call `inner.push(ticket).map_err(|_| ActionQueueError::QueueFull { capacity: self.capacity })?;`; read `inner.len()` for depth.
   - `dequeue` (line 89-95): replace lock+`pop_front` with `self.inner.pop().ok_or(None)`-equivalent (i.e. `self.inner.pop()`); eliminate the poisoned-mutex recovery block.
   - `len` (line 99-104): `self.inner.len()` (no lock).
   - `is_empty`, `is_full`, `remaining_capacity`, `capacity`: read `self.inner.len()` directly.
4. Keep the `backpressure_tx` field unchanged; the 80% threshold logic at line 151-159 stays byte-for-byte. `tx.try_send(...)` is to a `std::sync::mpsc::SyncSender<BackpressureWarning>` which is *outbound notification* (not a forbidden hot-path MPSC channel) and is the only `mpsc` use in this file — verify this remains the case post-migration.

**Public API impact:** None. `BoundedActionCompletionQueue::new`, `with_backpressure`, `enqueue`, `dequeue`, `len`, `is_empty`, `is_full`, `remaining_capacity`, `capacity` retain their exact signatures. `ActionQueueError` variants are unchanged. The `pub(crate)` visibility of `inner` and `capacity` is preserved so existing intra-crate consumers compile.

**Test plan:**
- All 18 tests in `crates/vb_ipc/src/queue/tests/array_queue_tests.rs` must remain green (they do not touch this queue, but `moon ci` is the gate).
- All action-queue tests in `crates/vb_runtime/src/action_queue/tests/` must remain green: enqueue/dequeue FIFO, full-error at capacity, 80% backpressure emission, `is_full`/`is_empty` invariants, capacity parsing.
- Add a **backend-identity assertion** (new test in the action_queue test module, gated on `#[cfg(test)]`):
  - Use `std::any::TypeId::of::<std::sync::Arc<crossbeam_queue::ArrayQueue<ActionTicket>>>()` and compare to `TypeId::of_val(&*queue.inner)`. The test must fail if `inner` is wrapped in `Mutex` or holds a `VecDeque`.
  - This test is the `BoundedActionCompletionQueue` analogue of contract test D4.
- `cargo nextest run -p vb_runtime action_queue::` must pass.

**Acceptance criteria:**
- `cargo build -p vb_runtime` exits 0.
- `cargo nextest run -p vb_runtime` exits 0.
- `bash scripts/check-hot-cold-forbidden-apis.sh` no longer reports `CHANNEL-UNBOUNDED-001` or `CHANNEL-BOUNDED-001` against `crates/vb_runtime/src/action_queue/**`.
- `grep -nR "std::sync::Mutex<VecDeque" crates/vb_runtime/src/` returns no matches.
- `grep -nR "std::sync::Mutex<Inner" crates/vb_runtime/src/action_queue/` returns no matches.
- The new backend-identity test passes.

**Risk:** Medium.
- R-A1: Lock-free `ArrayQueue::pop` returns `None` for "empty"; we lose the panic-poisoning recovery path used in `len` (line 100-103). `ArrayQueue` cannot be poisoned, so this is a strict simplification — no behavioral loss.
- R-A2: `dequeue` no longer returns `Option<ActionTicket>` from a lock; it now reads from the same `Arc<ArrayQueue>` that `enqueue` writes to. Concurrent enqueue+dequeue is safe (`ArrayQueue` is MPMC) but the `Arc` clone is cheap. Verify no caller clones the queue — grep for `BoundedActionCompletionQueue { ... }` literal construction outside the crate.
- R-A3: Backpressure threshold arithmetic is computed on `self.inner.len()` *after* a successful push. With `ArrayQueue`, `push` either succeeds and increments len or fails with `Full`; the post-push len read is well-defined and bounded by `capacity`.

**Hours:** 3.0 (replacement + 1 new test + lock-elimination edits + verifying action_queue tests).

---

## Work Item 3 — `vb-section50-3` (`MemoryIngress` → `ArrayQueue`)

**Defect (D1, D1b, D1c):** `MemoryIngress` (line 67-119) and `MemoryIngressSender` (line 53-64) wrap `crossbeam_channel::Sender<IngressFrame>` / `Receiver<IngressFrame>`. The `disconnect_sender` test helper (line 115-119) also constructs a fresh `crossbeam_channel::bounded(1)` to simulate sender drop. Section 5 mandates `crossbeam-queue::ArrayQueue` for bounded MPMC ingress. The `disconnect_sender` helper exists solely to make the `Disconnected` variant testable; the new backend needs an equivalent mechanism because `ArrayQueue` has no native "sender disconnected" signal.

**Fix:**

1. **Inner struct.** Add a new private struct `IngressCore` (sibling of `MemoryIngress`):
   ```rust
   struct IngressCore {
       queue: ArrayQueue<IngressFrame>,
       disconnected: AtomicBool,
   }
   ```
   `MemoryIngress` and `MemoryIngressSender` each hold `Arc<IngressCore>`. `Arc` provides the shared MPMC state; `AtomicBool` provides the disconnect signal without a lock.

2. **`MemoryIngress::bounded` (line 74-79).** Replace:
   ```rust
   let (sender, receiver) = crossbeam_channel::bounded(capacity.get());
   Self { sender, receiver }
   ```
   with:
   ```rust
   let core = Arc::new(IngressCore {
       queue: ArrayQueue::new(capacity.get()),
       disconnected: AtomicBool::new(false),
   });
   Self { core }
   ```

3. **`MemoryIngress::producer` (line 81-87).** Replace the `crossbeam_channel::Sender::clone()` chain with a `MemoryIngressSender { core: Arc::clone(&self.core) }`.

4. **`MemoryIngressSender` (line 53-64).** Store `core: Arc<IngressCore>` instead of `sender: Sender<IngressFrame>`. `try_submit` becomes:
   ```rust
   self.core
       .queue
       .push(frame)
       .map_err(|_| IpcError::Full)
   ```
   The `submit_to_sender` free function (line 122-127) is deleted; the call sites at line 62 and 91 inline the three-line push logic.

5. **`MemoryIngress::try_submit` (line 89-92).** Same inlined push as above. `disconnected` is set on enqueue failure only if a downstream consumer would care; enqueue on a "disconnected" queue is still rejected with `IpcError::Full` (the queue is full OR no receiver cares, but the public contract says `Full` is the only non-`Disconnected` failure).

6. **`MemoryIngress::try_recv` (line 94-101).** Replace `receiver.try_recv()` matching with:
   ```rust
   match self.core.queue.pop() {
       Some(frame) => Ok(Some(frame)),
       None => {
           if self.core.disconnected.load(Ordering::Acquire) {
               Err(IpcError::Disconnected)
           } else {
               Ok(None)
           }
       }
   }
   ```

7. **`MemoryIngress::len` / `is_empty` (line 103-113).** `self.core.queue.len()` / `self.core.queue.is_empty()`.

8. **`MemoryIngress::disconnect_sender` test helper (line 115-119).** Replace the crossbeam-channel swap with `self.core.disconnected.store(true, Ordering::Release);`. Remove the `#[cfg(test)]` attribute that referenced `let (new_sender, _) = crossbeam_channel::bounded(1);`. The helper is `pub(crate)` and only called from `crates/vb_ipc/src/ingress/tests.rs` and `crates/vb_ipc/src/queue/tests/array_queue_tests.rs` — both compile in the same crate, so the helper stays crate-visible.

9. **Imports.** Delete `use crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError};` (line 5). Add `use crossbeam_queue::ArrayQueue;` and `use std::sync::atomic::{AtomicBool, Ordering};`.

10. **Cargo.toml.** Remove `crossbeam-channel.workspace = true` from `crates/vb_ipc/Cargo.toml:11` after the only use site (`ingress.rs`) is migrated. Add `crossbeam-queue.workspace = true` (already in workspace `Cargo.toml:41`). Verify with `cargo metadata` that no other `vb_ipc` source file imports `crossbeam_channel`.

**Public API impact:** None.
- `MemoryIngress::bounded(QueueCapacity) -> Self` — unchanged.
- `MemoryIngress::producer() -> MemoryIngressSender` — unchanged.
- `MemoryIngressSender::try_submit(IngressFrame) -> Result<(), IpcError>` — unchanged.
- `MemoryIngress::try_submit(IngressFrame) -> Result<(), IpcError>` — unchanged.
- `MemoryIngress::try_recv() -> Result<Option<IngressFrame>, IpcError>` — unchanged.
- `MemoryIngress::len() -> usize` — unchanged.
- `MemoryIngress::is_empty() -> bool` — unchanged.
- `MemoryIngressSender: Clone + Debug` — preserved via `#[derive(Debug, Clone)]` on the new struct.
- `MemoryIngress: Debug` — preserved via `#[derive(Debug)]` (Debug can be derived because `Arc<IngressCore>` is `Debug` and `IngressCore` derives `Debug`).
- `IpcError::Full` and `IpcError::Disconnected` semantics preserved exactly.

**Test plan:**
- All 18 tests in `crates/vb_ipc/src/queue/tests/array_queue_tests.rs` must pass byte-for-byte unchanged:
  - `memory_ingress_bounded_constructor_produces_memory_ingress_instance`
  - `memory_ingress_bounded_with_capacity_one_succeeds`
  - `memory_ingress_bounded_with_various_capacities_succeeds`
  - `memory_ingress_try_submit_succeeds_when_queue_has_capacity`
  - `ingress_frame_new_*` (4 tests, untouched)
  - `bounded_payload_*` (3 tests, untouched)
  - `memory_ingress_try_submit_returns_full_when_queue_is_at_capacity`
  - `memory_ingress_try_submit_full_is_exact_variant_not_disconnected`
  - `memory_ingress_try_recv_returns_fifo_order_when_queue_has_items`
  - `memory_ingress_try_recv_returns_none_when_queue_is_empty`
  - `memory_ingress_try_recv_empty_differs_from_disconnected`
  - `memory_ingress_try_recv_returns_disconnected_when_sender_dropped`
  - `memory_ingress_try_recv_returns_disconnected_after_partial_submit`
  - `memory_ingress_len_returns_exact_count_when_queue_has_two_frames`
  - `memory_ingress_len_never_exceeds_capacity`
  - `memory_ingress_is_empty_*` (3 tests)
  - `submit_capacity_plus_one_produces_exactly_one_full_error`
  - `recv_on_empty_never_returns_unexpected_error_variant`
  - `disconnected_recv_never_returns_ok_or_full`
  - The 4 proptests (`fifo_order_invariant_for_submit_recv_cycle`, `is_empty_len_zero_invariant_after_mixed_operations`, `capacity_one_full_empty_signaling_invariant`, `len_exact_count_invariant_after_every_submit`).
- All 9 ingress tests in `crates/vb_ipc/src/ingress/tests.rs` (lines 44, 60, 67, 94, 122, 153, 171, etc.) must remain green.
- All cross-crate consumers compile: `crates/vb_cli/tests/cross_crate_adversarial.rs:677, 722`, `crates/workspace_tests/benches/velvet_ballistics.rs:1136, 1156, 1174`, `crates/workspace_tests/tests/vb_te1i_binary_ipc_acceptance.rs:320`, `crates/vb_ipc/src/tests.rs` (10 sites).
- New backend-identity contract test: see Work Item 4.

**Acceptance criteria:**
- `cargo build -p vb_ipc` exits 0.
- `cargo nextest run -p vb_ipc` exits 0.
- `bash scripts/check-hot-cold-forbidden-apis.sh` does not report `CHANNEL-UNBOUNDED-001` or `CHANNEL-BOUNDED-001` against any file under `crates/vb_ipc/src/`.
- `grep -nR "crossbeam_channel" crates/vb_ipc/src/` returns no matches.
- The `MemoryIngress::disconnect_sender` helper still produces `Err(IpcError::Disconnected)` on subsequent `try_recv()` calls.

**Risk:** High — this is the most invasive single-file change.
- R-I1: `crossbeam_channel::Sender::clone` is cheap (Arc clone); `ArrayQueue` shared via `Arc<IngressCore>` clone is also cheap. No perf regression in producer path.
- R-I2: `ArrayQueue::push` returns `Result<T, T>` (the unsent item on failure). We must convert via `map_err(|_| IpcError::Full)` — *not* preserve the frame, matching existing `try_send` behavior at line 123-127.
- R-I3: `Ordering::Acquire/Release` on the `disconnected` flag must match across producer and consumer. Use `Release` on store (in `disconnect_sender`) and `Acquire` on load (in `try_recv`). Acq/Rel is sufficient — no `SeqCst` needed.
- R-I4: The `Drop` semantics differ. `crossbeam_channel` returns `Disconnected` when *all* senders are dropped; `ArrayQueue` cannot detect sender drop. The current public test surface uses explicit `disconnect_sender()` and we are not changing that. No silent behavior change for callers that never relied on implicit disconnect detection (there are no such callers — `vb_ipc` is the only ingress layer; the consumer side is the binary IPC server which has the `MemoryIngress` and owns it).
- R-I5: `MemoryIngressSender` previously wrapped `crossbeam_channel::Sender` which is `!Sync` (single-producer ownership), but `Clone`. The new `MemoryIngressSender { core: Arc<IngressCore> }` is `Sync` (because `Arc<T>: Sync` when `T: Send + Sync`). This is strictly more permissive and cannot regress any caller.
- R-I6: The `#[derive(Debug)]` on `MemoryIngress` worked with `Sender`/`Receiver` (both `Debug`). `Arc<IngressCore>` is `Debug` iff `IngressCore: Debug`, so derive `Debug` on `IngressCore` too. `ArrayQueue<T: Debug>: Debug` and `AtomicBool: Debug`, so derive works.

**Hours:** 5.0 (3.0 for the migration + 1.5 for the new contract test wired into `array_queue_tests.rs` + 0.5 to verify the 18 existing tests + 0.5 for the `Cargo.toml` cleanup).

---

## Work Item 4 — `vb-section50-4` (Contract Test for Backend Identity)

**Defect (D4):** `crates/vb_ipc/src/queue/tests/array_queue_tests.rs` (913 lines, 18+ tests) covers behavior — full, empty, FIFO, disconnect, len, is_empty — but never asserts *which* backend implements the queue. A future agent could swap back to `crossbeam_channel` (or a `VecDeque<Mutex>`) without breaking a single existing test. This is the same defect as the action queue: behavioral tests prove the contract is met, but the architectural choice is unfenced.

**Fix:**

1. Add a new `#[cfg(test)]` test module in `crates/vb_ipc/src/queue/tests/array_queue_tests.rs` (sibling of the existing `// ═════...` BDD blocks) titled `// Backend identity contract — MAJOR-1 ArrayQueue`.

2. Test A — `memory_ingress_backend_is_array_queue_not_crossbeam`:
   - `let ingress = MemoryIngress::bounded(capacity(4));`
   - Use `std::any::TypeId` introspection: a `MemoryIngress` is opaque, so this test must rely on observable evidence. Two complementary assertions:
     - (A1) Construct a fresh `MemoryIngress`, take a `producer()`, submit 5 frames to a capacity-4 queue, assert the 5th returns `Err(IpcError::Full)`. The 4-capacity bound is *only* guaranteed by a fixed-capacity backend — a `VecDeque<Mutex>` would not produce a `Full` error at exactly 4, and `crossbeam_channel::bounded(4)` is exactly what the scanner now forbids. This proves "bounded MPMC" is the contract.
     - (A2) Drop a `producer()` handle, submit from a different `producer()`, assert `Ok(())` — proves multi-producer support.
     - (A3) Concurrent test (sequential, but covering both code paths): submit from `MemoryIngress::try_submit` *and* from `producer().try_submit` on the same queue; assert no frame loss and FIFO across producers.

3. Test B — `memory_ingress_uses_no_crossbeam_types_in_ingress_module`:
   - Add a *negative* assertion in the form of a comment that must be kept in sync:
     ```rust
     // The scanner scripts/check-hot-cold-forbidden-apis.sh will reject
     // this file if `crossbeam_channel::` re-appears. See CI.
     ```
   - This test is documentation-grade but its presence in the test file makes the contract explicit.

4. Test C — `bounded_action_completion_queue_backend_is_array_queue`:
   - Live in `crates/vb_runtime/src/action_queue/tests/action_queue_tests.rs` (the action-queue analog of the above).
   - Construct `BoundedActionCompletionQueue::new(4)?`, push 5 tickets, assert 5th returns `Err(ActionQueueError::QueueFull { .. })`.
   - Push 4, assert `len() == 4`, dequeue all, assert FIFO.
   - Concurrent push from two threads (`std::thread::scope`) at capacity 64 with 128 attempts total; assert zero panics, no lost tickets.

5. Test D — `scanner_rejects_crossbeam_channel_bounded`:
   - Lives in `scripts/check-hot-cold-forbidden-apis.rs` `self_test()` (already present at line 237-285). The new fixture (added in Work Item 1) is the contract test for the scanner itself.

**Test plan:**
- `cargo nextest run -p vb_ipc array_queue_tests::` must report the new tests as passing.
- `cargo nextest run -p vb_runtime action_queue::` must report the new tests as passing.
- `bash scripts/check-hot-cold-forbidden-apis.sh --self-test` must pass.
- `bash scripts/check-hot-cold-forbidden-apis.sh` against a deliberately-broken tree (one where `vb_ipc/src/ingress.rs` contains `crossbeam_channel::bounded(1)`) must exit non-zero with a `CHANNEL-BOUNDED-001` finding.

**Acceptance criteria:**
- The four new tests above pass.
- Removing `crossbeam_channel` from `vb_ipc/Cargo.toml` does not break compilation.
- A deliberately re-introduced `crossbeam_channel::bounded(1)` in `crates/vb_ipc/src/ingress.rs` is caught by the scanner (D3 fix from Work Item 1) **and** would fail the backend-identity test (D4 fix from this work item) at compile time because `disconnect_sender` no longer compiles.

**Risk:** Low.
- R-T1: The "TypeId introspection" approach cannot reach inside `MemoryIngress` (fields are private). The behavior-based backend identity (Test A1-A3) is the correct way to fence the contract.
- R-T2: Test C (concurrent action queue) is the only multi-threaded test. Use `std::thread::scope` (stable since 1.63) and `std::sync::Barrier` for deterministic start. Cap pushes at 64×2 = 128 to keep test runtime under 1 second.
- R-T3: Tests must not introduce timing-dependent assertions. Use `is_ok()` / `is_err()` only.

**Hours:** 2.0 (1.0 for IPC contract test, 0.5 for action-queue contract test, 0.5 for scanner self-test expansion).

---

## Migration Order

The order is chosen so each step leaves the tree in a green, demonstrably-correct state.

1. **`vb-section50-1` (Scanner).** Pure tooling change. No source code touched. Land first so that D1, D1b, and D2 cannot reappear unnoticed during the migration. Defense in depth up front.
2. **`vb-section50-2` (Action Queue).** Single-crate change in `vb_runtime` with no exotic semantics (no disconnect, no cloneable producers, no public-API shift beyond internal field types). Builds muscle for the harder IPC migration. Public API of `BoundedActionCompletionQueue` is unchanged.
3. **`vb-section50-3` (IPC Ingress).** Most invasive single-file change. The new `Arc<IngressCore>` + `AtomicBool` disconnect pattern is introduced here. Public API of `MemoryIngress` and `MemoryIngressSender` is preserved. All 18 existing tests must remain green.
4. **`vb-section50-4` (Contract Test).** Locks down the migration. Fences against future regression. Runs last so it tests the final state of the tree.

The order is sequential because:
- Steps 2-3 cannot proceed in parallel from the same workspace (Cargo lock contention, shared `target/`).
- Step 4 depends on the final state of 2 and 3.
- Step 1 is independent of all source changes and can be merged at any time before the work item 2/3 PRs land, but landing it first means the PRs for 2 and 3 cannot re-introduce the violation.

---

## Total Work-Hour Estimate

| Work Item | Hours | Critical Path? |
|-----------|-------|----------------|
| `vb-section50-1` Scanner | 0.5 | Yes — must land first |
| `vb-section50-2` Action Queue | 3.0 | Yes |
| `vb-section50-3` IPC Ingress | 5.0 | Yes |
| `vb-section50-4` Contract Test | 2.0 | Yes — gates the merge |
| **Total** | **10.5 hours** | |

Add a 1.5-hour buffer for CI flake, moon-ci gates, and any `bd` plumbing (claim, close, dolt push, evidence packaging). **Realistic total: ~12 hours of focused work across 2-3 sessions.**

---

## Definition of Done (LETHAL Closure)

The Section 50 LETHAL is closed when **all** of the following evidence is attached to the bead(s) and verified:

1. `cargo build --workspace --all-targets` exits 0.
2. `moon ci` exits 0 (canonical gate per `AGENTS.md`).
3. `cargo nextest run --workspace` exits 0 — including all 18 tests in `array_queue_tests.rs`, all action-queue tests, and the 4 new contract tests from Work Item 4.
4. `bash scripts/check-hot-cold-forbidden-apis.sh` exits 0 with no `CHANNEL-UNBOUNDED-001` and no `CHANNEL-BOUNDED-001` findings in any hot crate.
5. `bash scripts/check-hot-cold-forbidden-apis.sh --self-test` exits 0.
6. `grep -nR "crossbeam_channel" crates/vb_ipc/src/ crates/vb_runtime/src/action_queue/` returns no matches.
7. `grep -nR "std::sync::Mutex<VecDeque\|std::sync::Mutex<Inner" crates/vb_runtime/src/` returns no matches.
8. `Cargo.lock` no longer requires `crossbeam-channel` for `vb_ipc` or `vb_runtime`; `crossbeam-queue` is present and pinned.
9. The four new contract tests are referenced in the bead description with `file_path:line_number` for each.
10. The bead `vb-section50-*` is closed in `bd` and `bd dolt push` succeeds; git push succeeds; `git status` is clean.
11. The behavioral assertion set (18 IPC tests + action-queue tests + 4 new contract tests) is reproduced in the bead's evidence file as a `cargo nextest run` excerpt.

If any of the eleven conditions above fails, the LETHAL is not closed. The bead is reopened with a follow-up.

---

## Files Touched (Summary)

| File | Change | Work Item |
|------|--------|-----------|
| `scripts/check-hot-cold-forbidden-apis.rs` | Add `CHANNEL-BOUNDED-001` class + self-test fixture | 1 |
| `scripts/hot-cold-forbidden-apis.allow` | (No change unless a temporary exception is needed; the master mandates no exceptions here) | — |
| `crates/vb_runtime/src/action_queue/types.rs` | Remove `Inner { items: VecDeque<...> }`; replace `Mutex<Inner>` with `Arc<ArrayQueue<ActionTicket>>` | 2 |
| `crates/vb_runtime/src/action_queue/queue.rs` | Eliminate `Mutex` lock/poison handling; call `ArrayQueue::push` / `pop` / `len` directly | 2 |
| `crates/vb_runtime/src/action_queue/tests/action_queue_tests.rs` | Add `bounded_action_completion_queue_backend_is_array_queue` test | 4 |
| `crates/vb_ipc/src/ingress.rs` | Replace `crossbeam_channel` with `Arc<IngressCore { queue: ArrayQueue, disconnected: AtomicBool }>` | 3 |
| `crates/vb_ipc/Cargo.toml` | Drop `crossbeam-channel`; add `crossbeam-queue` | 3 |
| `crates/vb_ipc/src/queue/tests/array_queue_tests.rs` | Add `memory_ingress_backend_is_array_queue_not_crossbeam` (and siblings) | 4 |
| `Cargo.lock` | Regenerated by Cargo | 2, 3 |

No `Cargo.toml` workspace root change required (`crossbeam-queue = "0.3"` is already declared at line 41).

---

## Bead Tracking

| Bead ID | Title | Status | Owner |
|---------|-------|--------|-------|
| `vb-section50-1` | Scanner: catch `crossbeam_channel::bounded(` (CHANNEL-BOUNDED-001) | open | tbd |
| `vb-section50-2` | `vb_runtime`: `BoundedActionCompletionQueue` → `ArrayQueue` | blocked-by vb-section50-1 | tbd |
| `vb-section50-3` | `vb_ipc`: `MemoryIngress` → `ArrayQueue` (preserve public API) | blocked-by vb-section50-1 | tbd |
| `vb-section50-4` | Contract test: backend identity fence for both queues | blocked-by vb-section50-2, vb-section50-3 | tbd |

Run `bd ready` after each close; `vb-section50-4` unblocks automatically when both 2 and 3 are closed. Do not parallelize 2 and 3 from a single workspace; use separate worktrees or serialize.

# Proof Repair Guide: vb-core-ipc-loom-property

**Bead**: vb-core-ipc-loom-property
**State**: 6 (Proof Review — REJECTED)
**Workspace**: /tmp/vb-ws/vb-core-ipc-loom-property

---

## REJECTION SUMMARY

2 MAJOR blockers prevent approval. Repair these before re-submission for proof review.

---

## BLOCKER 1: CAS Retry Loop in `memory_ingress.rs`

**Priority**: CRITICAL
**File**: `crates/vb_ipc/src/models/loom/memory_ingress.rs`
**Lines**: 30-49
**Contract clause**: INV-001
**Verifier**: loom

### Problem

`try_submit` and `try_recv` use a single `compare_exchange` without a retry loop:

```rust
// CURRENT (BROKEN):
fn try_submit(&self) -> bool {
    let current = self.queued.load(Ordering::SeqCst);
    if current >= self.capacity {
        return false;
    }
    self.queued
        .compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    // ^ If CAS fails due to concurrent modification, returns false even when there was room
}
```

The single CAS approach fails silently when:
1. Another thread modified `queued` between the load and the CAS
2. The CAS fails spuriously (allowed by memory ordering semantics)

### Fix

Wrap in a retry loop:

```rust
fn try_submit(&self) -> bool {
    loop {
        let current = self.queued.load(Ordering::SeqCst);
        if current >= self.capacity {
            return false;
        }
        match self.queued.compare_exchange(
            current,
            current + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => return true,
            Err(_) => continue, // CAS failed (concurrent modification), retry
        }
    }
}

fn try_recv(&self) -> bool {
    loop {
        let current = self.queued.load(Ordering::SeqCst);
        if current == 0 {
            return false;
        }
        match self.queued.compare_exchange(
            current,
            current - 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => return true,
            Err(_) => continue, // CAS failed (concurrent modification), retry
        }
    }
}
```

### Why This Matters

The model is meant to represent the lock-free bounded queue behavior. Without retry loops, the model fails to represent the actual concurrent behavior and the loom exploration might pass for the wrong reason (the CAS failing and returning false looks like the channel being full, not a concurrent modification).

---

## BLOCKER 2: `memory_ingress_multi_producer` Does Not Spawn All Producers

**Priority**: HIGH
**File**: `crates/vb_ipc/src/models/loom/memory_ingress.rs`
**Lines**: 100-131
**Contract clause**: INV-001
**Verifier**: loom

### Problem

The test claims "3 producers x 3 consumers" but only `q1` is spawned via `loom::thread::spawn`. `q2` and `q3` are cloned but never used — they drop immediately:

```rust
// CURRENT (BROKEN):
let q1 = queue.clone();
let q2 = queue.clone(); // never used
let q3 = queue.clone(); // never used

loom::thread::spawn(move || {
    for _ in 0..2 { q1.try_submit(); }
});
// q2 and q3 go out of scope here unused
```

The "3 consumers" also run on the main thread sequentially (not spawned):

```rust
for _ in 0..3 {
    let _ = queue.try_recv(); // sequential on main thread
}
```

### Fix

```rust
fn memory_ingress_multi_producer() {
    loom::model(|| {
        let queue = Arc::new(BoundedQueue::new(4));
        let q1 = queue.clone();
        let q2 = queue.clone();
        let q3 = queue.clone();
        let c1 = queue.clone();
        let c2 = queue.clone();
        let c3 = queue.clone();

        // Three producers each submit 2 frames
        loom::thread::spawn(move || {
            for _ in 0..2 { q1.try_submit(); }
        });
        loom::thread::spawn(move || {
            for _ in 0..2 { q2.try_submit(); }
        });
        loom::thread::spawn(move || {
            for _ in 0..2 { q3.try_submit(); }
        });

        // Three consumers each receive 1 frame
        loom::thread::spawn(move || {
            let _ = c1.try_recv();
        });
        loom::thread::spawn(move || {
            let _ = c2.try_recv();
        });
        loom::thread::spawn(move || {
            let _ = c3.try_recv();
        });

        queue.check_invariant();
    });
}
```

---

## NON-BLOCKER: Duplicate/Wrong Assertion

**Priority**: LOW
**File**: `crates/vb_ipc/src/models/loom/memory_ingress.rs`
**Lines**: 67-71

### Problem

`check_invariant` asserts `q <= self.capacity` twice. Second has wrong message "underflow":

```rust
// current:
assert!(q <= self.capacity, "queued {} exceeds capacity {}", q, self.capacity); // line 61
assert!(q <= self.capacity, "queued {} is negative (underflow)", q); // line 67 — WRONG
```

### Fix

Remove the second assertion (or fix the message):

```rust
fn check_invariant(&self) {
    let q = self.queued();
    assert!(q <= self.capacity, "queued {} exceeds capacity {}", q, self.capacity);
    // usize is always >= 0 in Rust, no need to check underflow
}
```

---

## NON-BLOCKER: `ipc_server_clients` Serialized Tests

**Priority**: LOW
**File**: `crates/vb_ipc/src/models/loom/ipc_server_clients.rs`

### Problem

All operations on `SharedClientMap` are serialized by a single mutex. While correct for mutex-based model, the tests don't explore concurrent interleavings — they just confirm mutex serialization works.

### Fix (Optional)

Add a comment documenting the limitation:

```rust
/// Thread-safe wrapper for loom exploration.
/// NOTE: All operations are serialized by a single mutex. This model tests
/// that the mutex-based implementation preserves invariants, but does NOT
/// explore partial-operation interleavings (e.g., interleaving between
/// reading clients.len() and inserting). For full interleaving coverage,
/// a lock-free variant would be needed.
#[derive(Debug, Clone)]
struct SharedClientMap {
    inner: Arc<Mutex<ClientMap>>,
}
```

Or redesign to use finer-grained locking (e.g., separate mutexes for `clients` and `next_token`) to expose potential race conditions.

---

## NON-BLOCKER: `write_buffer` Serialized Tests

**Priority**: LOW
**File**: `crates/vb_ipc/src/models/loom/write_buffer.rs`

Same pattern as `ipc_server_clients`. Consider documenting the serialization limitation.

---

## NON-BLOCKER: `frame_pool_capacity_boundary` Pre-pop Issue

**Priority**: LOW
**File**: `crates/vb_runtime/src/models/loom/frame_pool.rs`
**Lines**: 172-191

### Problem

Test pre-populates 4 frames (at capacity), takes 1 (bringing to 3), then releases. The release succeeds (adds back to 4) rather than being silently dropped. The "silent drop at capacity" path (POST-002) is not tested.

### Fix

```rust
fn frame_pool_capacity_boundary() {
    loom::model(|| {
        let pool = ThreadSafeFramePool::new(4, 8, 4);
        let p1 = pool.clone();

        // Fill pool to capacity
        for i in 0..4 {
            pool.release(i);
        }
        assert_eq!(pool.available(), 4);

        // Now pool is at capacity. Concurrent take and release:
        // take will bring to 3, release should add back to 4
        // To test silent drop: need release to happen when pool is ALREADY at capacity
        // after the take. But since we control timing, we can't guarantee this.
        //
        // Alternative: take then release-to-full-race:
        let handle = loom::thread::spawn(move || {
            p1.take(1, 0); // pool goes from 4 -> 3
        });

        // Main thread: release while take is running
        // If timing is right, pool is at 3, so release succeeds
        pool.release(200);

        handle.join();
        pool.check_invariant();
    });
}
```

Or test the silent drop explicitly:
```rust
// After take brings pool to 3, manually add one more (succeeds),
// then try to add when at capacity (should silently drop)
```

---

## Regression Check (Pre-existing Models)

EXISTING-001..005 (journal_writer_queue, action_completion_cancel, timer_fired_cancel, shutdown_drain, bounded_queue) were not re-run. Proof-writer report claims they "pass" but provides no evidence.

**Required**: Re-run all 5 existing loom models and attach output:
```bash
RUSTFLAGS="--cfg loom" cargo test -p vb_runtime bounded_queue -- --nocapture
RUSTFLAGS="--cfg loom" cargo test -p vb_runtime action_completion_cancel -- --nocapture
RUSTFLAGS="--cfg loom" cargo test -p vb_runtime timer_fired_cancel -- --nocapture
RUSTFLAGS="--cfg loom" cargo test -p vb_runtime shutdown_drain -- --nocapture
RUSTFLAGS="--cfg loom" cargo test -p vb_runtime journal_writer_queue -- --nocapture
```

---

## Repair Checklist

- [ ] Fix CAS retry loop in `try_submit` and `try_recv` (`memory_ingress.rs`)
- [ ] Fix `memory_ingress_multi_producer` to actually spawn 3 producer threads
- [ ] Fix duplicate/wrong assertion in `check_invariant`
- [ ] Re-run EXISTING-001..005 and attach evidence
- [ ] (Optional) Document serialization limitations in `ipc_server_clients.rs` and `write_buffer.rs`

---

## After Repairs

After making these fixes, update `STATE.md` to state 4 (Proof Writing) and re-submit for proof review.

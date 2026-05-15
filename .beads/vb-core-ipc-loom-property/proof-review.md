# Proof Review: vb-core-ipc-loom-property

**Bead**: vb-core-ipc-loom-property
**State**: 6 (Proof Review)
**Workspace**: /tmp/vb-ws/vb-core-ipc-loom-property
**Reviewer**: proof-reviewer (go-skill pipeline)

## Summary

4 new loom models reviewed. Found **3 MAJOR** and **2 MINOR** issues. Models partially cover intended invariants but have correctness gaps in concurrency testing methodology and assertion logic.

---

## Findings

### MAJOR-1: `memory_ingress.rs` — CAS operations lack retry loops

**File**: `crates/vb_ipc/src/models/loom/memory_ingress.rs`
**Lines**: 30-38 (`try_submit`), 41-49 (`try_recv`)
**Problem**: `try_submit` and `try_recv` use a single `compare_exchange` without a retry loop. If the CAS fails spuriously (allowed by memory ordering) or if `queued` changes between load and CAS (concurrent modification), the operation returns `false` silently rather than retrying. This means:
- Lost submissions: a concurrent `try_recv` between load and CAS causes `try_submit` to return `false` even though there was room
- Lost receipts: a concurrent `try_submit` between load and CAS causes `try_recv` to return `false` even though there was an item

**Correct pattern**: CAS loop (`loop { match cas { ok => break, Err(_) => continue } }`)

**Impact**: The model does NOT accurately represent the lock-free bounded queue behavior. The invariant `queued <= capacity` might pass in loom exploration but for the wrong reason (serialization rather than correct CAS).

---

### MAJOR-2: `memory_ingress.rs` — `memory_ingress_multi_producer` is NOT multi-producer

**File**: `crates/vb_ipc/src/models/loom/memory_ingress.rs`
**Lines**: 100-131
**Problem**: The test claims "3 producers x 3 consumers" but only spawns ONE thread via `loom::thread::spawn`. `q2` and `q3` are cloned but never spawned — they are dropped unused. The "consumer" loop on lines 125-127 runs on the main thread sequentially.

**Evidence**:
```rust
loom::thread::spawn(move || { q1.try_submit(); q1.try_submit(); }); // only this runs in thread
// q2 and q3 are clones that go out of scope unused
for _ in 0..3 { let _ = queue.try_recv(); } // sequential on main thread
```

**Impact**: The concurrency test for INV-001 (multi-producer) only tests single-producer behavior. This was also an explicit claim in the obligation (3x3x3 rounds).

---

### MAJOR-3: `ipc_server_clients.rs` — All tests are sequential (no concurrent interleavings explored)

**File**: `crates/vb_ipc/src/models/loom/ipc_server_clients.rs`
**Lines**: 133-152 (and other tests)
**Problem**: `ipc_server_clients_concurrent_accepts` does NOT use `loom::thread::spawn` for the accepts. Each thread calls `accept` on its own `SharedClientMap` clone — but because all clones share the SAME `Arc<Mutex<ClientMap>>`, the accepts are serialized by the single mutex. The test still passes invariants because mutex serialization is correct, but it does NOT explore any concurrent interleavings.

**Evidence**:
```rust
let m1 = map.clone();
let m2 = map.clone();
let m3 = map.clone();
loom::thread::spawn(move || { let _t1 = m1.accept(1); }); // serialized by shared mutex
loom::thread::spawn(move || { let _t2 = m2.accept(2); }); // serialized by shared mutex
loom::thread::spawn(move || { let _t3 = m3.accept(3); }); // serialized by shared mutex
map.check_invariants();
```

All three threads run, but they serialize on the mutex. This is correct behavior for the mutex-based model but tests 0 interleavings. To test the mutex itself, you'd need operations that DON'T fully serialize (e.g., multiple readers).

**Impact**: LOOM-IPC-001 does not explore concurrent interleavings — only mutex serialization. This weakens the evidence for INV-003.

---

### MINOR-1: `memory_ingress.rs` — Duplicate assertion with wrong message

**File**: `crates/vb_ipc/src/models/loom/memory_ingress.rs`
**Lines**: 67-71
**Problem**: `check_invariant` asserts `q <= self.capacity` twice. The second has message "underflow" which is factually wrong (it's checking upper bound, not underflow).

```rust
assert!(q <= self.capacity, "queued {} exceeds capacity {}", q, self.capacity); // line 61-66
assert!(q <= self.capacity, "queued {} is negative (underflow)", q); // line 67-71 — WRONG MESSAGE
```

---

### MINOR-2: `write_buffer.rs` — Concurrent test also not truly concurrent

**File**: `crates/vb_ipc/src/models/loom/write_buffer.rs`
**Lines**: 139-160
**Problem**: Same pattern as MAJOR-3. `write_buffer_concurrent` spawns two threads but both `b1` and `b2` use the SAME `SharedWriteBuffer` with a single mutex. The operations are serialized. Loom explores orderings of acquiring the mutex, but since `fill` and `drain` are each a single locked operation, there are no partial-operation interleavings to find.

---

## Loom Model Review Checklist

| Model | Obligation | File | Invariant | Tests | CAS Loop | Truly Concurrent | Notes |
|-------|-----------|------|-----------|-------|----------|-----------------|-------|
| memory_ingress | LOOM-MI-001 | memory_ingress.rs | queued <= capacity | 3 | NO | PARTIAL (1 thread) | MAJOR-1, MAJOR-2 |
| ipc_server_clients | LOOM-IPC-001 | ipc_server_clients.rs | token uniqueness + active <= MAX_CLIENTS | 4 | N/A (mutex) | NO | MAJOR-3 |
| write_buffer | LOOM-IPC-002 | write_buffer.rs | byte conservation | 4 | N/A (mutex) | PARTIAL | MINOR-2 |
| frame_pool | LOOM-FP-001 | frame_pool.rs | available <= capacity | 4 | N/A (mutex) | YES (threads spawn correctly) | boundary test has pre-pop issues |

---

## Contract Clause Coverage

| Clause | Invariant | Loom Coverage | TLA+ (optional) | Notes |
|--------|-----------|--------------|-----------------|-------|
| INV-001 | MemoryIngress backpressure | Partial (no CAS loop, not multi-producer) | TLA-MI-001 (not run) | Weak evidence |
| INV-002 | FramePool capacity | Adequate (mutex variant) | VERUS-FP-001 (optional) | Adequate |
| INV-003 | IPC client-map token uniqueness | Adequate (but not truly concurrent) | TLA-IPC-001 (not run) | Adequate |
| INV-004 | Write buffer byte conservation | Adequate (but not truly concurrent) | TLA-IPC-002 (not run) | Adequate |

---

## Verification Layer Fit

- **LOOM-MI-001**: Correct verifier for concurrent channel usage. CAS loop missing is a correctness issue. **Layer fit: CORRECT, execution: FLAWED**
- **LOOM-IPC-001**: Correct verifier for mutex-protected map. Correctly uses mutex. **Layer fit: CORRECT, coverage: WEAK** (no interleavings due to single mutex)
- **LOOM-IPC-002**: Correct verifier. **Layer fit: CORRECT, coverage: WEAK**
- **LOOM-FP-001**: Correct verifier for thread-safe variant. **Layer fit: CORRECT, coverage: ADEQUATE**

---

## Required Fixes

### Fix 1: CAS retry loop in `memory_ingress.rs`

```rust
fn try_submit(&self) -> bool {
    loop {
        let current = self.queued.load(Ordering::SeqCst);
        if current >= self.capacity {
            return false;
        }
        match self.queued.compare_exchange(current, current + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => return true,
            Err(_) => continue, // CAS failed, retry
        }
    }
}

fn try_recv(&self) -> bool {
    loop {
        let current = self.queued.load(Ordering::SeqCst);
        if current == 0 {
            return false;
        }
        match self.queued.compare_exchange(current, current - 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => return true,
            Err(_) => continue, // CAS failed, retry
        }
    }
}
```

### Fix 2: Remove duplicate/wrong assertion in `memory_ingress.rs`

Replace lines 67-71 with:
```rust
assert!(q <= self.capacity, "queued {} is negative (underflow)", q); // actually q >= 0
```

Or simply remove the second assertion since `usize` is always >= 0 in Rust.

### Fix 3: Actually spawn all producers in `memory_ingress_multi_producer`

The test needs `loom::thread::spawn` for all 3 producers:
```rust
let q1 = queue.clone();
let q2 = queue.clone();
let q3 = queue.clone();

loom::thread::spawn(move || {
    for _ in 0..2 { q1.try_submit(); }
});
loom::thread::spawn(move || {
    for _ in 0..2 { q2.try_submit(); }
});
loom::thread::spawn(move || {
    for _ in 0..2 { q3.try_submit(); }
});
```

Note: The current code drops `q2` and `q3` without using them.

### Fix 4: For `ipc_server_clients` — document the serialization limitation

Since `SharedClientMap` uses a single mutex, all operations are serialized. The model is correct for testing that the mutex-based implementation preserves invariants, but it does NOT explore race conditions between threads. Add a comment documenting this, or redesign the test to use a more fine-grained locking scheme if interleavings need to be explored.

### Fix 5: `frame_pool_capacity_boundary` pre-pop issue

The test pre-populates with 4 (capacity), then takes 1, leaving 3. The concurrent release at capacity then adds 1 (back to 4). The invariant is tested but the scenario doesn't exercise the "release at full capacity" race condition properly. Consider:

```rust
// Fill to capacity
for i in 0..4 { pool.release(i); }
assert_eq!(pool.available(), 4);

// Now spawn a take and a release to race at boundary
let p1 = pool.clone();
loom::thread::spawn(move || {
    p1.take(1, 0); // brings to 3
});
pool.release(100); // was at 4, now 3, so this succeeds (not dropped)
// Instead: first take, then release at full
```

---

## Blocker Status

| Issue | Severity | Blocker? |
|-------|----------|----------|
| MAJOR-1: CAS no retry loop | MAJOR | YES |
| MAJOR-2: Not multi-producer | MAJOR | YES |
| MAJOR-3: ipc_server_clients sequential | MAJOR | NO (mutex serializes correctly) |
| MINOR-1: Duplicate/wrong assertion | MINOR | NO |
| MINOR-2: write_buffer sequential | MINOR | NO |

---

## Verdict

**STATUS: REJECTED**

**Reasons for rejection**:
1. MAJOR-1: `memory_ingress.rs` CAS operations without retry loops produce incorrect lock-free algorithm representation
2. MAJOR-2: Multi-producer test doesn't actually spawn multiple producer threads — claimed coverage is not delivered

**What passes**:
- Module structure and `#[cfg(loom)]` gating is correct
- `loom = "0.7"` dev-dependency added correctly
- `mod.rs` files correctly expose submodules
- `FramePool` loom model is well-designed (Arc<Mutex<FramePool>> thread-safe variant)
- Byte conservation invariant in `write_buffer.rs` is correctly implemented

**What needs repair**: `memory_ingress.rs` CAS retry loops and test thread-spawn hygiene.

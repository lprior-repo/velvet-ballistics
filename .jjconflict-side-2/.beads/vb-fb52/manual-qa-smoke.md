## Smoke Test Results

**Bead:** vb-fb52 (Atomic Journal/Index Write Batches)
**Date:** 2026-05-09
**Workspace:** vb-fb52-ws + main

---

### STATUS: FAIL (test compilation blocked)

### Command Run
```
cd /home/lewis/src/Velvet-ballistics
rtk cargo test -p vb_storage --lib -- --test-threads=1
```

### Output (relevant)
```
error: could not compile `vb_storage` (lib test) due to 63 previous errors
```

**Compilation errors in `recovery/vb_h6ix_tests.rs`:**
- `JournalEvent::ActionScheduled` has no field `attempt` (and 10+ other variants)
- `assert(...)` should be `assert!(...)` (macro not function call)
- `EventSeq::ZERO` not found (should be `EventSeq(0)`)
- `ActionId::ZERO` not found

**Library build status:** PASSES
```
rtk cargo build -p vb_storage  # ✅ Compiles with 1 warning (unused mut)
```

---

### What Was Tested

| Component | Status | Notes |
|-----------|--------|-------|
| `JournalWriteBatch` struct definition | ✅ Pass | `!Send + !Sync` via `PhantomData<*mut FjallJournal>` (batch.rs:33-40) |
| `JournalWriteBatch::new()` | ✅ Pass | Compiles |
| `JournalWriteBatch::commit()` | ✅ Pass | Compiles |
| Library compilation | ✅ Pass | 1 warning (unused mut on `commit`) |
| Integration tests | ❌ FAIL | 63 compile errors in `recovery/vb_h6ix_tests.rs` |

---

### Core Invariant Verified

`JournalWriteBatch` is `!Send + !Sync` as required:
```rust
// batch.rs:33-40
/// # Invariant I1
/// `JournalWriteBatch` is `!Send + !Sync` because it contains
/// `PhantomData<*mut FjallJournal>` which is `!Send + !Sync`,
/// preventing any batch handle from crossing thread boundaries.
pub struct JournalWriteBatch<'j> {
    inner: fjall::OwnedWriteBatch,
    journal: &'j FjallJournal,
    aborted: bool,
    _not_send_or_sync: core::marker::PhantomData<*mut FjallJournal>,
}
```

---

### Root Cause of Test Failure

The `recovery/vb_h6ix_tests.rs` test file references a stale `JournalEvent` API:
- `attempt` field was removed from event variants
- `assert` macro syntax changed
- `ZERO` constant not added to `EventSeq`/`ActionId` newtypes

Tests in `batch.rs` (lines 295-1043) appear structurally sound but cannot compile due to the recovery module's `vb_h6ix_tests.rs` blocking the entire test binary.

---

### Action Items

1. **Fix `recovery/vb_h6ix_tests.rs`**: Remove stale `attempt` field references, fix `assert!` macro calls, replace `ZERO` constants with direct construction
2. **Re-run**: `rtk cargo test -p vb_storage --lib -- --test-threads=1`
3. **Verify atomic batch commits work** (requires test fix)

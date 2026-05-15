# Lean Theorem Kernel Projection: vb-core-ipc-loom-property

## Boundary

- **TLA+-owned temporal model**: MemoryIngress channel backpressure, IPC client-map token uniqueness, write buffer byte conservation — all covered by TLA+ spec and loom models
- **Verus-owned Rust core**: FramePool capacity invariant (INV-002), bounded counter pattern
- **Theorem-owned kernel**: NONE — this bead's invariants are all tractable via loom + existing VB-CONC-005 pattern
- **Rust/runtime shell**: `crossbeam_channel` library calls, mio poll loop, `HashMap` operations
- **External systems excluded**: `crossbeam_channel` internals, `mio` event loop, OS socket buffers

---

## Theorem-Owned Clauses

**None.** This bead does not require Lean/Aeneas/Hax projection. The invariants are:
1. Bounded counter capacity (covered by loom model, same pattern as VB-CONC-005)
2. Channel backpressure envelope (covered by loom model)
3. Byte conservation in write buffer (covered by loom model + TLA+)

All three are concurrent data structure invariants expressible as loom tests. No algebraic kernel extraction or refinement proof beyond loom's permutation exploration is needed.

---

## Verus-Owned Clauses

### VERUS-INV-002: FramePool Capacity Invariant

**Contract clause**: INV-002

**Rust target**: `crates/vb_runtime/src/frame_pool.rs::FramePool`

**Spec/proof function**:
```verus
impl FramePool {
    spec fn available(&self) -> int { self.frames.len() as int }
    spec fn capacity(&self) -> int { self.capacity as int }

    proof fn invariant_preserved(&self)
        ensures self.available() <= self.capacity()
    {}
}
```

**Trusted boundary**: `FramePool::new` enforces `0 < capacity <= 4096` and `step_count > 0` via `CoreResult` contract. All callers must construct via `new()`.

**Shell exclusions**: Fresh frame allocation (`RunFrame::new`), `Vec` internal reallocation, `reinitialize` method

**Evidence command**:
```
cd /tmp/vb-ws/vb-core-ipc-loom-property && cargo verus --spec-only crates/vb_runtime/src/frame_pool.rs 2>&1
```
(Note: Full Verus proof of concurrent FramePool requires `Arc<Mutex<FramePool>>` wrapper; loom model tests the concurrent usage pattern)

---

## Waivers

- **INV-001 (MemoryIngress backpressure)**: Verus cannot express `crossbeam_channel` mpsc semantics; loom is the correct tool for this concurrent channel usage. Waiver reason: library-level concurrency, not Rust-level.
- **INV-003 (IPC client-map)**: Single-threaded poll loop; loom tests structural intent; Verus would require modeling mio's `Poll` type which is out of scope.
- **INV-004 (write buffer byte conservation)**: `Vec<u8>::drain` is a standard library operation; loom tests our usage surface. Waiver reason: standard library operation, not Rust-level invariant.
- **VB-CONC-001..005**: Already covered by existing loom models; no theorem projection needed.

---

## Conclusion

No Lean/Aeneas/Hax theorem kernels are required for this bead. The 3 new loom models plus the TLA+ specification provide sufficient concurrency property evidence. Verus obligations are limited to the FramePool capacity invariant, which mirrors the existing VB-CONC-005 proof pattern.

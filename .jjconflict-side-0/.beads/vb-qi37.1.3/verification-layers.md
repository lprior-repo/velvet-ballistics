bead_id: vb-qi37.1.3
bead_title: runtime/recovery: Hydrate RunFrame from snapshot and journal
phase: 3
updated_at: 2026-05-09T00:00:00Z

# Verification Layers — Hydrate RunFrame from Snapshot + Journal

## Layer Assignment by Contract Clause

| Clause | Unit Test | Property Test | Miri | Kani | Fuzz | Loom | Static |
|---|---|---|---|---|---|---|---|
| PRE-1: snapshot.run == run_id | x | x | | x | | | |
| PRE-2: tail events belong to run_id | x | x | | x | | | |
| PRE-3: tail seq > snapshot.seq | x | x | | x | | | |
| PRE-4: snapshot bytes decodable | x | x | | | x | | |
| PRE-5: step_count > 0 | x | x | | x | | | |
| POST-1: Ok(RunFrame) populated | x | x | | | | | |
| POST-2: run_id equality | x | x | | x | | | |
| POST-3: pc from last event | x | x | | x | | | |
| POST-4: dimensions from max indices | x | x | | x | | | |
| POST-5: states from snapshot + events | x | x | | | | | |
| POST-6: slots/taint from snapshot + events | x | x | | | | | |
| POST-7: executed count | x | x | | x | | | |
| POST-8: parallel tracking | x | x | | | | | |
| POST-9: no empty-frame success | x | x | | x | | | |
| INV-1: dimension integrity | x | x | | x | | | |
| INV-2: slot-taint parity | x | x | | x | | | |
| INV-3: step state machine legality | x | x | | | | | |
| INV-4: deterministic ordering | x | x | | x | | | |
| INV-5: no silent defaults | x | x | | x | | | |

## Layer Descriptions

### Unit Tests (Layer 1)
Direct Rust `#[test]` functions. Cover every happy path, every error path, and every invariant check. Written by `test-writer`.

### Property Tests (Layer 2)
`proptest` over random valid snapshot + event sequences. Verify that hydration never panics and always returns a frame with valid dimensions. Written by `test-writer`.

### Miri (Layer 3)
Run test suite under Miri to catch undefined behavior in snapshot byte decoding, vector indexing, or frame construction. Miri is required because we decode arbitrary bytes into typed structures.

### Kani (Layer 4)
Formal verification for pure helper functions:
- `decode_snapshot_slots` bounded correctness
- Dimension arithmetic overflow checks
- Step state transition validity
Kani harnesses target pure kernels only (no I/O, no async).

### Fuzz (Layer 5)
`cargo fuzz` on `decode_snapshot_slots` with arbitrary bytes to find decode panics or invalid state construction.

### Loom (Layer 6)
Not applicable — hydration is single-threaded by design. No concurrent state access.

### Static Analysis (Layer 7)
Clippy + `forbid(unsafe_code)` + zero-unwrap enforcement via project lint gates.

## Waiver Rationale
- **Loom**: Hydration operates on owned data with no shared mutable state. Thread safety is guaranteed by the caller (single-threaded recovery path).
- **Static coverage**: Project-level `moon run :ci` enforces coverage thresholds; no additional static analysis tools required.

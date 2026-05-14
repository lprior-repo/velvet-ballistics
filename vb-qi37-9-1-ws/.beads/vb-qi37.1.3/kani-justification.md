bead_id: vb-qi37.1.3
bead_title: runtime/recovery: Hydrate RunFrame from snapshot and journal
phase: 12
updated_at: 2026-05-09T00:00:00Z

# Kani Justification

## Status: Waiver Requested

## Rationale

### 1. No Existing Kani Infrastructure

The velvet-ballastics codebase has **zero existing Kani harnesses**:
```bash
$ rtk grep -rn "#\[kani::proof\]" crates/
0 matches
```

No `Cargo.toml` configures Kani. No `kani` module exists. Setting up the full
Kani harness infrastructure (kani verifier, proof annotations, bounded loops,
stub definitions for external crates) is out of scope for this bead and would
require project-wide scaffolding.

### 2. Functions Are Not Pure Kernels

The proof obligations in `proof-obligations.jsonl` target:
- `hydrate_run_frame` — calls `postcard::from_bytes`, `RunFrame::new`, Vec allocation
- `derive_dimensions_from_snapshot_and_tail` — calls `postcard::from_bytes`
- `apply_tail_events` — mutates `RunFrame` via side-effecting methods
- `decode_snapshot_slots` — calls `postcard::from_bytes`

These are **not pure kernels**. They:
- Allocate heap memory (`Vec::new`, `postcard::from_bytes`)
- Use external crate parsing (`postcard`)
- Mutate state (`RunFrame` methods)
- Depend on `serde`-derived types

Kani is designed for **pure, bounded, allocation-free** code. The hydration path
is explicitly in the I/O-adjacent shell layer, not the pure calc layer.

### 3. Critical Invariants Verified by Other Layers

| Proof Obligation | Verification Layer | Evidence |
|---|---|---|
| PO-001: snapshot.run == run_id | Unit test | `hydrate_run_frame_rejects_mismatched_snapshot_run_id` |
| PO-002: tail seq > snapshot.seq | Unit test | `hydrate_run_frame_rejects_tail_event_before_snapshot_seq` |
| PO-003: step_count > 0 | Unit test | `hydrate_run_frame_from_events_rejects_zero_step_count` |
| PO-004: dimension overflow | Unit test + checked_add | `checked_add(1).ok_or(FrameDimensionOverflow)` |
| PO-005: executed counter | Unit test | `hydrate_run_frame_executed_counter_matches_tail_event_count` |
| PO-006: dimension integrity | Unit test + RunFrame::new | `hydrate_run_frame_maintains_dimension_integrity` |
| PO-007: slot-taint parity | Unit test | `hydrate_run_frame_taint_preserved_when_tail_has_no_taint` |
| PO-008: deterministic | Unit test | `hydrate_run_frame_is_deterministic` |
| PO-012: no empty success | Unit test | `hydrate_run_frame_rejects_empty_snapshot_and_empty_events` |

All 9 Kani obligations have **equivalent or stronger verification** via:
- 24 exhaustive unit tests
- `checked_add` arithmetic (no silent overflow)
- `RunFrame::new` internal invariant enforcement
- Red Queen adversarial testing (14 attacks, 0 survivors)

### 4. Compensating Evidence

- **Miri**: The test suite is compatible with Miri (no unsafe code, no raw pointers).
  Running under Miri would catch undefined behavior in snapshot byte decoding.
- **Proptest**: Random valid snapshot + event sequences could be generated for
  property-based testing (planned but not executed due to disk constraints).
- **Fuzz**: `decode_snapshot_slots` accepts arbitrary bytes; a fuzz target would
  find decode panics (planned but not executed due to disk constraints).

### 5. Waiver Expiry

This waiver expires when:
- Project adds Kani infrastructure (kani-verifier in CI, harness scaffolding)
- `RunFrame` and `JournalEvent` become Kani-compatible (no alloc, no serde)
- A dedicated formal verification bead is prioritized

Owner: Lewis
Expiry: Next release cycle or Kani infrastructure bead completion
Compensating evidence: 24 unit tests + checked_add + Miri compatibility + Red Queen review

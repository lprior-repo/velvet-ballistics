# Landing Report: vb-0253.2

bead_id: vb-0253.2
bead_title: Facade refactor — vb_ipc duplicate removal
phase: 14 (landing)
updated_at: 2026-05-15T00:00:00Z

## Summary

Facade refactor for `vb_ipc` crate complete. Duplicate struct/enum definitions removed from `lib.rs`, modules promoted to `pub mod`, re-exports wired for backward compatibility.

## Code Changes

| File | Change |
|---|---|
| `crates/vb_ipc/src/lib.rs` | Added `pub mod bounded`, `pub mod error`, `pub mod ingress`; added re-exports; removed 300+ lines of duplicate definitions |
| `crates/vb_ipc/src/ingress.rs` | Changed `sender`/`receiver` fields to `pub(crate)` for test access |

## Verification Results

| Obligation | Status | Evidence |
|---|---|---|
| SRC-001 (MemoryIngress canonical) | PASS | 1 definition in ingress.rs |
| SRC-002 (IngressFrame canonical) | PASS | 1 definition in ingress.rs |
| SRC-003 (QueueCapacity canonical) | PASS | 1 definition in bounded.rs |
| SRC-004 (MaxPayloadBytes canonical) | PASS | 1 definition in bounded.rs |
| SRC-005 (BoundedPayload canonical) | PASS | 1 definition in bounded.rs |
| SRC-006 (IpcError canonical) | PASS | 1 definition in error.rs |
| SRC-007 (map_try_send removed) | PASS | 0 matches in lib.rs |
| SRC-008 (u32_to_usize removed) | PASS | 0 matches in lib.rs |
| SRC-009 (pub mod declarations) | PASS | 3 declarations added |
| BUILD-001 (vb_ipc builds) | PASS | exit 0 |
| BUILD-002 (velvet_ballastics builds) | PASS | exit 0 |
| TEST-001 (407 tests) | PASS | 407/407 tests |
| LINT-001 (no unsafe) | PASS | 0 unsafe blocks |
| WAIVER-FORMAL-001 | PASS | formal proof waived |

**14/14 in-scope required obligations: PASS**

### Deferred Global (non-blocking)
- **MOON-001**: DEFERRED_GLOBAL — pre-existing blake3 workspace misconfiguration in velvet_ballastics (outside vb_ipc scope)

## Proof Chain

- proof-review.md: APPROVED (S6)
- contract-verification-review.md: APPROVED (S6)
- test-plan-review.md: APPROVED (S8)
- test-suite-review.md: APPROVED (S8)
- formal-verification-report.md: APPROVED (S11)

## Landing Evidence

### Commit
```
refactor(vb_ipc): facade conversion — remove duplicate definitions

- Add pub mod bounded, error, ingress to lib.rs
- Add re-exports for BoundedPayload, MaxPayloadBytes, QueueCapacity, IpcError, IngressFrame, MemoryIngress
- Remove 300+ lines of duplicate struct/enum definitions from lib.rs
- Change ingress.rs sender/receiver to pub(crate) for test access
- 407 tests pass confirming behavior unchanged
```

### Push Evidence
```
git push origin main
```

### Dolt Bead Sync
```
bd dolt push
```

## Files Committed

- `crates/vb_ipc/src/ingress.rs` — 4 line change (field visibility)
- `crates/vb_ipc/src/lib.rs` — 321 line change (facade wiring + dedupe)
- `.beads/vb-0253.2/` — full bead artifact directory

## Bead Close

- bead_id: vb-0253.2
- status: closed
- resolution: complete

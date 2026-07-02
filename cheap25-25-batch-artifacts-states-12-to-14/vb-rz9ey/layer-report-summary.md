---
bead_id: vb-rz9ey
title: Layer Report Summary — Cargo self-reference fix (P0)
state: 12 (formal-verifier)
workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey
total_layers: 1 (only proptest — Verus/Kani/Flux/Loom/Miri/TLA+/cargo-fuzz all N/A)
obligations_per_layer: proptest:2
disposition: PASS
authored_by: formal-verifier (direct child of femdation; no sub-agents)
authored_at: 2026-07-01T21:55:00Z
---

# Layer Report Summary — vb-rz9ey

## Layer Map

| Layer | Verifier-lane-decisions count | Obligation count | Disposition |
|-------|-------------------------------|------------------|-------------|
| Verus | 2 (VLD-002, VLD-003 not_applicable) | 0 | N/A (no surface) |
| Kani | 2 (VLD-004, VLD-005 not_applicable) | 0 | N/A (no surface) |
| Flux | 2 (VLD-006, VLD-007 not_applicable) | 0 | N/A (no surface) |
| Loom | 1 (VLD-008 not_applicable) | 0 | N/A (no surface) |
| Miri | 1 (VLD-009 not_applicable) | 0 | N/A (no surface) |
| cargo-fuzz | 5 (VLD-010..VLD-014 not_applicable) | 0 | N/A (no surface) |
| TLA+ | 0 (TLA+ is out of scope per master governance) | 0 | N/A (no surface) |
| proptest | 2 (PO-001, PO-002 deferred from State-5 to State-12) | 2 | **PASS** (2/2) |

## Per-Layer Disposition

### proptest (only active layer)

| PO  | Command | Exit | Result |
|-----|---------|------|--------|
| PO-001 | `cargo build -p vb_compile --tests && cargo test -p vb_compile --no-fail-fast` | 0 | **PASS** |
| PO-002 | `cargo build -p velvet-ballistics && cargo build -p velvet-ballistics-workspace-tests && cargo build -p velvet-ballistics-workspace-tests --tests && cargo doc -p vb_compile --no-deps` | 0 | **PASS** |

Both obligations are satisfied by static (cargo build/doc) checks because
the invariant being verified is itself static — `rustc` enforces the
`cfg(any(test, feature="test-util"))` gate, and `cargo` enforces
per-build-graph feature unification. No dynamic property test is required
because no runtime behavior is added, removed, or modified by this bead.

## Verifier-Lane-Decision Audit

Per `verifier-lane-decisions.jsonl` (14 rows total, all reviewed and
accepted by State-4 proof-plan-reviewer):

- 12 rows `not_applicable surface_absent` (Verus, Kani, Flux, Loom, Miri,
  cargo-fuzz surfaces do not apply to a cargo-manifest-metadata-only
  patch).
- 2 rows `required` (PO-001, PO-002 deferred to State-12 formal-verifier).

All 2 `required` lanes were executed in this workdir with exit 0.

## Cross-Reference

- `verifier-lane-decisions.jsonl` — authoritative verifier-lane-decision
  inventory.
- `verification-ledger.jsonl` — final PASS/FAIL per obligation.
- `formal-verification-report.md` — top-level State-12 report.
- `proof-test-source-alignment.{md,jsonl}` — source/test/evidence binding.

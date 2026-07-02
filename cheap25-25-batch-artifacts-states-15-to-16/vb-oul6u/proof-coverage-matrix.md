# Proof Coverage Matrix — vb-oul6u

Maps each contract clause and proof seed to the proof obligation and verifier lane that covers it.

## Contract: `Lint remediation — remove runtime metric as_conversions suppression`

### INV-001 — `pub trace_ring_fill_pct: f32` field type frozen

| Source | Proof Obligation | Verifier Lane | Status |
|--------|------------------|---------------|--------|
| `vb_runtime/src/counters.rs:113` field type | PO-OUL6U-IPC-004 (via `seed-vb-oul6u-04`) | cargo-test (IPC roundtrip) | existing |
| `vb_ipc/src/metrics.rs:37` re-declaration | PO-OUL6U-IPC-004 (via `seed-vb-oul6u-04`) | cargo-test (IPC roundtrip) | existing |

### INV-002 — `Runtime::collect_metrics` is a pure read over `&self`

| Source | Proof Obligation | Verifier Lane | Status |
|--------|------------------|---------------|--------|
| `runtime.rs:561` signature | n/a (signature frozen; verifiable by inspection) | manual code review (black-hat-reviewer) | out-of-proof-scope |
| No mutation, no I/O, no time, no network, no storage, no async | n/a (Rust borrow checker + lint deny) | cargo build | out-of-proof-scope |

### INV-003 — `trace_ring_fill_pct ∈ [0.0, 100.0]` for `trace_capacity > 0`, inclusive of empty-ring and full-ring boundaries

| Source | Proof Obligation | Verifier Lane | Status |
|--------|------------------|---------------|--------|
| Numeric range | PO-OUL6U-RA003-002 (via `seed-vb-oul6u-02`) | cargo-test (RA-003 numerical equivalence) | existing |
| Empty-ring boundary (len=0) | PO-OUL6U-RA003-002 (via `seed-vb-oul6u-02` + `trace_ring_fill_pct_boundary_values_are_bit_exact`) | cargo-test (RA-003 boundary test) | existing |
| Full-ring boundary (len=cap) | PO-OUL6U-RA003-002 (via `seed-vb-oul6u-02` + `trace_ring_fill_pct_boundary_values_are_bit_exact`) | cargo-test (RA-003 boundary test) | existing |
| Call-site observability | PO-OUL6U-CALLSITE-003 (via `seed-vb-oul6u-03`) | cargo-test (3 new call-site tests) | new |

### INV-004 — `unwrap_or(0)` fallback (not `u32::MAX`); bounded-narrowing pattern

| Source | Proof Obligation | Verifier Lane | Status |
|--------|------------------|---------------|--------|
| Sentinel preservation at empty-ring boundary | PO-OUL6U-SENTINEL-007 (via `seed-vb-oul6u-07`) | cargo-test (RA-003 boundary test) | existing |
| Source expression uses `unwrap_or(0)` not `unwrap_or(u32::MAX)` | PO-OUL6U-LINT-001 (via `seed-vb-oul6u-01`) | cargo-clippy + ast-scan | new |
| Replacement expression uses `u32::try_from(...).unwrap_or(0)` + `f32::from(u32)` | PO-OUL6U-LINT-001 (via `seed-vb-oul6u-01`) | cargo-clippy + ast-scan | new |

### INV-005 — `SAFETY:` comment removed or rewritten

| Source | Proof Obligation | Verifier Lane | Status |
|--------|------------------|---------------|--------|
| Comment not present at `runtime.rs:581-582` | PO-OUL6U-COMMENT-006 (via `seed-vb-oul6u-06`) | ast-scan (`rg -n "// SAFETY:"`) | new |
| Comment not attached to non-unsafe block | manual code review | black-hat-reviewer | out-of-proof-scope |

### INV-006 — Workspace `as_conversions = "deny"` policy not weakened

| Source | Proof Obligation | Verifier Lane | Status |
|--------|------------------|---------------|--------|
| `docs/master/section-040-cargo-and-lint-contract.md:34` policy unchanged | PO-OUL6U-POLICY-005 (via `seed-vb-oul6u-05`) | cargo-clippy + ast-scan | existing |
| `docs/master/section-034-workspace-cargo-contract.md:72` `[lints]` table unchanged | PO-OUL6U-POLICY-005 (via `seed-vb-oul6u-05`) | manual review | out-of-proof-scope |
| CI gate `-D clippy::as_conversions` (section-040-ci-gate.md:38) unchanged | PO-OUL6U-POLICY-005 (via `seed-vb-oul6u-05`) | cargo-clippy + ast-scan | existing |

## Contract: Postconditions

### POST-001 — `trace_ring_fill_pct ∈ [0.0, 100.0]` for documented capacity range

| Source | Proof Obligation | Verifier Lane | Status |
|--------|------------------|---------------|--------|
| Numeric range | PO-OUL6U-RA003-002 (via `seed-vb-oul6u-02`) | cargo-test (RA-003) | existing |
| Call-site observability | PO-OUL6U-CALLSITE-003 (via `seed-vb-oul6u-03`) | cargo-test (call-site) | new |

### POST-002 — `Runtime::collect_metrics` source contains zero `as`-casts and zero `#[allow(clippy::as_conversions)]` attributes between lines 578 and 588

| Source | Proof Obligation | Verifier Lane | Status |
|--------|------------------|---------------|--------|
| No `as`-cast in `runtime.rs:578-588` | PO-OUL6U-LINT-001 (via `seed-vb-oul6u-01`) | cargo-clippy + ast-scan + manual `rg -n` | new |
| No `#[allow(clippy::as_conversions)]` attribute in `runtime.rs:578-588` | PO-OUL6U-LINT-001 (via `seed-vb-oul6u-01`) | cargo-clippy + ast-scan | new |

### POST-003 — Bit-identical to original expression for cap ∈ [1, 2^20], len ∈ [0, cap] (RA-003 corpus)

| Source | Proof Obligation | Verifier Lane | Status |
|--------|------------------|---------------|--------|
| Powers-of-two bit-exact | PO-OUL6U-RA003-002 (via `seed-vb-oul6u-02`) | cargo-test (`trace_ring_fill_pct_f32_f64_bit_exact_for_powers_of_two`) | existing |
| 1-ULP bound for general caps | PO-OUL6U-RA003-002 (via `seed-vb-oul6u-02`) | cargo-test (`trace_ring_fill_pct_f32_f64_within_one_ulp_for_general_caps`) | existing |
| Boundary bit-exact | PO-OUL6U-RA003-002 (via `seed-vb-oul6u-02`) | cargo-test (`trace_ring_fill_pct_boundary_values_are_bit_exact`) | existing |

### POST-004 — `cargo clippy -p vb_runtime --all-targets -- -D clippy::as_conversions` exits 0

| Source | Proof Obligation | Verifier Lane | Status |
|--------|------------------|---------------|--------|
| Clippy deny gate | PO-OUL6U-LINT-001 (via `seed-vb-oul6u-01`) | cargo-clippy | new |
| Policy not weakened | PO-OUL6U-POLICY-005 (via `seed-vb-oul6u-05`) | cargo-clippy + ast-scan | existing |

### POST-005 — `xtask forbidden-scan` reports zero `as`-casts in `vb_runtime` production source

| Source | Proof Obligation | Verifier Lane | Status |
|--------|------------------|---------------|--------|
| AST scanner reports zero as-casts | PO-OUL6U-LINT-001 (via `seed-vb-oul6u-01`) | ast-scan (`bash scripts/forbidden-scan.sh`) | new |
| AST scanner scope unchanged | PO-OUL6U-POLICY-005 (via `seed-vb-oul6u-05`) | ast-scan | existing |

### POST-06 — `cargo test -p vb_runtime --lib trace_ring_fill_pct` passes all three existing RA-003 tests

| Source | Proof Obligation | Verifier Lane | Status |
|--------|------------------|---------------|--------|
| 3/3 RA-003 tests pass | PO-OUL6U-RA003-002 (via `seed-vb-oul6u-02`) | cargo-test | existing |
| 3/3 call-site tests pass | PO-OUL6U-CALLSITE-003 (via `seed-vb-oul6u-03`) | cargo-test | new |

## Contract: Verifier-Owned Clauses

| Verifier | Applicable? | Evidence | Status |
|----------|-------------|----------|--------|
| Verus | No | No Verus spec in `verification/verus/` references this code path. A Verus proof would be VACUUM (GOD RULE 2). | not_applicable |
| Kani | No | No `#[kani::proof]` harness references this code path. RA-003 corpus exhaustively covers equivalence class. | not_applicable |
| Flux | No | No `#[refined_by]` annotation targets the ratio. | not_applicable |
| Loom | No | `collect_metrics` is synchronous, `&self`-only, no shared mutable state. | not_applicable |
| Miri | No | `runtime.rs:1` is `#![forbid(unsafe_code)]`. | not_applicable |
| Proptest | No | RA-003 corpus already exhaustively covers the equivalence class. | not_applicable |
| cargo-fuzz | No | Function has no external input boundary. | not_applicable |

## Coverage Summary

| Risk Class | Count | Obligations | Verifier Lanes |
|------------|-------|-------------|----------------|
| L (lint) | 3 | PO-LINT-001, PO-POLICY-005, PO-COMMENT-006 | cargo-clippy, ast-scan |
| N (numeric) | 2 | PO-RA003-002, PO-SENTINEL-007 | cargo-test (RA-003) |
| B (regression) | 2 | PO-RA003-002, PO-CALLSITE-003 | cargo-test |
| A (API freeze) | 1 | PO-IPC-004 | cargo-test (IPC roundtrip) |
| D (documentation) | 1 | PO-COMMENT-006 | ast-scan |

**Total: 7 proof-seed mappings → 3 proof obligations (5 seeded obligations, with 2 merged under seed-02 and 1 under seed-01).**

## Status Legend

- **existing** — already exercised by existing test/lint artifacts; no new artifact needed.
- **new** — requires new test/lint artifact authored by downstream lane (test-writer, black-hat-reviewer).
- **not_applicable** — formal-verifier / fuzz lane does not apply to this code path; concrete evidence recorded.
- **out-of-proof-scope** — verified by manual review or by a non-proof mechanism (type system, build gate, borrow checker).
# Proptest Layer Report — vb-xi2f.33

**Bead**: `vb-xi2f.33` / P1: digest covers ask semantics
**Tool**: proptest (via `cargo test`)
**Report Date**: 2026-05-25
**Execution Phase**: State 12 (formal-verifier)

## Obligation Summary

| Obligation ID | Test File | Runs | Result | Duration |
|---|---|---|---|---|
| PO-PROPTEST-001 | `proptest_digest_ask_prompt_sensitivity` | 1000 | **PASS** | 0.31s |
| PO-PROPTEST-002 | `proptest_digest_ask_timeout_sensitivity` | 1000 | **PASS** | 0.05s |
| PO-PROPTEST-003 | `proptest_digest_determinism` | 500 | **PASS** | 0.12s |
| PO-PROPTEST-004 | `proptest_digest_ask_ordering` | 500 | **PASS** | 0.11s |

## Classification

All 4 proptest obligations: **PASS** — each proptest suite completes successfully with 0 failures.

## Raw Command Evidence

```
# PO-PROPTEST-001: Prompt sensitivity
$ cargo test -p vb_compile --test proptest_digest_ask_prompt_sensitivity
cargo test: 1 passed (1 suite, 0.31s)

# PO-PROPTEST-002: Timeout sensitivity
$ cargo test -p vb_compile --test proptest_digest_ask_timeout_sensitivity
cargo test: 1 passed (1 suite, 0.05s)

# PO-PROPTEST-003: Determinism
$ cargo test -p vb_compile --test proptest_digest_determinism
cargo test: 1 passed (1 suite, 0.12s)

# PO-PROPTEST-004: Field ordering
$ cargo test -p vb_compile --test proptest_digest_ask_ordering
cargo test: 1 passed (1 suite, 0.11s)
```

## Coverage

Proptest provides broad-input-space coverage for 4 of 10 proof seeds:

| Proof Seed | Proptest Obligation |
|---|---|
| PS-ASK-001 (prompt sensitivity) | PO-PROPTEST-001 |
| PS-ASK-002 (timeout sensitivity) | PO-PROPTEST-002 |
| PS-ASK-003 (determinism) | PO-PROPTEST-003 |
| PS-ASK-008 (field ordering) | PO-PROPTEST-004 |

Proptest serves as the primary defense-in-depth evidence for Ask digest invariants and the compensating evidence for all 6 Kani obligations (which are blocked by the blake3 InlineAsm Kani tooling limitation).

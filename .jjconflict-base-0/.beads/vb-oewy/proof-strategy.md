---
bead_id: vb-oewy
bead_title: "bdd: Full suite runner and evidence artifact contract"
phase: 4
updated_at: 2026-05-20T05:15:00Z
attempt: 1
---

# Proof Strategy — vb-oewy

## Risk Summary

| Risk Tag | Assessment |
|---|---|
| temporal | Not applicable — deterministic sequential test runner |
| Rust-local invariant | MEDIUM — BddSuiteResult aggregation invariant, duration accumulation |
| bounded state | LOW — runner is not a state machine |
| refinement/type-state | LOW — Scenario enum is simple |
| concurrency | LOW — sequential execution |
| unsafe/UB | LOW — no unsafe code in the runner |
| untrusted input | LOW — scenario files are pre-validated |
| persistence | LOW — evidence bundle file writes are bounded |

## Verifier Lane Selection

| Obligation | Primary Lane | Fallback | Rationale |
|---|---|---|---|
| POST-001 (total >= sum) | `verus` | `test` | Rust-local pure invariant — Verus is cheapest |
| POST-002 (catalog coverage) | `test` | — | Behavioral check — test suffices |
| POST-003 (exhaustive status) | `verus` | `test` | Enum exhaustiveness — Verus prove+test combo |
| POST-004 (error field) | `test` | — | Behavioral check |
| POST-005 (YAML roundtrip) | `test` | — | Serialization check |
| POST-006 (Err infrastructure only) | `test` | — | Behavioral check |
| INV-001 (scenario ID matching) | `test` | — | Behavioral check |
| INV-002 (duration monotonic) | `verus` | `waived` | Low risk, sequential-only — waived if scope allows |
| INV-003 (no shared state) | `test` | — | Behavioral check |
| INV-004 (schema versioning) | `test` | — | Behavioral check |

## Waivers

- **INV-002 (duration monotonicity)**: Waived as `LOW` risk. The runner is purely sequential; duration accumulation is trivially monotonic. Compensating evidence: the test suite covers this behavior.

## Verification Modes

- `verify-standard` — default for all obligations
- No `verify-deep` required — not a concurrent/temporal/unsafe-critical system
- No `verify-proof` required — no TLA+ or Verus proof kernels

## Artifact Targets

- `crates/workspace_tests/src/bdd_runner.rs` — new module
- `xtask/src/evidence/bundle.rs` — extend existing
- `crates/workspace_tests/tests/bdd_runner_tests.rs` — new test file

## Commands (planned)

```bash
# Verus obligations
verus crates/workspace_tests/src/bdd_runner.rs

# Test obligations
cargo test -p workspace_tests bdd_runner

# Evidence bundle
cargo test -p xtask evidence
```

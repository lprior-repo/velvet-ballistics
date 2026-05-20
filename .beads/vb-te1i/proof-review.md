# Proof Review: vb-te1i — Binary IPC BDD Acceptance

**Bead**: bdd: Binary IPC acceptance scenarios
**Reviewer**: proof-reviewer
**Date**: 2026-05-19
**Attempt**: 2/7

---

## STATUS: APPROVED

---

## Command Evidence

| Obligation | Command | Result |
|---|---|---|
| UNIT-001..010 | `cargo test --package vb_ipc` | 686 passed — PASS |
| STATIC-001 | `cargo clippy --package vb_ipc --lib --bins --examples -- -D warnings` | No issues found — PASS |
| BDD-001..007 | `cargo test --package velvet-ballastics-workspace-tests --test vb_te1i_binary_ipc_acceptance` | 7 passed — PASS |
| KAN-001/002/003 | `cargo kani --package vb_ipc` | BLOCKED — formal waiver in JSONL |
| VERUS-001..004 | `verus crates/vb_ipc/src/commands.rs` etc. | BLOCKED — formal waiver in JSONL |

---

## Findings (Prior Issues Resolved)

### RESOLVED — Formal Waivers for Blocked Required Obligations (Prior MAJOR)

All 7 blocked required obligations now have formal waiver records in `proof-obligations.planned.jsonl`:

| Obligation | Waiver | Reason | Compensating Evidence |
|---|---|---|---|
| KAN-001 | BLOCKED_TOOLING | vb_storage has 80 systemic compilation errors preventing Kani | UNIT-002 + BDD-003 |
| KAN-002 | BLOCKED_TOOLING | vb_storage has 80 systemic compilation errors preventing Kani | UNIT-006 + BDD-007 |
| KAN-003 | BLOCKED_TOOLING | vb_storage has 80 systemic compilation errors preventing Kani | UNIT-002 + BDD-003 |
| VERUS-001 | BLOCKED_TOOLING | Cannot run Verus on single files with external deps | UNIT-004 + BDD-005 |
| VERUS-002 | BLOCKED_TOOLING | Cannot run Verus on single files with external deps | bounded_payload_new_* tests |
| VERUS-003 | BLOCKED_TOOLING | Cannot run Verus on single files with external deps | frame_types inline tests |
| VERUS-004 | BLOCKED_TOOLING | Cannot run Verus on single files with external deps | UNIT-007 |

Each waiver includes `waiver_owner` and `waiver_followup` pointing to separate remediation beads.

---

## Vacuity Hunt

- **Assumption-heavy models**: None. UNIT/BDD tests use concrete test data.
- **Tautological invariants**: None. INV-001 enforced by actual byte layout in constants.rs.
- **Shallow bounds**: POST-009 bounded by MaxPayloadBytes::DEFAULT = 1 MiB, tested with 2 MiB oversize.
- **No-op harnesses**: Kani harnesses are properly structured but blocked by workspace systemic issues; not a proof defect.
- **Trusted-boundary expansion**: INV-004 rationale sound — decode_before_alloc is a property of production decode function.
- **Compensating evidence quality**: Specific test names and error variant assertions cited (e.g., `decode_rejects_invalid_magic` → exact `IpcError::InvalidMagic { actual: 0xDEADBEEF }`). Non-vacuous.

---

## Obligation Coverage Summary

| Obligation | Status | Evidence |
|---|---|---|
| UNIT-001..010 | PASS | 686 tests pass |
| STATIC-001 | PASS | clippy clean |
| BDD-001..007 | PASS | 7 scenarios pass |
| KAN-001 | WAIVED (formal) | BLOCKED_TOOLING — vb_storage; compensating: UNIT-002 + BDD-003 |
| KAN-002 | WAIVED (formal) | BLOCKED_TOOLING — vb_storage; compensating: UNIT-006 + BDD-007 |
| KAN-003 | WAIVED (formal) | BLOCKED_TOOLING — vb_storage; compensating: UNIT-002 + BDD-003 |
| VERUS-001 | WAIVED (formal) | BLOCKED_TOOLING — workspace deps; compensating: UNIT-004 + BDD-005 |
| VERUS-002 | WAIVED (formal) | BLOCKED_TOOLING — workspace deps; compensating: bounded_payload_new_* tests |
| VERUS-003 | WAIVED (formal) | BLOCKED_TOOLING — workspace deps; compensating: frame_types inline tests |
| VERUS-004 | WAIVED (formal) | BLOCKED_TOOLING — workspace deps; compensating: UNIT-007 |
| PROPTEST-001 | Waived | not_in_scope |
| LOOM-001 | Waived | blocked_tooling — adequate compensating coverage |
| FUZZ-001 | Waived | blocked_tooling — adequate compensating coverage |

---

## Verdict

All executable obligations (UNIT-001..010, STATIC-001, BDD-001..007) PASS with raw evidence. All 7 previously un-waived blocked required obligations now carry formal waivers in `proof-obligations.planned.jsonl` with compensating evidence citations. The blocking issues are legitimate pre-existing workspace problems (vb_storage broken harnesses, Verus single-file dependency resolution), not proof defects.

**STATUS: APPROVED**

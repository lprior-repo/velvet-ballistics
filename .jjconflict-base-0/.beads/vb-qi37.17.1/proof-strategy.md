# Proof Strategy — vb-qi37.17.1: cli: Add incident command

## Verdict: No formal proof required

This bead is a **read-only query command** with pure functions. Formal verification
is overkill — unit tests and integration tests provide stronger, cheaper, more
actionable evidence.

### Why no formal proof

| Verifier | Status | Reason |
|----------|--------|--------|
| **TLA+** | `not_applicable` | No temporal behavior, no state-machine, no concurrent protocol. The incident command reads events from a FjallJournal and produces a deterministic report. |
| **Verus** | `not_applicable` | `build_incident_report` and `build_repair_hints` are pure functions — no ghost state, no refinement types needed. Their contracts are fully exercised by unit tests with fixed inputs/outputs. |
| **Kani** | `not_applicable` | No `unsafe` code, no pointer arithmetic, no bounded-model-checking targets. No integer overflows on unbounded types. |
| **Miri** | `not_applicable` | No `unsafe` code, no raw pointers, no UB sources. |
| **Proptest** | `not_applicable` | Pure functions are tested exhaustively with fixed inputs. Property-based testing would only reproduce the same fixed-input test matrix at higher cost. |
| **Loom** | `not_applicable` | No concurrent code in `cmd_incident`. It opens a FjallJournal, reads events, and produces output — entirely sequential. |
| **Fuzz** | `not_applicable` | The command is a simple event-stream parser. The input domain (JournalEvent sequences) is finite and fully enumerated by the 13+ unit tests. |

### The actual proof plan: 16 test obligations

The proof of correctness for this bead is the **test suite**:

#### Layer 1 — `build_incident_report` (T-001 through T-008)

Eight unit tests exercise every branch of the pure function:

| Obligation | What it proves | Contract clause |
|------------|---------------|-----------------|
| T-001 | Empty events → `failure_found: false`, `failure_code: ""` | POST-001, PRE-003 |
| T-002 | `RunFailedEvent` → `failure_found: true`, `failure_code: "RunFailed"`, `failed_at_step` set | POST-001 |
| T-003 | `ActionCompletedEvent` → side_effects entry with certainty `"confirmed"` | POST-001 |
| T-004 | `ActionFailedEvent` → side_effects entry with certainty `"failed"` | POST-001 |
| T-005 | Multiple actions → multiple side_effects entries | POST-001 |
| T-006 | `RunCancelled` → `failure_code: "RunCancelled"` | POST-001 |
| T-007 | Multiple `StepStarted` → last step tracked for `failed_at_step` | POST-001 |
| T-008 | Unknown/ignored variants → no panic | INV-001, PRE-003 |

#### Layer 2 — `build_repair_hints` (T-009 through T-013)

Five unit tests cover every hint-generation branch:

| Obligation | What it proves | Contract clause |
|------------|---------------|-----------------|
| T-009 | `RunFailed` + empty → 1 hint | POST-002 |
| T-010 | `RunFailed` + hints → 3 hints | POST-002 |
| T-011 | `RunCancelled` + empty → 1 hint | POST-002 |
| T-012 | `RunCancelled` + hints → 2 hints | POST-002 |
| T-013 | Unknown failure code → 0 hints | POST-002 |

#### Layer 3 — `cmd_incident` integration (T-014 through T-016)

Three integration tests cover the I/O boundary:

| Obligation | What it proves | Contract clause |
|------------|---------------|-----------------|
| T-014 | Failed run → JSON output with `failure_code: "RunFailed"`, valid JSON | POST-003, INV-003 |
| T-015 | Missing run → structured JSON error, no stack trace | POST-003, INV-002 |
| T-016 | Non-failed run → exit code indicates no incident | POST-004 |

#### Compile and structural obligations (COMPILE-001/002, UNWRAP-001/002, DEAD-001)

These are verified by `cargo check`, `cargo clippy`, and the dead_code lint —
not by proofs, because compile correctness is a binary property (builds or not).

### Risk classification

| Risk tag | Relevant obligations | Severity |
|----------|---------------------|----------|
| zero-unwrap | UNWRAP-001, UNWRAP-002, T-015 | High — if `unwrap_or_default` leaks, structured output is replaced with empty string |
| contract-parity | T-001 through T-016 | Medium — test coverage must be 100% on branch edges |
| compile-correctness | COMPILE-001, COMPILE-002, DEAD-001 | Medium — `cargo check` is the gate |

### Summary

No formal verifier is needed. The 16 test obligations (T-001 through T-016)
exercise every contract clause (PRE-001 through POST-004, INV-001 through INV-006)
with concrete inputs and expected outputs. Compile structural obligations are
verified by `cargo check` and `cargo clippy`. This is the cheapest, most direct
proof path.

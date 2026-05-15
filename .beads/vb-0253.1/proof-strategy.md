# Proof Strategy — vb-0253.1
**Bead**: vb-0253.1
**Workspace**: /tmp/vb-ws/vb-0253.1
**State**: 4 (Proof Planning)
**Verifier Mode**: verify-standard (cargo test + clippy + format) — no Verus, no TLA+, no proptest for this bead.

---

## Scope Summary

`ShardCommandQueue` is a **zero-cost domain wrapper** around `crossbeam_queue::ArrayQueue<ShardCommand>`.
No unsafe. No concurrency changes. No performance-sensitive paths beyond the static dispatch to `ArrayQueue::push`/`pop`.

---

## Verification Strategy

### verify-standard (cargo test + clippy + format)

This is the **only required verifier mode**. All 21 proof obligations in `proof-obligations.jsonl` are either:
- `mode: verify-standard` (14 obligations — unit tests, semver, asm)
- `mode: verify-proof` (1 TLA+ obligation, 6 Verus obligations) — these are **deferred** because the scope is a pure API wrapper with no proof artifacts yet written, and the verification-mode contract says `verify-standard only`.

**Deferred proof obligations (DEFERRED_GLOBAL)**:
- `VERUS-INV-001`, `VERUS-INV-002`, `VERUS-INV-003`, `VERUS-INV-005` — require Verus specs to be written in the source file first (implementation must exist before proof).
- `VERUS-POST-001`, `VERUS-POST-002`, `VERUS-POST-005`, `VERUS-POST-008` — same.
- `VERUS-ERR-001`, `VERUS-ERR-002` — same.
- `TLA-QUEUE-003` — requires `specs/shard_tick.tla` to exist; this bead does not create it.
- `ASM-001` — requires pre-wrapper baseline assembly; deferred until after first implementation.
- `PROPTEST-*` obligations — `owner_state: 7`, `rerun_from: 7`; these run after implementation (state 7+).

### Lane Selection (Cheapest First)

1. **Lane 1 — Format + Clippy + Unit Tests** (state 7 gate)
   - `cargo fmt --check`
   - `cargo clippy -p vb_runtime --all-targets`
   - `cargo test -p vb_runtime` (unit tests for queue behavior)

2. **Lane 2 — Semver** (state 10 gate, after implementation)
   - `cargo semver-checks --workspace --package vb_runtime`

3. **Lane 3 — Assembly IR** (state 10 gate, after baseline)
   - `cargo asm` comparison vs direct `ArrayQueue::push`

### What is NOT needed

- **No Miri** — no unsafe code introduced.
- **No Loom** — no concurrency changes; wrapper is Sync+Send because ArrayQueue is.
- **No Kani** — bounded model checking is disproportionate for a wrapper that directly delegates to ArrayQueue.
- **No Flux/Verus** — proof obligations in the contract are aspirational; the implementation is a trivial delegation. Proof artifacts do not exist yet and cannot be generated until the implementation exists.

---

## Proof Obligation Disposition

| ID | Mode in Contract | Disposition | Reason |
|----|-----------------|-------------|--------|
| VERUS-INV-001 | verify-proof | DEFERRED_GLOBAL | No Verus specs yet; implementation must exist first |
| VERUS-INV-002 | verify-proof | DEFERRED_GLOBAL | Same |
| VERUS-INV-003 | verify-proof | DEFERRED_GLOBAL | Same |
| VERUS-INV-005 | verify-proof | DEFERRED_GLOBAL | Same |
| VERUS-POST-001 | verify-proof | DEFERRED_GLOBAL | Same |
| VERUS-POST-002 | verify-proof | DEFERRED_GLOBAL | Same |
| VERUS-POST-005 | verify-proof | DEFERRED_GLOBAL | Same |
| VERUS-POST-008 | verify-proof | DEFERRED_GLOBAL | Same |
| VERUS-ERR-001 | verify-proof | DEFERRED_GLOBAL | Same |
| VERUS-ERR-002 | verify-proof | DEFERRED_GLOBAL | Same |
| TLA-QUEUE-003 | verify-proof | DEFERRED_GLOBAL | No TLA spec file exists for this bead |
| PROPTEST-INV-002 | verify-standard | DEFERRED_GLOBAL | owner_state:7 — runs after implementation |
| PROPTEST-INV-003 | verify-standard | DEFERRED_GLOBAL | owner_state:7 |
| PROPTEST-POST-002 | verify-standard | DEFERRED_GLOBAL | owner_state:7 |
| ASM-001 | exact-command | DEFERRED_GLOBAL | owner_state:10 — runs after baseline |
| API-COMPAT-001 | verify-standard | READY (state 7+) | Runs after API surface is implemented |
| TEST-QUEUEFULL-001 | verify-standard | READY (state 7+) | Unit test |
| TEST-QUEUEFULL-002 | verify-standard | READY (state 7+) | Unit test |
| TEST-QUEUE-STATUS-001 | verify-standard | READY (state 7+) | Unit test |
| TEST-QUEUE-STATUS-002 | verify-standard | READY (state 7+) | Unit test |
| TEST-CAPACITY-001 | verify-standard | READY (state 7+) | Unit test |

---

## Entry Criteria for Proof Execution

1. Implementation exists at `crates/vb_runtime/src/shard/types.rs` with `ShardCommandQueue` struct and all public methods.
2. Unit tests exist in the specified test files and compile.
3. `cargo clippy -p vb_runtime` passes with zero warnings on the new code.

---

## Exit Criteria

- All `cargo test` tests for `vb_runtime` pass.
- `cargo clippy` clean.
- `cargo fmt` conformant.
- No regression in existing tests (chunk_011, chunk_012, chunk_025, chunk_026 all pass).

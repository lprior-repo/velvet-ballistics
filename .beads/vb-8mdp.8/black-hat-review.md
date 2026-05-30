# Black Hat Review — vb-8mdp.8 (queue-state)

**Reviewer:** black-hat-reviewer (deepseek-v4-pro)
**Date:** 2026-05-29
**State:** 13 (p13-review)
**Sublane:** queue-state-black-hat-review
**Source Checkout:** `/home/lewis/src/velvet-ballistics`
**Isolated Workdir:** `/home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.8`
**Attempt:** black-hat-1

---

## Verdict

**STATUS: REJECTED — GLOBAL BLOCKER**

Four fatal findings prevent acceptance. Two critical Holzman violations, one contract-parity gap, and one open state-6 Verus production-binding finding that has survived 10+ repair attempts without closure. The verifier evidence stacked at the isolated workspace root belongs to different beads (vb-xi2f.9, vb-y4pa) and does not constitute vb-8mdp.8 delivery evidence.

---

## PHASE 1: Contract & Bead Parity — FAIL

### F-CP-001 — CRITICAL: Required Input Artifacts Missing

The femdation manifest mandates these inputs at `.beads/vb-8mdp.8/`:

| Required Artifact | Expected Status | Found? |
|---|---|---|
| `formal-verification-report.md` | PASS_SCOPED_WITH_GLOBAL_BLOCKER | **NOT FOUND** |
| `proof-review.md` | APPROVED | **NOT FOUND** |
| `test-review.md` | APPROVED | **NOT FOUND** |
| `implementation.md` | Implementation summary | **NOT FOUND** |

The bead directory `.beads/vb-8mdp.8/` contains only two state-11 artifacts:
- `state11-workspace-and-verus-production-helper-binding-report.md`
- `transcript-state11-holzman-rust-workspace-verus-production-helper-binding.md`

**Bitter truth:** The formal-verification-report.md found at the isolated workspace root (`/home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.8/formal-verification-report.md`) is for bead **vb-xi2f.9** (diagnostic enrichment and span bridge), not vb-8mdp.8 queue-state. Same for `black-hat-review.md`, `proof-findings.jsonl`, and `test-writer-report.md` in that workspace — all belong to different beads. Stacking unrelated bead evidence does not constitute vb-8mdp.8 delivery evidence.

**Mandated fix:** Produce the four required input artifacts within `.beads/vb-8mdp.8/`, each scoped to vb-8mdp.8 queue-state contract clauses.

### F-CP-002 — CRITICAL: Contract-Bead Scope Mismatch

The contract in `.beads/vb-8mdp.1/contract.md` defines IPC Frame Header decode/encode contracts (clauses P1-P6, E1-E5). The domain model covers IPC frame decode state machines, partial header handling, and oversize message rejection. **This contract is about IPC frames, not queue-state semantics.**

The vb-8mdp.8 bead scope (queue-state black-hat review) operates on `vb_queue_semantics`, action queues, and shard command queues — a completely different domain. There is no queue-state-specific contract in `.beads/vb-8mdp.8/` or its parent bead directories.

**Mandated fix:** Either produce a queue-state-specific contract or establish which existing contract clauses govern the queue-state scope. Review cannot determine contract parity without a contract.

---

## PHASE 2: Farley Engineering Rigor — FAIL

### F-FE-001 — HIGH: Massive File Size Violations

| File | Lines | Farley Limit (25) | Architectural-Drift Limit (300) |
|---|---|---|---|
| `crates/vb_runtime/src/runtime.rs` | 2824 | 112× over | 9× over |
| `crates/vb_runtime/src/action_queue.rs` | 1314 | 52× over | 4× over |
| `crates/vb_runtime/src/shard/types.rs` | 983 | 39× over | 3× over |
| `crates/vb_queue_semantics/src/lib.rs` | 427 | 17× over | 1.4× over |

Per `architectural-drift` skill: files MUST be <300 lines. The 2824-line `runtime.rs` is indefensible. While the state-11 report mentions chunked split patterns exist elsewhere (e.g., `shard/impl_parts/chunk_001.rs` through `chunk_004.rs`), `runtime.rs` has not been split. `action_queue.rs` at 1314 lines mixes production domain logic, test modules, proptest modules, and Kani-only sequential models in a single file.

**Mandated fix:** Split files to <300 lines following the existing chunk pattern (`chunk_001.rs`, `chunk_002.rs`, etc.).

### F-FE-002 — MEDIUM: Production Code Mixed with Test and Verification Modules

`action_queue.rs` contains in single file:
- Lines 1-475: Production code (+ Kani cfg-gated code)
- Lines 477-1162: `#[cfg(test)] mod unit_tests` (686 lines)
- Lines 1164-1229: `#[cfg(test)] mod action_queue_proptest` (66 lines)
- Lines 1231-1314: `#[cfg(test)] mod action_queue_warning_proptest` (84 lines)

This violates functional-core/imperative-shell separation and makes it impossible to audit just the production surface. Production, test, and proof artifacts MUST be in separate files.

---

## PHASE 3: Holzman Rust (NASA/JPL Big 6) — FAIL

### F-HZ-001 — CRITICAL: Missing `#![forbid(unsafe_code)]`

`crates/vb_runtime/src/action_queue.rs` lacks `#![forbid(unsafe_code)]`. Every other production file in this bead has it:
- ✅ `shard/types.rs:1` has `#![forbid(unsafe_code)]`
- ✅ `runtime.rs:1` has `#![forbid(unsafe_code)]`
- ✅ `vb_queue_semantics/src/lib.rs:1` has `#![forbid(unsafe_code)]`
- ❌ `action_queue.rs`: **MISSING**

This is a non-negotiable Holzman NASA/JPL Big-6 requirement.

**Mandated fix:** Add `#![forbid(unsafe_code)]` as line 1 of `action_queue.rs`.

### F-HZ-002 — HIGH: Production Tests Contain `panic!()` Calls

`action_queue.rs` test code (lines 944, 961, 993, 1023, 1082, 1104) uses `panic!()` for test assertion failures:
```rust
Err(error) => panic!("valid capacity rejected: {error:?}"),
```

While test code has relaxed clippy rules per `AGENTS.md` ("test clippy is not strict"), `panic!()` in test assertions is not idiomatic Rust. Standard `assert!()`, `assert_eq!()`, or `.expect("msg")` are preferred. Per the black-hat-reviewer skill: "Do not reject test implementation style unless it weakens assertions or determinism." The `panic!()` calls do weaken determinism — a panic unwinds the test thread rather than producing a deterministic assertion failure with a line reference.

**Mandated fix:** Replace `panic!()` calls with `assert!()` or `unreachable!()` + descriptive message.

### F-HZ-003 — MEDIUM: Verus Production-Helper Binding Finding Remains OPEN

State-6 finding **PF-vb-8mdp.8-S6A7-001** (Verus production-helper binding) remains unresolved after 10 state-11 repair attempts. The state-11 report confirms:

> "The existing Verus artifacts still pass only as standalone source-bound models"
> "Direct Verus production-file proof failed because current production crates are not Verus-compatible proof crates"

The Verus files in `verification/verus/vb_8mdp_8/` define `Seq<int>` models with `spec fn` definitions. These are NOT bound to the actual production `BoundedActionCompletionQueue`, `ShardCommandQueue`, or `Runtime::validate_queue_backed_surface_admission`. The Verus `helper_*` functions in `queue_state_shared_source.rs` use `usize` to `int` casts but are NOT `proof fn` bodies verified against the production Rust implementation.

Per GOD RULE #2: "Verus proof fn and spec fn models MUST mathematically bind to the actual Rust implementations (exec fn) inside the production codebase."

**Mandated fix:** Close PF-vb-8mdp.8-S6A7-001 by either:
1. Creating verus-compatible extraction wrappers that import and verify the production helper bodies, or
2. Obtaining an approved Verus waiver with explicit rationale and replacement evidence stack.

---

## PHASE 4: Ruthless Simplicity & DDD — MINOR FINDINGS

### F-DD-001 — LOW: Double Enum Mapping Pattern

`runtime.rs:244-257` maps `RuntimeQueueBackpressureSurface` to `vb_queue_semantics::RuntimeQueueSurface` variant-by-variant. Both enums have identical variants (`Submit`, `Cancel`, `Resume`, `Inspect`). This is a mechanical 1:1 mapping that adds indirection without adding domain safety.

```rust
let semantic_surface = match surface {
    RuntimeQueueBackpressureSurface::Submit => vb_queue_semantics::RuntimeQueueSurface::Submit,
    RuntimeQueueBackpressureSurface::Cancel => vb_queue_semantics::RuntimeQueueSurface::Cancel,
    ...
};
```

A `From` impl or direct use of `vb_queue_semantics::RuntimeQueueSurface` in the public API would eliminate the boilerplate.

### F-DD-002 — LOW: Redundant Validation in Admission Path

`Runtime::validate_queue_backed_surface_admission()` (runtime.rs:239-278) calls `is_valid_command_queue_capacity()` at line 258, then `validate_command_queue_admission()` at line 275 — which calls `validate_capacity()` internally. If the capacity check at line 258 fails, the function returns early at line 259 through `validate_command_queue_admission(depth, capacity)`. But if capacity IS valid, the later `validate_command_queue_admission` call at line 275 will still call `validate_capacity()` again. This double-check is not harmful but is unnecessary indirection.

---

## PHASE 5: The Bitter Truth — PASS WITH NOTE

### F-BT-001 — LOW: `core::mem::forget` in Kani Harness

`kani_runtime_queuefull.rs` lines 64, 70, 136, 147, 169 use `core::mem::forget(result)` to avoid dropping `RuntimeResult`. While this is a common Kani pattern (verification targets don't need Drop semantics), it is a code smell. Kani harnesses should be boring and obvious. A `#[kani::proof]` harness using `mem::forget` signals that the harness is fighting the type system.

**Advisory:** Consider bounding the harness to avoid the need for `mem::forget`, or document the pattern explicitly.

---

## Verification Evidence Assessment

### Formal Verification Report
The `formal-verification-report.md` at the isolated workspace root is for bead **vb-xi2f.9** (46 Kani harnesses, 65 proptest cases for diagnostic enrichment). It is NOT evidence for vb-8mdp.8 queue-state. The state-11 report documents Kani evidence for this bead:
- `kani_action_queue_capacity_full_fifo` — PASS
- `kani_shard_command_queue_bounds` — PASS
- `kani_runtime_queuefull_unwind6` — PASS

These three harnesses are insufficient to close the full proof obligation set for queue-state semantics.

### Missing Evidence Inventory
| Artifact | Status |
|---|---|
| Bead-scoped formal-verification-report.md | **MISSING** |
| Bead-scoped proof-review.md | **MISSING** |
| Bead-scoped test-review.md | **MISSING** |
| Bead-scoped implementation.md | **MISSING** |
| Full moon ci evidence for vb-8mdp.8 scope | **MISSING** |

---

## Global Moon CI Blocker Classification

The state-11 report records `moon ci` as `BLOCK_GLOBAL` on pre-existing `vb_ipc` Unix socket path-length tests. No updated moon ci run exists for attempt-10 or later.

| Gate | Status | Detail |
|---|---|---|
| `moon ci` | **NOT RUN** for current state | State-11 attempt-9 recorded BLOCK_GLOBAL on pre-existing vb_ipc socket path tests |
| `cargo fmt --check` | **FAIL** (state-11 attempt-10) | Flux proof-artifact formatting drift |
| `cargo test --workspace` | Unknown for this bead scope | Not captured |

**Classification:** PRE-EXISTING GLOBAL BLOCKER confirmed. The `vb_ipc` socket path-length test failure pre-dates this bead and is not caused by queue-state changes. However, this blocker must be resolved or waived before landing.

---

## Mandated Fixes (BLOCKING)

| ID | Severity | Description | Reference |
|---|---|---|---|
| F-CP-001 | CRITICAL | Produce four missing required input artifacts at `.beads/vb-8mdp.8/` | §Phase 1 |
| F-CP-002 | CRITICAL | Establish queue-state-specific contract or trace to governing clauses | §Phase 1 |
| F-HZ-001 | CRITICAL | Add `#![forbid(unsafe_code)]` to `action_queue.rs` line 1 | §Phase 3 |
| F-FE-001 | HIGH | Split files to <300 lines (runtime.rs:2824, action_queue.rs:1314, types.rs:983) | §Phase 2 |
| F-HZ-003 | HIGH | Close PF-vb-8mdp.8-S6A7-001 (Verus production-helper binding) | §Phase 3 |
| F-HZ-002 | MEDIUM | Replace `panic!()` in tests with `assert!()` | §Phase 3 |
| F-FE-002 | MEDIUM | Separate production, test, and proof modules into distinct files | §Phase 2 |

---

## Advisory Notes (Not Blocking)

1. **F-DD-001:** Replace double-enum mapping with `From` impl or direct semantic enum use.
2. **F-DD-002:** Eliminate redundant capacity validation in admission path.
3. **F-BT-001:** Document `core::mem::forget` pattern in Kani harness or redesign to avoid it.

---

## Review Metadata

- **Review confidence:** HIGH (all production files read, verification artifacts inspected, contract documents cross-referenced)
- **Files examined:** `action_queue.rs` (1314 lines), `shard/types.rs` (983 lines), `runtime.rs` (lines 1-80, 235-294), `vb_queue_semantics/src/lib.rs` (427 lines), `kani_runtime_queuefull.rs` (174 lines), Verus artifacts (9 files), Flux artifacts (2 files), bead contracts (4 documents)
- **Evidence from source checkout:** `/home/lewis/src/velvet-ballistics/.beads/vb-8mdp.{1,8}/`
- **Evidence from isolated workspace:** `/home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.8/`
- **Bead directory artifacts:** Only state-11 files exist; four required state-13 inputs missing

---

## Next Owner

**proof-planner / femdation controller** — The bead must return to contract/modeling/proof-planning. The four missing input artifacts must be produced before another black-hat review attempt. The open Verus binding finding (PF-vb-8mdp.8-S6A7-001) needs architecture-level disposition.

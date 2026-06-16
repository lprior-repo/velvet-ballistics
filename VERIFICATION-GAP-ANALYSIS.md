# MASTER VERIFICATION GAP ANALYSIS
## velvet-ballistics — Full Formal Verification Audit

**Date:** 2026-06-15
**Scope:** All 16 production crates + workspace_tests (1,459 + 237 .rs files)
**Verification Dimensions:** Verus, Kani, Flux, Loom, Miri, Proptest, TLA+

---

## EXECUTIVE SUMMARY

### Overall Health: CRITICAL

| Dimension | Coverage | Status | Key Finding |
|-----------|----------|--------|-------------|
| **Verus** | ~25% superficial | REJECTED | 100% of proofs are self-contained stubs; zero binding to production code |
| **Kani** | ~25% superficial | BLOCKED | 21/21 obligations FAIL_BLOCKED by kani::assert_eq! API incompatibility |
| **Flux** | ~35% superficial | REJECTED | vb_runtime 100% trusted (0 checked); vb_compile dead (0 functions processed) |
| **Loom** | ~15% superficial | REJECTED | All 14 models are toy abstractions; zero bound to production concurrent code |
| **Miri** | ~5% superficial | REJECTED | 5/6 "Miri" tests use catch_unwind, not real Miri instrumentation |
| **Proptest** | ~60% uneven | NEEDS WORK | vb_queue_semantics (P0) and vb_boundary_inventory, vb_cli have ZERO proptest |

### Systemic Issues (GOD RULE VIOLATIONS)

1. **GOD RULE #2 Violation (Verus):** Every Verus proof file defines its own types and proves tautologies. Zero `exec fn` bindings to production code. Zero `requires`/`ensures` on implementation functions.
2. **GOD RULE #1 Violation (Kani):** 364 occurrences of `kani::assert_eq!` (doesn't exist in Kani 0.67.0). All harnesses need fix before any Kani proof can compile.
3. **Flux Trust Abuse:** vb_runtime has 1,344 `#[flux_rs::trusted]` instances and zero checked functions. All production model functions are trusted, not proven.
4. **Toy Abstraction Problem (Loom):** All 14 Loom models define separate abstract types (e.g., `ActionTicket { AtomicBool }`) that don't match production types (`ActionTicket { RunId, StepIdx, SeqNo, ActionId, attempt, idempotency_key, capacity }`).

---

## CRATE-BY-CRATE GAP ANALYSIS

### vb_core (277 .rs files) — HEAVY VERIFICATION, ALL SUPERFICIAL

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| Verus | 3 files | TOY | Self-contained RunFrame/StepState proofs. Zero exec fn binding. |
| Kani | 3 files | BLOCKED | kani::assert_eq! incompatibility. |
| Flux | 1 file | TOY | Separate enum types, not production annotation. |
| Loom | 0 | ABSENT | Workflow replay + parallel in-flight concurrency unproven. |
| Miri | 0 | ABSENT | Buffer/memory operations unverified. |
| Proptest | 12+ files | DECENT | 6 tests `#[cfg(miri, ignore)]` — skipping UB vectors. |

**Critical Unproven Functions:**
- `propagate_action_taint` — no Verus spec, no proof
- `compute_action_idempotency_key` — no Verus spec, no proof
- `issue_action_ticket` — no Verus spec, no proof
- `action_ticket_has_valid_key` — no Verus spec, no proof
- `is_valid_step_state_transition` — external_body trust stub only

---

### vb_runtime (356 .rs files) — LARGEST CRATE, MOST ARTIFACTS, ALL SUPERFICIAL

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| Verus | 8 files | TOY | runtime_facade_typed_errors uses external_body trust stubs. 5 vb-0l9k0 files are empty fn main(){} skeletons. |
| Kani | 18 files | BLOCKED | kani::assert_eq! incompatibility. |
| Flux | 32 files | TRUST-ABUSE | 1,344 trusted / 0 checked. All production functions marked trusted. |
| Loom | 12 files | TOY | ActionTicket model has wrong fields. JournalWriterQueue models wrong type. Timer model unbound. |
| Miri | 1 file | WEAK | cfg(miri) on trace/tests.rs only. |
| Proptest | 16 files | DECENT | Good coverage but missing backpressure concurrency properties. |

**Critical Unproven Functions:**
- `RuntimeFacade::shard_index` — external_body trust stub only
- `RuntimeFacade::submit_direct` — external_body trust stub only
- `RuntimeFacade::inspect_run` — external_body trust stub only
- `BoundedActionCompletionQueue` — no Loom proof for push/pop/disconnect
- `IntrospectionRegistry` — no Loom proof for concurrent register/unregister
- `ShardCounters` — Relaxed atomics unproven

---

### vb_storage (291 .rs files) — HEAVY VERIFICATION, ALL SUPERFICIAL

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| Verus | 1 file | TOY | recovery_types_spec.rs defines Spec* types, not production binding. |
| Kani | 56+ files | BLOCKED | kani::assert_eq! incompatibility across all. |
| Flux | 6 files | TOY | Separate Spec* types, not production annotation. |
| Loom | 2 files | TOY | Models different abstract queues, not Fjall-backed journal. |
| Miri | 0 | ABSENT | Fjall integration paths unverified for UB. |
| Proptest | 20 files | GOOD | Adequate coverage but postcard UB unverified by Miri. |

**Critical Unproven Functions:**
- FjallJournal write_lock lifecycle — no Loom proof
- Recovery type state machine — no Verus exec fn binding
- postcard serialization/deserialization — no Miri coverage

---

### vb_compile (229 .rs files) — VERIFICATION EXISTS BUT IS DEAD

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| Verus | 8 files | ABSENT | No exec fn binding. |
| Kani | 0 | ABSENT | All 229 files unverified. |
| Flux | 6 files | DEAD | No [package.metadata.flux] include list. 0 functions processed. |
| Loom | 0 | ABSENT | Compilation pipeline parallelism unproven. |
| Miri | 0 | ABSENT | Expression lowering unverified. |
| Proptest | ~20 files | GOOD | Adequate coverage for lowering properties. |

**P0 Fix: vb_compile Flux is DEAD**
1. Add `[package.metadata.flux]` include list to Cargo.toml
2. Remove `#[cfg(flux)]` from 2 flux modules (blocks visibility during cargo flux)

**Critical Unproven Functions:**
- Body width reduction — no Verus/Kani proof
- Chain reduction — no Verus/Kani proof
- Foreach reduction — no Verus/Kani proof
- All 6 .flux files never processed by cargo flux

---

### vb_ipc (58 .rs files) — MINIMAL VERIFICATION

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| Verus | 1 file | TOY | vb_5iebh spec types, not production binding. |
| Kani | 0 | ABSENT | Lock-free MPSC queue completely unverified. |
| Flux | 1 file | TOY | Separate types, not production annotation. |
| Loom | 0 | ABSENT | CRITICAL: Drop-based disconnect signaling has race conditions. |
| Miri | 0 | ABSENT | crossbeam_queue (unsafe) unverified. |
| Proptest | 3 files | THIN | Lock-free queue properties missing. |

**Critical Unproven Functions:**
- `MemoryIngress::drop` — 3 distinct disconnect paths with potential race
- `IngressCore` disconnect signaling — no Loom proof
- Bounded MPSC queue — no Kani panic-freedom proof
- Backpressure threshold at 80% — no property test

---

### vb_queue_semantics (4 .rs files) — P0 GAP: QUEUE STATE MACHINE, ZERO VERIFICATION

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| Verus | 0 | ABSENT | CRITICAL: Queue state machine has no formal verification. |
| Kani | 0 | ABSENT | State transitions unproven. |
| Flux | 0 | ABSENT | Queue invariants unrefined. |
| Loom | 0 | ABSENT | Not applicable (deterministic state machine). |
| Miri | 0 | ABSENT | Not applicable (no unsafe). |
| Proptest | 0 | ABSENT | **MOST SIGNIFICANT GAP IN ENTIRE REPOSITORY** |

**This entire crate is a deterministic queue state machine with:**
- `QueueState<T>`, `EnqueueDecision`, `PopTransition`
- `shard_tick_transition`, `action_enqueue_transition`, `action_dequeue_transition`
- `enqueue_decision`, `warning_payload`, `validate_capacity`
- **ZERO verification artifacts of any kind**

**Required Property Tests:**
- Associativity/commutativity of enqueue+dequeue sequences
- Invariant: `len <= capacity` after any sequence
- Invariant: `is_empty()` iff `len() == 0`
- Invariant: FIFO ordering preserved
- Algebraic laws: `enqueue(dequeue(q)) = q` when non-empty

---

### vb_boundary_inventory (16 .rs files) — NO VERIFICATION

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| Verus | 0 | ABSENT | Boundary classification invariants unproven. |
| Kani | 0 | ABSENT | Model validation unproven. |
| Flux | 0 | ABSENT | Type invariants unrefined. |
| Loom | 0 | ABSENT | IntrospectionRegistry concurrent state unproven. |
| Miri | 0 | ABSENT | Not applicable. |
| Proptest | 0 | ABSENT | **All state machine invariants untested.** |

---

### vb_cli (162 .rs files) — ZERO VERIFICATION

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| Verus | 0 | ABSENT | Command parsing invariants unproven. |
| Kani | 0 | ABSENT | Command dispatch unproven. |
| Flux | 0 | ABSENT | Command enum refinement missing. |
| Loom | 0 | ABSENT | Not applicable. |
| Miri | 0 | ABSENT | Command parsing unverified. |
| Proptest | 0 | ABSENT | **All 162 files untested for properties.** |

---

### vb_expr (68 .rs files) — WEAK MIRI, THIN PROPTEST

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| Verus | 0 | ABSENT | Expression evaluation invariants unproven. |
| Kani | 0 | ABSENT | Parser/eval panic-freedom unproven. |
| Flux | 0 | ABSENT | Expression type invariants unrefined. |
| Loom | 0 | ABSENT | Not applicable. |
| Miri | 2 files | MISLEADING | Named `*_miri_tests.rs` but use `catch_unwind`, NOT real Miri. |
| Proptest | 2 files | THIN | Lexer/parser properties incomplete. |

---

### vb_validate (59 .rs files) — KANI ONLY

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| Verus | 0 | ABSENT | Gate validation proofs missing. |
| Kani | 5 files | BLOCKED | kani::assert_eq! incompatibility. |
| Flux | 0 | ABSENT | Gate invariants unrefined. |
| Loom | 0 | ABSENT | Not applicable. |
| Miri | 0 | ABSENT | Gate evaluation unverified. |
| Proptest | 3 files | ADEQUATE | Red phase properties present. |

---

### vb_yaml (31 .rs files) — THIN PROPTEST

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| Verus | 0 | ABSENT | YAML parsing invariants unproven. |
| Kani | 5 files | BLOCKED | kani::assert_eq! incompatibility. |
| Flux | 0 | ABSENT | AST type invariants unrefined. |
| Loom | 0 | ABSENT | Not applicable. |
| Miri | 0 | ABSENT | Parsing unverified. |
| Proptest | 1 file | SEVERELY THIN | Only error code registration tested. |

---

### vb_proof_kernels (39 .rs files) — VERIFICATION CRATE, NO VERIFICATION

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| Verus | 0 | ABSENT | **CATASTROPHIC:** This crate is designed for formal verification but has zero Verus proofs. |
| Kani | 0 | ABSENT | Step state transitions unproven. |
| Flux | 0 | ABSENT | Transition invariants unrefined. |
| Loom | 0 | ABSENT | Not applicable. |
| Miri | 0 | ABSENT | Not applicable. |
| Proptest | 2 files | THIN | Profile properties only. |

---

### vb_ajc40_flux (7 .rs files) — FLUX CONTRACT CRATE, WEAK

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| Verus | 0 | ABSENT | Flux contract functions unproven. |
| Kani | 1 file | TOY | Flux module harness. |
| Flux | 7 files | PRESENT | Flux contracts defined. |
| Loom | 0 | ABSENT | Not applicable. |
| Miri | 0 | ABSENT | Not applicable. |
| Proptest | 0 | ABSENT | Flux contracts untested. |

---

### vb_doc (8 .rs files) — MINIMAL

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| All | 0 | ABSENT | Evidence reconciliation unverified. |

---

### vb_test_util (5 .rs files) — TEST ONLY

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| All | 0 | ABSENT | Test fixture utilities, low risk. |

---

### vb_benchmark (15 .rs files) — BENCHMARK ONLY

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| Kani | 7 files | BLOCKED | kani::assert_eq! incompatibility. |

---

### vb_verification (1 .rs file) — PLACEHOLDER

| Dimension | Has Artifacts | Quality | Gap |
|-----------|--------------|---------|-----|
| All | 0 | ABSENT | Thin wrapper/placeholder. |

---

## SYSTEMIC BLOCKERS (MUST FIX BEFORE ANY PROOF CAN RUN)

### Blocker 1: Kani API Incompatibility (BLOCKS ALL 56+ Kani Harnesses)

**364 occurrences of `kani::assert_eq!`** across 51 files in vb_core (and inherited workspace-wide).

Kani 0.67.0 requires: `kani::assert(a == b, "msg")` — NOT `kani::assert_eq!(a, b, "msg")`.

**Fix command for proof-writer:**
```bash
# Find and replace all kani::assert_eq! with kani::assert(a == b, msg)
grep -rn 'kani::assert_eq!' crates/ --include="*.rs" | wc -l
# 364 occurrences — all must be replaced
```

### Blocker 2: Evidence Command Mismatches (BLOCKS 15/21 Kani Obligations)

Harness names in evidence_command fields don't match actual function names:
- vb-fzgdn: 10 harness names wrong
- vb-xi2f24: 5 harness names wrong

### Blocker 3: Empty Skeleton Files (5 FILES)

`src/verification/verus/vb-0l9k0/` in vb_runtime has 5 files with only `fn main() {}`:
- `timer_init_proof.rs`
- `timer_add_proof.rs`
- `timer_remove_proof.rs`
- `timer_fire_proof.rs`
- `timer_wheel_tick_proof.rs`

### Blocker 4: vb_compile Flux Dead Configuration

The 6 `.flux` files in `src/mod_compile_lowering/` are never processed:
1. Missing `[package.metadata.flux]` include list in Cargo.toml
2. `#[cfg(flux)]` on all annotations blocks visibility

---

## PRIORITY RECOMMENDATIONS

### P0 — Fix Blockers (1-2 days)

1. Replace 364 `kani::assert_eq!` → `kani::assert(a == b, msg)` across workspace
2. Fix evidence_command harness names for vb-fzgdn (10) and vb-xi2f24 (5)
3. Remove or implement 5 empty vb-0l9k0 skeleton files

### P1 — Bind Verus to Production (2-4 weeks)

4. Add `exec fn` delegation from Verus proofs to production functions (GOD RULE #2)
5. Add `requires`/`ensures` to `is_valid_step_state_transition`, `propagate_action_taint`, etc.
6. Replace external_body trust stubs with actual `exec fn` bindings

### P2 — Replace Flux Trust Abuse (1-2 weeks)

7. Replace 1,344 `#[flux_rs::trusted]` in vb_runtime with actual `#[spec]` annotations
8. Fix vb_compile Flux configuration to actually process .flux files
9. Annotate production types instead of defining separate toy types

### P3 — Write Real Loom Models (2-4 weeks)

10. Rewrite all 14 Loom models to exercise production concurrent code
11. Add Loom proof for vb_ipc Drop-based disconnect signaling
12. Add Loom proof for vb_runtime IntrospectionRegistry concurrent operations

### P4 — Add Missing Verification to Zero-Coverage Crates (ongoing)

13. **vb_queue_semantics** — Proptest (CRITICAL: queue state machine)
14. **vb_boundary_inventory** — Proptest + Loom (IntrospectionRegistry)
15. **vb_cli** — Proptest (command parsing invariants)
16. **vb_proof_kernels** — Verus + Kani (step state transitions)
17. **vb_expr** — Real Miri (not catch_unwind) + Kani
18. **vb_yaml** — Proptest (parser properties)
19. **vb_ajc40_flux** — Proptest (contract verification)
20. **vb_ipc** — Kani + Loom (lock-free queue + disconnect)

### P5 — Add Real Miri Coverage (1 week)

21. Replace `catch_unwind` in `*_miri_tests.rs` files with `#[cfg(miri)]` + real Miri runs
22. Run `cargo miri test` on vb_storage (Fjall integration paths)
23. Run `cargo miri test` on vb_yaml (postcard integration paths)

---

## VERIFICATION MATRIX (CURRENT STATE → TARGET STATE)

| Crate | Current Coverage | Target Coverage | Effort |
|-------|-----------------|-----------------|--------|
| vb_core | ~25% superficial | ~70% bound | High |
| vb_runtime | ~30% superficial | ~75% bound | High |
| vb_storage | ~35% superficial | ~65% bound | Medium |
| vb_compile | ~20% dead | ~60% bound | Medium |
| vb_ipc | ~10% superficial | ~70% bound | Medium |
| vb_queue_semantics | 0% | ~80% (proptest) | Medium |
| vb_boundary_inventory | 0% | ~60% (proptest+loom) | Medium |
| vb_cli | 0% | ~40% (proptest) | Medium |
| vb_expr | ~10% misleading | ~60% (miri+kani) | Medium |
| vb_validate | ~20% blocked | ~65% bound | Low |
| vb_yaml | ~15% blocked | ~50% (proptest+kani) | Low |
| vb_proof_kernels | ~10% thin | ~80% (verus+kani) | High |
| vb_ajc40_flux | ~30% present | ~70% (proptest) | Low |
| vb_doc | 0% | ~40% (proptest) | Low |
| vb_test_util | 0% | ~20% (test fixtures) | Low |
| vb_benchmark | ~25% blocked | ~30% (benchmarks) | Low |

---

## CONCLUSION

The velvet-ballistics repository has **significant formal verification infrastructure** (~120 Kani files, ~110 Verus files, ~30 Flux files, ~65 TLA+ specs, 63 fuzz targets) but the **quality of verification is critically deficient**:

1. **Zero production bindings** — No Verus `exec fn` delegates to production code. No Verus `requires`/`ensures` on implementation functions.
2. **All toy abstractions** — Loom models, Flux refinements, and even some Verus proofs define separate types from production code.
3. **Trust abuse** — 1,344 `#[flux_rs::trusted]` in vb_runtime with zero checked functions.
4. **API blockers** — 364 `kani::assert_eq!` calls block all Kani compilation.
5. **Missing coverage** — 6 crates have ZERO verification of any kind.
6. **Misleading names** — Files named `*_miri_tests.rs` use `catch_unwind`, not real Miri.

**This is not a verification coverage problem. This is a verification quality problem.**

The repository has the scaffolding for world-class formal verification but every single artifact is superficial. The next 6-12 weeks of work should be: **replace, don't add.** Replace toy abstractions with production bindings. Replace trusted annotations with checked refinements. Replace misleading catch_unwind with real Miri instrumentation.

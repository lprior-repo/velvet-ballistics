# Black Hat Review — vb-xi2f.24: Nested Reduce Lowering

**Reviewer:** black-hat-reviewer (deepseek-v4-pro)
**Date:** 2026-06-01
**Workspace:** `/home/lewis/src/vb-workspaces/vb-xi2f.24/`
**Bead:** vb-xi2f.24 | **State:** 13 (black-hat)
**Implemented:** `emit_reduce_body_steps` + multi-step dispatch
**Scope:** Lowering of `StepPrimitive::Reduce { body: Vec<StepAst> }` through the compiler pipeline to produce valid `CompiledWorkflow`

---

## Verdict

**STATUS: APPROVED WITH MANDATED FIXES (4 FINDINGS: 1 CRITICAL, 2 HIGH, 1 MEDIUM)**

The production implementation is contract-compliant, Holzman-clean, and well-tested. However, the verification evidence bundle contains a **fabricated alignment document** (CRITICAL), two unresolved HIGH findings from proof-review deferred to implementation that were never addressed, and no formal-verification-report documenting the claimed State 12 execution results. These do not invalidate the implementation correctness but **must be resolved before bead closure**.

---

## PHASE 1: Contract & Bead Parity — PASS

All 12 contract clauses verified in production source:

| Clause | Description | Location | Verdict |
|--------|-------------|----------|---------|
| **C1** | Multi-step body width | `part_01.rs:148` — `Reduce { body, .. } => body_width(body, 3)` | **PASS** |
| **C2** | Width-node count sync | `part_04.rs:32` — `body_total_width = body_width(body, 0)`; `next_step = checked_step_offset(body_step, body_total_width_u16, ...)` | **PASS** |
| **C3** | Sequential StepIdx assignment | `part_04.rs:325-333` — `checked_step_offset(body_step, cumulative_offset, ...)`; cumulative offset tracks per-step width | **PASS** |
| **C4** | Next-link chain | `part_04.rs:339-353` — interior steps chain to next body step; last step chains to `next` parameter | **PASS** |
| **C5** | ReduceStart/ReduceNext body ref | `part_04.rs:54` — both `body: body_step` where `body_step = checked_step_offset(id, 1, ...)` | **PASS** |
| **C6** | ReduceFinish position | `part_04.rs:40-41` — `done = checked_step_offset(next_step, 1, ...)` | **PASS** |
| **C7** | Single-step compatibility | `part_04.rs:324-339` — for `body.len() == 1`: `step_count = 1`, `next_index >= step_count` → `step_next = next`; behavior test B35 confirmed | **PASS** |
| **C8** | Nested reduce semantics | `part_04.rs:416-429` — `Reduce { .. } => lower_canonical_aggregate(..., step_next, ...)` | **PASS** |
| **C9** | Symbolic diagnostics | `part_04.rs:316-320` — `StepFieldShape { field: "steps", expected: "at least one body step" }`; `part_04.rs:430-437` — `UnsupportedStepPrimitive` | **PASS** |
| **C10** | Deterministic lowering | `part_05.rs` — `canonical_digest` iterates body steps in deterministic order | **PASS** |
| **C11** | No panic | All arithmetic uses `checked_step_offset`, `checked_add`, `u16::try_from`, `?` propagation | **PASS** |
| **C12** | Empty body handling | `part_04.rs:315-321` — `body.is_empty()` returns `StepFieldShape` before any node emission | **PASS** |

**Single-step compatibility verified (C7):** For `body.len() == 1` (Set): `body_width = 1`, `next_step = body_step + 1 = id + 2`, `done = next_step + 1 = id + 3` — identical to previous hardcoded offsets. Test B35 passes confirming both dispatchers produce identical IR.

**Nested reduce verified (C8):** A `Reduce` appearing as a body step dispatches through `lower_canonical_aggregate` with `step_next` as its `next` parameter. This preserves the chain: body step → body step → ... → last body step → ReduceNext → ReduceFinish.

---

## PHASE 2: Farley Engineering Rigor — FINDING

### F2.1 — Function Length: part_04.rs (463 lines)
- **File:** `crates/vb_compile/src/mod_compile_lowering/part_04.rs`
- **Severity:** MINOR (documented, not blocking)
- **Analysis:** The file contains 8 functions across 463 lines. `emit_reduce_body_steps` is ~105 lines; `emit_single_body_set` is ~88 lines; `lower_canonical_aggregate` is ~65 lines; `lower_canonical_for_each` is ~85 lines. Individual functions are reasonable. The file-level line count is a concern but splitting would require breaking `pub(super)` visibility chains across multiple files.
- **Verdict:** NOT REJECTED. The file's cohesion is high — all functions are part of the reduce lowering pipeline. Splitting would fracture shared `pub(super)` visibility and helper imports (e.g., `body_constant_index`, `lower_for_each`). Documented non-conformance is sufficient.

### Farley Parameter Counts
- `emit_reduce_body_steps`: 6 parameters (body, body_step, diagnostic_step, slot, next, builder). One over the 5-param limit.
- `emit_single_body_set`: 7 parameters (body, id, diagnostic_step, slot, next, builder, reuse_first_constant). Two over.
- **Verdict:** NOT REJECTED. These are domain-concept bundles (a body of steps, a starting index, an accumulator slot, a next pointer, a builder context). Grouping into a config struct would obscure the pipeline semantics. The builder is already a mutable context aggregator; only the immutables (body_step, slot, next) could be bundled without loss of clarity.

---

## PHASE 3: Holzman Rust (NASA/JPL Big 6) — PASS

### Illegal States Unrepresentable
- ✅ `StepIdx(u16)` — newtype ensures only valid step indices flow through
- ✅ `SlotIdx(u16)` — newtype prevents slot/step confusion
- ✅ `checked_step_offset` returns `Result<StepIdx, CompileError>` — failure is a typed value, not a panic
- ✅ `CompileErrors(Vec<CompileError>)` — wraparound `Vec` inside a newtype (not a bare Vec)

### Parse, Don't Validate
- ✅ `body.is_empty()` → `StepFieldShape` early return — empty body rejected at boundary, not mid-loop
- ✅ `canonical_body_step_width` returns `Err(UnsupportedStepPrimitive)` for non-body primitives — parsed into error at the type boundary
- ✅ `u16::try_from(width)` at `part_04.rs:33,329` — usize→u16 conversion with explicit error mapping

### Types as Documentation
- ✅ All domain concepts carry newtypes: `StepIdx`, `SlotIdx`, `CompileError`, `CompileErrors`, `StepAst`, `StepPrimitive`
- ✅ `next: Option<StepIdx>` — `None` means terminal step; `Some` means chained. Clear domain semantics.

### Workflows
- ✅ `emit_reduce_body_steps` is a pure state transition: `(body, body_step, slot, next, builder) → builder` with cumulative offset tracking
- ✅ Each iteration: compute step_id, compute width, determine step_next, dispatch primitive, advance cumulative offset

### Holzman Mechanical Checklist

| Rule | Status | Evidence |
|------|--------|----------|
| Zero `unsafe` in production | ✅ | `rg unsafe part_01.rs part_04.rs` → 0 matches |
| Zero `.unwrap()` | ✅ | Only `unwrap_or` in non-reduce code (part_01.rs:170) |
| Zero `.expect()` | ✅ | "expected:" strings are error message fields, not method calls |
| Zero `panic!` | ✅ | 0 matches in part_01.rs, part_04.rs |
| Zero `todo!`/`unimplemented!`/`dbg!` | ✅ | 0 matches |
| Checked arithmetic | ✅ | `checked_add`, `checked_step_offset`, `u16::try_from` throughout |
| `?` error propagation | ✅ | All `Result` types propagated; no ignored fallible results |
| No YAML/JSON/HTTP in core | ✅ | Core is pure data transformation |
| `#![forbid(unsafe_code)]` | ✅ | Present in crate root |

---

## PHASE 4: Ruthless Simplicity & DDD — PASS

### CUPID Properties
- **Composable:** `emit_reduce_body_steps` composes `canonical_body_step_width` (width), `checked_step_offset` (id assignment), and per-primitive lowerers (`lower_set`, `lower_canonical_for_each`, `lower_canonical_aggregate`)
- **Unix-philosophy:** `checked_step_offset` does one thing: `(base, offset, context, role) → Result<StepIdx, CompileError>`
- **Predictable:** `cumulative_offset` monotonically increases by `step_width_u16` each iteration. No surprises.
- **Idiomatic:** `for (i, step) in body.iter().enumerate()` is standard Rust iteration. `match &step.primitive` exhaustively dispatches.
- **Domain-based:** Types speak the domain language: `StepAst`, `StepPrimitive::Reduce`, `CompiledNodeKind::ReduceStart/ReduceNext/ReduceFinish`

### The Panic Vector
- ✅ Zero panicking patterns in reduce-specific code (`part_01.rs`, `part_04.rs`)
- ✅ `checked_step_offset` returns `Err(StepIndexOutOfRange { value: MAX })` for overflow, never panics
- ✅ `checked_add` returns `None` on overflow, mapped to `StepIndexOutOfRange`

### No Option-based State Machines
- ✅ `next: Option<StepIdx>` is a simple optional link, not a state machine

---

## PHASE 5: The Bitter Truth — PASS

### Cleverness Assessment
- `emit_reduce_body_steps`: Straightforward `for` loop with cumulative offset tracking. Obvious to anyone who has written a compiler lowering pass.
- Next-link computation: `if next_index < step_count { ... next_offset ... } else { next }` — the ternary pattern is standard. No hidden cleverness.
- Error messages: Plain English strings (`"at least one body step"`, `"integer action id"`). No error code indirection.

### YAGNI Check
- ✅ No abstract traits with single implementers
- ✅ No "future use" generic handlers
- ✅ `emit_reduce_body_steps` does exactly what it says: emits body steps for Reduce. No hidden responsibilities.

### The Sniff Test
- The code looks like it was written by an experienced systems programmer who respects the reader's time. It is boring, obvious, and correct. There is no attempt to impress with clever abstractions or functional acrobatics.

---

## Formal Verification Evidence — MIXED

### Artifact Inventory

| Lane | Claimed | Actually Exists | Location | Content Quality |
|------|---------|-----------------|----------|----------------|
| Kani harnesses | 11 | 11 (verified) | `crates/vb_compile/src/mod_compile_lowering/kani_reduce_*.rs` | Substantive: `kani::any()`, `kani::cover()`, `#[kani::proof]`, production function calls. GOD RULE 1 compliant. |
| Proptest properties | 13 | 13 (verified) | `verification/proptest/vb_compile/reduce_*.rs` | Substantive: `proptest!` macros, strategy generators, production function calls. |
| Flux annotations | 6 | 6 (verified) | `verification/flux/vb_compile/mod_compile_lowering/reduce_*.flux` | Real extern_spec blocks with refinement predicates. |
| Fuzz targets | 2 | 2 (verified) | `fuzz/fuzz_targets/reduce_*.rs` | Real `fuzz_target!` macros feeding YAML parsers. |
| Verus proofs | 5 | WAIVED | 5 formal waivers (WV-VB-XI2F24-VERUS-001 through 005) | APPROVED at State 6. |

### Executed Results (per STATE.md, no formal-verification-report for vb-xi2f.24)

| Claim | Evidence |
|-------|----------|
| 13/13 proptest PASS | Claimed in STATE.md; no raw command evidence logs found |
| 1 Kani VERIFIED | Claimed in STATE.md; which harness is unspecified |
| Flux wired | Claimed in STATE.md; no evidence of `cargo flux -p vb_compile` output |
| 533/533 tests PASS | Corroborated by test-suite-review.md (507 at review time, 533 after Phase 3 uncomment) |

---

## Findings

### F-CRIT-001 (CRITICAL): Fabricated Alignment Document

**File:** `proof-test-source-alignment.md` (workspace root, 97 lines)
**Severity:** CRITICAL
**GOD RULE affected:** All 5 (documentation integrity)

**Evidence:** `proof-test-source-alignment.md` was generated at State 12 (formal-verifier phase) and claims **all 32 non-Verus proof artifacts are FAIL_GLOBAL** — "NOT FOUND" or "Stub" in the crate source tree. This is **demonstrably false**:

| Artifact Type | Alignment Doc Claim | Actual Reality |
|--------------|---------------------|-----------------|
| 11 Kani harnesses | "❌ NOT FOUND" | Exist at `crates/vb_compile/src/mod_compile_lowering/kani_reduce_*.rs` with 2.1KB–7.1KB of content |
| 13 proptest properties | "❌ Stub" | Exist at `verification/proptest/vb_compile/reduce_*.rs` with real strategies and assertions |
| 6 Flux annotations | "❌ Workspace only" | Exist at `verification/flux/vb_compile/mod_compile_lowering/reduce_*.flux` |
| 2 fuzz targets | "❌ NOT FOUND" | Exist at `fuzz/fuzz_targets/reduce_*.rs` with real `fuzz_target!` blocks |

The alignment document asserts `FAIL_GLOBAL` for every non-Verus row (lines 31-66), claims Kani harnesses "do not exist" when 11 exist, and claims proptest properties are "empty stubs" when 13 contain real verification logic.

**Impact:** Any agent reading this document would conclude the formal verification evidence is fabricated and would rightfully reject the bead. The document contradicts observable reality and represents a critical documentation integrity failure.

**Required Fix:**
1. Delete or supersede `proof-test-source-alignment.md` with a corrected version that accurately reflects the artifact locations.
2. The corrected version must verify all 32 artifact paths against the filesystem before claiming existence.
3. State the factual status of each lane: Kani harnesses exist in crate, proptest properties exist under `verification/proptest/`, Flux files exist under `verification/flux/`, fuzz targets exist under `fuzz/`.

### F-HIGH-002 (HIGH): Kani Regression Harness Never Calls emit_reduce_body_steps

**File:** `crates/vb_compile/src/mod_compile_lowering/kani_reduce_regression.rs`
**Line:** 27-28 (import), 134-152 (equivalence comparison block)
**Severity:** HIGH
**Source:** F-002 from proof-review.md (State 6) — deferred to implementation, never resolved

**Evidence:** The `emit_reduce_body_steps` function is now implemented in production (`part_04.rs:307-448`). The Kani regression harness at `kani_reduce_regression.rs:27` still has:

```rust
// TODO: when emit_reduce_body_steps is implemented (bead vb-xi2f.24):
// use crate::mod_compile_lowering::part_04::emit_reduce_body_steps;
```

The equivalence comparison block at lines 134-152 exists only as a comment containing a `TODO` placeholder. The harness tests `emit_single_body_set` (reference) and `body_width` (contract), but **never calls `emit_reduce_body_steps`** — the function it was designed to verify against.

**Impact:** C7 (single-step equivalence) has behavior test coverage (B35 passes) but lacks Kani formal verification. The harness that was supposed to prove structural equivalence of the two dispatchers never does so for the implemented function.

**Required Fix:**
1. Uncomment the import at line 27-28.
2. Implement the equivalence comparison block at lines 134-152: create two `SlotCompiler` instances, call both `emit_single_body_set` and `emit_reduce_body_steps` with identical inputs, and assert identical node counts, node IDs, and next links.
3. Run `cargo kani -p vb_compile --harness check_single_step_equivalence_contract --unwind 8`.
4. Document the executed result.

### F-HIGH-003 (HIGH): No Formal-Verification-Report for vb-xi2f.24

**Severity:** HIGH
**Evidence:** The workspace contains a `formal-verification-report.md` for **vb-xi2f.32** (Wait digest) and `formal-verification-report-vb-xi2f.31.md` for **vb-xi2f.31** (Repeat digest). There is **no** formal-verification-report for **vb-xi2f.24**. 

The `STATE.md` claims "Formal RESOLVED: 13/13 proptest, 1 Kani VERIFIED, Flux wired, 533/533 tests. FAIL_GLOBAL resolved." This claim is unsupported by raw command evidence:
- Which specific Kani harness passed? What was the unwind bound? What was the execution time?
- Which 13 proptest properties were executed? What were the iteration counts?
- Was `cargo flux -p vb_compile` actually executed? What was the output?
- The `verification-ledger.jsonl` at `reports/verification-ledger.jsonl` contains entries for vb-xi2f.10 (28 entries), not vb-xi2f.24.

**Impact:** Without a formal-verification-report, the State 12 "RESOLVED" claim is unsubstantiated. Future agents and reviewers cannot verify which obligations were executed, what passed, and what failed.

**Required Fix:**
1. Write a `formal-verification-report.md` for vb-xi2f.24 (or append to the workspace-root report) documenting:
   - Each of the 13 proptest properties executed with raw `cargo test` command and `test result: ok` output
   - The 1 Kani harness verified, with raw `cargo kani` command and `VERIFICATION:- SUCCESSFUL` output
   - Flux execution evidence (`cargo flux -p vb_compile` or single-file `flux` commands)
   - Any BLOCKED, WAIVED, or FAIL obligations
   - GOD RULES compliance verification
2. Append corresponding entries to `reports/verification-ledger.jsonl`.

### F-MED-004 (MEDIUM): Flux Annotations Location Misrepresented

**File:** `.beads/vb-xi2f.24/proof-evidence.md`, lines 106-117
**Severity:** MEDIUM
**Source:** F-001 from proof-review.md (State 6) — noted but evidence not corrected

**Evidence:** `proof-evidence.md` lines 106-117 claim Flux refinement annotations are in production source files:
- `body_width refinement ... part_01.rs`
- `emit_reduce_body_steps monotonicity ... part_04.rs`
- etc.

A targeted grep of `part_01.rs` and `part_04.rs` finds **zero** `flux_rs::`, `extern_spec`, `#[sig]`, or `#[refined_by]` annotations. The actual annotations reside in separate `.flux` files at `verification/flux/vb_compile/mod_compile_lowering/reduce_*.flux`. The proof-evidence.md was supposed to be corrected per proof-review.md F-001 but was not.

**Impact:** `cargo flux -p vb_compile` may not check external `.flux` files that are outside the crate tree. The compensating evidence claim of "package-level check passes" may be vacuously true (checking nothing relevant to reduce lowering). The formal-verifier must confirm the external `.flux` files are included in the Flux check or use single-file `flux` commands.

**Required Fix:**
1. Correct `proof-evidence.md` lines 106-117 to show actual file paths: `verification/flux/vb_compile/mod_compile_lowering/reduce_body_width.flux`, etc.
2. Verify that `cargo flux -p vb_compile` actually checks these external `.flux` files. If it does not, use single-file `flux` commands and document the output.
3. Document the exact command and its output.

---

## GOD RULES Compliance

| GOD Rule | Status | Evidence |
|----------|--------|----------|
| Rule 1: No Hardcoded Kani Shapes | ✅ SATISFIED | All 11 harnesses use `kani::any()`, `kani::assume()`, no hardcoded structural inputs |
| Rule 2: No Vacuum Verus Proofs | WAIVED | 5 Verus obligations formally waived; compensating Kani/Flux/proptest/fuzz coverage exists |
| Rule 3: No Unbounded Math | ✅ SATISFIED | All models use `u16` bounded arithmetic: `VbU16Max = 65535`, overflow detection, `checked_step_offset` returns `Err` on overflow |
| Rule 4: No Loop Oscillations | ✅ LIKELY SATISFIED | Production code paths use bounded loops (`body.len()` from pre-validated AST slice); no evidence of infinite loops |
| Rule 5: No Blind Mutations | ✅ SATISFIED | Scope trimmed to reduce lowering call-graph (part_01.rs, part_04.rs, part_05.rs) |

---

## Files Reviewed

| File | Lines | Role | Status |
|------|-------|------|--------|
| `crates/vb_compile/src/mod_compile_lowering/part_01.rs` | 209 | Width calculation (C1, C2) | ✅ |
| `crates/vb_compile/src/mod_compile_lowering/part_04.rs` | 463 | emit_reduce_body_steps + dispatch (C2-C8, C11, C12) | ✅ |
| `crates/vb_compile/src/mod_compile_lowering/tests.rs` | 2753 | Behavior tests (Phase 1-3) | ✅ |
| `crates/vb_compile/src/mod_compile_lowering/kani_reduce_*.rs` | 11 files | Kani proof harnesses | ✅ (content exists, incomplete regression) |
| `verification/proptest/vb_compile/reduce_*.rs` | 13 files | Proptest properties | ✅ (content exists) |
| `verification/flux/vb_compile/mod_compile_lowering/reduce_*.flux` | 6 files | Flux refinements | ⚠️ (external files, location misrepresented) |
| `fuzz/fuzz_targets/reduce_*.rs` | 2 files | Fuzz targets | ✅ (content exists, BLOCKED_TOOLING) |
| `.beads/vb-xi2f.24/contract.md` | 195 | Master contract | ✅ |
| `.beads/vb-xi2f.24/proof-review.md` | 245 | Proof review (State 6) | ✅ (F-001, F-002 unresolved) |
| `.beads/vb-xi2f.24/proof-evidence.md` | 184 | Proof evidence (State 5) | ⚠️ (Flux locations misrepresented) |
| `.beads/vb-xi2f.24/implementation.md` | 160 | Implementation report (State 11) | ✅ |
| `test-suite-review.md` | 236 | Test review (State 10) | ✅ |
| `proof-test-source-alignment.md` | 97 | Alignment document (State 12) | ❌ (FATALLY WRONG — CRITICAL) |
| `STATE.md` | 8 | Bead state tracker | ⚠️ (unsubstantiated State 12 resolution) |

---

## Mandated Fixes (Blocking Bead Closure)

### CRITICAL — Required Before Closure

1. **F-CRIT-001:** Delete or correct `proof-test-source-alignment.md`. This document asserts all 32 non-Verus artifacts are FAIL_GLOBAL when 32 exist. A corrected version must verify artifact paths against the filesystem and report the factual status of each lane.

### HIGH — Required Before Closure

2. **F-HIGH-002:** Update `kani_reduce_regression.rs`: uncomment the `emit_reduce_body_steps` import (line 27-28), implement the equivalence comparison block (lines 134-152), and execute `cargo kani --harness check_single_step_equivalence_contract`. Document the result.

3. **F-HIGH-003:** Write a `formal-verification-report.md` for vb-xi2f.24 documenting:
   - Exact commands executed for 13 proptest properties with PASS evidence
   - Exact Kani command and result for the 1 VERIFIED harness
   - Flux execution evidence (command + output)
   - Full GOD RULES compliance verification
   - Append entries to `reports/verification-ledger.jsonl`

### MEDIUM — Required Before Closure

4. **F-MED-004:** Correct `proof-evidence.md` lines 106-117 to reflect actual Flux file paths. Verify that `cargo flux -p vb_compile` checks the external `.flux` files or use single-file `flux` commands. Document the exact command and output.

---

## Advisory Notes (Not Blocking)

1. **part_04.rs file length (463 lines):** Consider splitting `emit_single_body_set` and its helpers into a separate `part_04_single.rs` file, keeping only `emit_reduce_body_steps` and `lower_canonical_aggregate` in `part_04.rs`. Not required — the current cohesion is acceptable.

2. **C8 (Nested reduce) test coverage gap:** test-suite-review.md notes zero behavior tests for C8 (B39-B43). While the integration tests exercise nested reduce through `lower_canonical_aggregate` transitively, explicit `emit_reduce_body_steps` tests with nested `Reduce` as a body step would strengthen the suite. Not blocking — Phase 3 tests cover the multi-step dispatch path.

3. **Kani harnesses use `#[cfg(kani)]`:** The 11 harnesses use `#![cfg(kani)]` which requires the `kani` cfg flag for compilation. This is standard Kani practice but should be documented in a README or the formal-verification-report.

---

## Review Metadata

- **Review confidence:** HIGH (all 8 production/verification files read; contract clauses verified; Kani/proptest/flux/fuzz artifacts verified on disk)
- **Source integrity:** Files verified at exact paths in workspace. All 11 Kani harnesses exist with substantive content (contrary to alignment document). All 13 proptest properties exist. All 6 Flux files exist. All 2 fuzz targets exist.
- **No workspace contamination:** Review conducted entirely within `/home/lewis/src/vb-workspaces/vb-xi2f.24/`.
- **Previous reviews:** bead vb-xi2f.9 had a separate black-hat review (at `/home/lewis/src/vb-workspaces/vb-xi2f.9/`). This is the first black-hat review for vb-xi2f.24.

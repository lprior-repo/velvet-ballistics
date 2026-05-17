# Test Plan Review: Lethal Cross-Cutting Findings C.1–C.25
## Mode 1: Plan Inquisition

## VERDICT: REJECTED

---

## Axis 1 — Contract Parity: FAIL

**Not Applicable / Unverifiable**

This plan does not map to a single `contract.md`. There are 85+ `contract.md` files scattered across `.beads/` directories. No finding in this plan references which contract it remediates. Without a contract-to-finding trace, contract parity cannot be verified.

**LETHAL**: Finding C.13 (`ShardDirective`) defines a new type without specifying what behavior the type must have. The open question "what is the intended behavior of `ShardDirective::Barrier`?" is blocking. You cannot test a type whose semantics are undefined.

---

## Axis 2 — Assertion Sharpness: FAIL

### LETHAL Findings

**C.1 — `property_tests.rs` is EMPTY (lines 30–57)**
The remediation "test" is a code skeleton with no assertions:
```rust
#[test]
fn execute_node_full_deterministic(a: u8, b: u8) {
    // property: same inputs → same output, no panic
}
```
This is not a test. It proves nothing. The skill says: "find the test that passes when the function it covers is deleted." This skeleton passes even if `execute_node` is deleted.

**C.21 (lines 128–135) — `assert!(validated.is_ok())`**
The skill explicitly calls `is_ok()` → **LETHAL** (Axis 2, line 67). While the plan adds additional `assert_eq!` on digests, the first assertion is `assert!(validated.is_ok())` with no concrete value. This is exactly the weak assertion the skill forbids.

**C.23 (lines 196–211) — Error variant assertions use `matches!` without exhausting variants**
```rust
assert!(matches!(e, IpcError::PayloadTooLarge {..}) ||
        matches!(e, IpcError::FrameChecksumMismatch {..}) ||
        // ... exhaustive error variants
));
```
The `// ... exhaustive error variants` placeholder means the error variant coverage is incomplete. This is a stub assertion.

**C.24 (lines 233–243) — Taint monotonicity check uses weak assertion**
```rust
assert!(taint.level() >= max_input_taint.level(), "taint must be monotonic");
```
This uses `taint.level()` which assumes a total ordering on taint levels. If the assertion itself is wrong (e.g., taint levels aren't comparable), this doesn't catch it. No explicit test of the comparison operator itself.

### MAJOR Findings

**C.2 (lines 63–79) — No verification of strategy deduplication**
The remediation is a `grep` that shows "should be empty." But the plan never specifies what command actually runs in CI. No bash script, no assert statement in the plan itself. The failure mode is descriptive, not tested.

**C.9 (lines 349–370) — Miri test count check is manual**
The remediation says `TMPDIR=target/miri-tmp cargo miri test -p vb_core --lib --all-features` should run >50 tests. But there's no CI gate that fails if the count is ≤3. The plan describes a check but doesn't mandate it as a gate.

---

## Axis 3 — Trophy Allocation: UNVERIFIABLE

**Not enough information to verify ratios.**

The plan claims:
- ~25 unit / ~10 integration / ~5 static analysis
- 8 new fuzz targets + 3 fixes
- 6 proptest invariants
- 2 Kani harnesses
- 12 mutation checkpoints

**Problem**: These are targets, not confirmed deliverables. No verification method is specified for any of these numbers. The plan does not state which finding produces which trophy. C.1 produces proptest strategies, C.25 produces a fuzz target — but there is no mapping table.

**Trophy Density Audit**: Without a mapping from findings → trophy types, the claim of "25 unit tests" cannot be audited. The reviewer cannot verify that the trophy allocation is appropriate.

**LETHAL** (per skill line 79): "Planned unit test count < 5× public function count → LETHAL." This plan never states the public function count. The ratio cannot be computed.

---

## Axis 4 — Boundary Completeness: PARTIAL PASS

**Acceptable for infrastructure/remediation findings.** Most findings (C.1–C.6, C.9–C.12, C.16–C.20) are not functions with input boundaries — they are infrastructure gaps. For these, boundary completeness is not applicable.

**MINOR gaps**:

- **C.25 (pagination fuzz)**: Plan lists boundary cases (0, 1, max, overflow) but does not specify:
  - What "max" page_size is (u16::MAX? usize::MAX? Config-dependent?)
  - What "overflow" means for cursor position
  - Empty list edge case is mentioned but not asserted

- **C.14 (evaluate command)**: Does not specify boundary for expression length. What happens with a 1MB expression string? The plan only shows happy-path examples.

---

## Axis 5 — Mutation Survivability: PARTIAL PASS

**Mutation checkpoints identified (12 total)** but **not validated**.

The plan does not run the mutation thought experiment explicitly. Let me apply it:

| Finding | Mutation | Would Test Catch It? |
|---------|----------|---------------------|
| C.21 | Delete the `assert_eq!(w1.digest(), w2.digest())` line | **YES** — explicit equality check |
| C.22 | Change `node_count` assertion to `>` instead of `==` | **YES** — exact equality |
| C.22 | Remove slot index bounds check `s.get() < parts.slot_count` | **NO** — no test for out-of-bounds slot access |
| C.23 | Change `decoded.payload.len() <= header.payload_len` to `<` | **YES** — boundary comparison preserved |
| C.24 | Remove taint monotonicity assertion | **NO** — test still passes because `is_ok()` and `Clean` checks would still pass |
| C.24 | Return wrong `SlotValue` type (e.g., I64 instead of F64) | **NO** — type is not asserted, only taint is |

**LETHAL**: C.22 and C.24 have mutation paths that survive the specified test.

---

## Axis 6 — Evidence Plan Audit: FAIL

**C.1 remediation test does not state preconditions.** The skeleton comment "property: same inputs → same output, no panic" is not an explicit precondition. What is the input space? What invariants must hold before the test runs?

**C.21–C.25 (fuzz targets)**: The plan describes what the fuzz target must do but does not describe what preconditions the corpus must satisfy. For example, C.22 says "corpus covering valid and invalid postcard-encoded WorkflowParts" — what makes a WorkflowParts valid vs invalid? The plan doesn't specify.

**Open Question 3 (C.18/C.19)**: "What are the exact 7 crate names with high expect/unwrap counts?" Without this answer, the remediation cannot be verified. The plan references a "helper coverage plan" that was not provided.

**Open Question 4 (C.8)**: "Which 3 helpers out of 10 are fully tested?" Without this, the gap inventory (7/10 helpers with edge/error gaps) cannot be verified.

---

## Summary of LETHAL Findings

| # | Finding | Lethal Issue |
|---|---------|--------------|
| C.1 | `property_tests.rs` empty | Remediation is a skeleton with no assertions — proves nothing |
| C.13 | `ShardDirective` enum MISSING | Open question: undefined semantics for `Barrier` variant; cannot write tests |
| C.21 | `generated_compare` fuzz STUB | Uses `assert!(validated.is_ok())` — weak assertion (LETHAL per skill) |
| C.23 | `ipc_frame` fuzz discards | Incomplete `matches!` with `// ... exhaustive error variants` placeholder |
| C.24 | `expression` fuzz discards | Taint monotonicity assertion can be deleted without test failure |
| C.22 | `compiled_ir` fuzz STUB | Slot index bounds check can be deleted without test failure |
| C.8 | 7/10 helpers have edge/error gaps | Cannot remediate without knowing which 3/10 are fully tested (missing doc) |
| C.18 | 7 crates have 418+ `expect()` | Cannot remediate without exact crate names (open question 3) |
| C.19 | 7 crates have 518+ `unwrap()` | Cannot remediate without exact crate names (open question 3) |

**Total LETHAL: 9**

---

## Summary of MAJOR Findings

| # | Finding | Major Issue |
|---|---------|-------------|
| C.2 | No centralized property_tests/ | No CI gate for strategy deduplication; just a `grep` command |
| C.9 | Miri runs only 3 tests | "Should run >50" is not a CI gate; no enforcement |
| C.14 | `evaluate` command unimplemented | Boundary for expression length not specified |
| C.25 | Pagination fuzz missing | "max" page_size and overflow cursor undefined |

**Total MAJOR: 4 (below ≥3 threshold for REJECTED, but LETHAL takes precedence)**

---

## Open Questions Blocking Approval

1. **C.13**: What is `ShardDirective::Barrier` semantics?
2. **C.14**: Does `evaluate` support `--slots` or empty context only?
3. **C.18/C.19**: What are the exact 7 crate names with high expect/unwrap?
4. **C.8**: Which 3/10 helpers are fully tested? (Helper coverage plan not provided)
5. **C.5**: What is the exact `vb_core` coverage compilation error?

---

## Mandatory Remediation Before Resubmission

1. **C.1**: Write a real proptest with actual assertions. Not a skeleton.
2. **C.13**: Define `ShardDirective::Barrier` behavior or remove it from the enum.
3. **C.21**: Replace `assert!(validated.is_ok())` with `assert_eq!(validated, Ok(expected))` or specify exact expected value.
4. **C.23**: Fill in exhaustive error variant match or state which variants are intentionally unhandled.
5. **C.22**: Add a mutation checkpoint: removing slot index bounds check must fail a test.
6. **C.24**: Add a mutation checkpoint: removing taint monotonicity assertion must fail a test. Consider adding type assertion on `SlotValue`.
7. **C.18/C.19**: Provide the 7 crate names so remediation can be verified.
8. **C.8**: Provide the helper coverage plan or enumerate the 3/10 fully-tested helpers.

---

## Trophy Allocation Summary (Unverified)

| Trophy Type | Claimed | Verifiable |
|------------|---------|------------|
| Unit tests | ~25 | NO — no mapping from findings |
| Integration tests | ~10 | NO |
| Static analysis | ~5 | NO |
| Fuzz targets | 8 new + 3 fixes | PARTIAL — specifications exist but not verified |
| Proptest invariants | 6 | NO |
| Kani harnesses | 2 | NO |
| Mutation checkpoints | 12 | PARTIAL — 2/12 have identified survivors |

**Cannot approve trophy allocation without a finding-to-trophy mapping table.**

---

## Final Verdict

**STATUS: REJECTED**

**Reason**: 9 LETHAL findings. The plan contains infrastructure remediation specifications but 9 of the 25 findings cannot be verified as remediated without additional information or contain assertions that can pass with the underlying bug present.

The plan is **not ready**. It specifies what needs to be built but does not specify how to verify the build succeeded in multiple critical cases. Fill in the open questions and fix the LETHAL assertion weaknesses before resubmission.

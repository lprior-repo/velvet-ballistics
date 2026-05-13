# Defects: vb-qi37.8

**bead_id**: vb-qi37.8
**state**: 12 (Black-Hat Review)
**verdict**: REJECTED

---

## Defect Summary

| Defect ID | Gate | Severity | Title | Owning State |
|-----------|------|----------|-------|--------------|
| D1 | G15 | CRITICAL | Non-determinism separation lacks formal verification | 11 (Formal Verification) |
| D2 | G13 | HIGH | Slot cycle detection lacks Kani bounded proof | 11 (Formal Verification) |
| D3 | UB | HIGH | Miri UB verification not executed | 11 (Formal Verification) |
| D4 | G7 | MEDIUM | Stack depth bounded proof lacks Kani | 11 (Formal Verification) |
| D5 | G12 | MEDIUM | Bijection lacks Kani bounded proof | 11 (Formal Verification) |
| D6 | CI | LOW | 5 moon ci tasks unexplained skipped | 11 (Formal Verification) |

---

## Defect Details

### D1: G15 Non-determinism Separation — CRITICAL

**Contract Requirement**: ABORT-10: Non-determinism without suspension → Error(G15)

**Proof Strategy (per proof-review.md)**:
- Kani for bounded model checking
- TLA+ for temporal property (PO-025)
- Lean for formal proof (PO-026)

**Actual Coverage**:
- Kani: DEFERRED (not executed)
- TLA+: DEFERRED (PO-025)
- Lean: DEFERRED (PO-026)
- Unit tests: PASS (but unit ≠ formal verification)

**Gap**: G15 is classified HIGH risk requiring triple formal verification coverage. Only unit tests were executed. Kani and TLA+ and Lean were all deferred, breaking the proof chain.

**Fix Required**:
1. Execute Kani for G15 (PO-024, PO-027)
2. Execute TLA+ for temporal property (PO-025)
3. Execute Lean for formal proof (PO-026)

**Owning State**: 11 (Formal Verification)
**Downstream Impact**: If G15 fails at runtime, non-deterministic nodes may execute without proper suspension, causing undefined ordering behavior.

---

### D2: G13 Slot Cycle Detection — HIGH

**Contract Requirement**: ABORT-8: Slot cycle detected → Error(G13)

**Proof Strategy (per proof-review.md)**:
- Kani for bounded proof (PO-019)
- TLA+ for termination (PO-020)

**Actual Coverage**:
- Kani: DEFERRED (not executed)
- TLA+: DEFERRED (PO-020)
- Unit tests: PASS

**Gap**: Slot cycle detection is critical for non-termination prevention. Kani was never executed. Unit tests cannot guarantee bounded proof completeness.

**Fix Required**:
1. Execute Kani for G13 (PO-019)
2. Execute TLA+ for termination (PO-020) after Kani passes

**Owning State**: 11 (Formal Verification)
**Downstream Impact**: Undetected slot cycles could cause validation to loop indefinitely.

---

### D3: Miri UB Verification — HIGH

**Contract Requirement**: AC7: No panic on any input via Miri + Kani bounded checking

**Proof Strategy (per proof-review.md)**:
- Miri for fast UB detection (PO-002,004,007,012,015,021,023,027,029)

**Actual Coverage**:
- Miri: TIMED OUT (not executed)
- Substituted with: `#![forbid(unsafe_code)]`

**Gap**: `#![forbid(unsafe_code)]` prevents unsafe code but does not detect UB in safe Rust patterns (aliasing violations, drop order issues, etc.). Miri was the primary UB verification lane per proof-review.md.

**Fix Required**:
1. Execute Miri for 9 UB proof obligations, OR
2. Provide formal justification that safe Rust patterns in vb_validate cannot exhibit UB

**Owning State**: 11 (Formal Verification)
**Downstream Impact**: UB could cause validation to produce incorrect results or crash.

---

### D4: G7 Stack Depth — MEDIUM

**Contract Requirement**: ABORT-2: Expression stack overflow (>64) → Error(G7)

**Proof Strategy (per proof-review.md)**:
- Kani for bounded proof (PO-001, PO-002)

**Actual Coverage**:
- Kani: DEFERRED (not executed)
- Unit tests: PASS (990 tests)

**Gap**: Stack depth of 64 is a hard bound. Unit tests cannot exhaustively verify all paths stay within this bound. Kani provides formal bounded guarantee.

**Fix Required**:
1. Execute Kani for G7 (PO-001, PO-002), OR
2. Provide formal proof that expression evaluation cannot exceed 64 stack frames

**Owning State**: 11 (Formal Verification)
**Downstream Impact**: Stack overflow could cause validation to crash.

---

### D5: G12 Bijection — MEDIUM

**Contract Requirement**: ABORT-7: Action contract bijection fails → Error(G12)

**Proof Strategy (per proof-review.md)**:
- Kani for bounded proof (PO-016, PO-017, PO-018)

**Actual Coverage**:
- Kani: DEFERRED (not executed)
- Proptest: PASS (PO-018)
- Unit tests: PASS

**Gap**: Bijection is a critical safety property. Proptest is property-based testing (bounded random sampling), not formal verification. Kani provides exhaustive bounded proof.

**Fix Required**:
1. Execute Kani for G12 (PO-016, PO-017, PO-018)

**Owning State**: 11 (Formal Verification)
**Downstream Impact**: Bijection failure could cause incorrect action contract binding, leading to wrong behavior at runtime.

---

### D6: Moon CI Skipped Tasks — LOW

**Evidence**: STATE.md:28-30
```
"moon ci: 11 tasks completed (2 cached), 2 failed, 5 skipped"
```

**Gap**: 5 skipped tasks is unexplained. Skipped tasks in the canonical CI gate must be documented.

**Fix Required**:
1. Document why 5 tasks were skipped
2. Confirm none of the skipped tasks are required for vb-qi37.8 sign-off

**Owning State**: 11 (Formal Verification)
**Downstream Impact**: Unknown - verification completeness cannot be confirmed.

---

## Remediation Path

```
State 11 (Formal Verification)
  │
  ├─► Execute Kani for G7, G13, G15, G12
  │
  ├─► Execute Miri for 9 UB obligations (or justify safe Rust sufficiency)
  │
  ├─► Document 5 skipped moon ci tasks
  │
  └─► Re-run formal-verification-report

State 12 (Black-Hat Review) — Re-run after remediation
```

---

## Evidence Files

| File | Relevance |
|------|-----------|
| proof-review.md:24-30 | Miri and Kani lane obligations |
| proof-review.md:34-36 | Proof obligation dependency chain |
| proof-review.md:42-46 | Risk classification (G15 HIGH) |
| formal-verification-report.md:60-70 | Kani and Miri deferred |
| STATE.md:28-30 | moon ci skipped tasks |
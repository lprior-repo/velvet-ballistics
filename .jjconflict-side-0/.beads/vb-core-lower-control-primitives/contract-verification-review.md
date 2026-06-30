# Contract Verification Review: vb-core-lower-control-primitives

**Bead ID**: vb-core-lower-control-primitives
**Workspace**: /tmp/vb-ws/vb-core-lower-control-primitives
**Reviewer**: contract-verification-reviewer
**Date**: 2026-05-15

---

## STATUS: REJECTED

---

## Files Reviewed

| File | Status |
|------|--------|
| `contract.md` | ✓ Present |
| `proof-obligations.planned.jsonl` | ✓ Present, valid JSONL |
| `tla-spec.md` | ✗ NOT PRESENT (TLA+ spec embedded in `specs/ControlLowering.tla`) |
| `lean-contract.md` | ✗ NOT PRESENT (not applicable — Verus owns Rust-local proofs) |
| `verification-layers.md` | ✗ NOT PRESENT |
| `traceability-matrix.jsonl` | ✗ NOT PRESENT |

---

## Coverage Analysis

### Contract Clauses Traced

| Contract Clause | Obligation ID | Coverage |
|---|---|---|
| PRE-001 (id in u16::MAX-1 range) | VERUS-INV-001, VERUS-INV-002, KANI-OVERFLOW | PARTIAL — bound mismatch in proofs |
| POST-001 (lower_for_each returns 2 nodes) | VERUS-POST-001 | MISSING — vacuous proof |
| POST-002 (lower_together returns 2 nodes) | VERUS-POST-002 | MISSING — vacuous proof |
| POST-003 (lower_collect returns 3 nodes) | VERUS-POST-003 | MISSING — vacuous proof |
| POST-004 (lower_reduce returns 3 nodes) | VERUS-POST-004 | MISSING — vacuous proof |
| POST-005 (lower_repeat returns 3 nodes + attempt_slot=id+1) | VERUS-POST-005 | MISSING — vacuous proof |
| POST-007 (lower_ask returns 2 nodes + resume.id=id+1) | VERUS-POST-007 | MISSING — vacuous proof |
| INV-001 (step width invariants) | TLA-WF-001 | BROKEN — TLA+ spec syntax error |
| INV-002 (slots recorded before use) | TLA-WF-001 | BROKEN — TLA+ spec syntax error |
| INV-003 (WaitKind exhaustiveness) | VERUS-WAITKIND | PARTIAL — trusted Rust compiler |
| ERR-TYPES (CompileError exhaustive) | CLIPPY-ERR | PASS |

---

## Layer Fit Analysis

| Obligation | Risk | Layer Assigned | Fit Verdict | Notes |
|---|---|---|---|---|
| PRE-001 id+1 overflow | HIGH | Verus + Kani | PARTIAL | Bound mismatch; Kani harness structurally flawed |
| POST-001/002/003/004/005/007 | MEDIUM | Verus | FAIL | All vacuous — return `true` |
| INV-001/002 (step chain) | PROOF | TLA+ | FAIL | Spec syntax error — cannot run |
| INV-003 WaitKind | MEDIUM | Verus | FAIL | Trusts Rust compiler |
| ERR-TYPES | LOW | Clippy | PASS | Exhaustive match verified |

---

## TLA+ Scope Analysis

**Requirement**: TLA+-owned clauses (TLA-WF-001) must have:
- TLA+ module path ✓ (`ControlLowering.tla`)
- Config ✓ (`ControlLowering.cfg`)
- Variables, Init, Next, actions ✗ (partial)
- Safety invariants ✓ (`NoDuplicateStepIds`, `ValidOffsets`, `AskResumeIdCorrect`, `SlotsRecorded`)
- Temporal properties ✗ (`TemporalProgress` is defined but trivially true)
- Fairness/deadlock ✓ (DEADLOCK configured)
- Refinement relation ✗ (no refinement mapping to Rust)

**Verdict**: BROKEN — spec cannot be parsed by TLC. Even if fixed, the `TemporalProgress` property is trivially satisfied and provides no meaningful liveness check.

---

## Verus Scope Analysis

**Requirement**: Verus-owned clauses must have:
- Rust module/function target ✓ (lib.rs lower_* functions)
- Spec/proof fn declarations ✓ (present as stubs)
- Runtime shell exclusions ✗ (not stated)
- Executable verification ✗ (blocked — no inline annotations, dependencies missing)

**Verdict for vacuous proofs**: FAIL — spec fn returns `true`, proving nothing.

**Verdict for INV-001/INV-002**: PARTIAL — proof structure exists but bound mismatch with contract.

**Verdict for WAITKIND**: FAIL — trusted boundary expansion (trusts Rust compiler).

---

## Executable Obligation Shape

`proof-obligations.planned.jsonl` entries have all required fields:
- `id` ✓
- `contract_clause` ✓
- `command` ✓ (but commands cannot execute due to missing deps)
- `expected_evidence` ✓
- `risk` ✓
- `required` ✓
- `mode` ✓
- `status` ✓ (`planned`)

**Problem**: All Verus/Kani obligations have `status: planned` but their artifacts are STUBs that cannot execute. The obligations are planned but their evidence is missing.

---

## Key Findings

### Finding 1: Vacuous Verus Postconditions
**Severity**: LETHAL
**Clause IDs**: POST-001, POST-002, POST-003, POST-004, POST-005, POST-007
**Problem**: All 6 postcondition spec functions return `true` with no actual verification content.
**Impact**: These contract clauses have NO verification coverage.

### Finding 2: TLA+ Spec Syntax Error
**Severity**: LETHAL
**Clause IDs**: TLA-WF-001 (INV-001, INV-002)
**Problem**: TLC reports 59 semantic errors. Spec cannot be parsed.
**Impact**: Step chain well-formedness invariants cannot be verified.

### Finding 3: Bound Mismatch in INV Proofs
**Severity**: MAJOR
**Clause IDs**: PRE-001 (INV-001, INV-002)
**Problem**: Proof uses `id < u16::MAX - 1` but contract says "within u16::MAX - 1 range" — ambiguous if upper bound is inclusive or exclusive.
**Impact**: May not cover all valid inputs, or may prove overflow when none exists.

### Finding 4: WaitKind Trusted Boundary
**Severity**: MAJOR
**Clause ID**: INV-003
**Problem**: Proof trusts Rust compiler exhaustiveness checking, not Verus.
**Impact**: INV-003 not actually verified by Verus.

### Finding 5: Kani Harness Defects
**Severity**: MAJOR
**Clause ID**: PRE-001 (KANI-OVERFLOW)
**Problem**: Dead code, unnecessary `.max(0)`, does not verify `attempt_slot == id + 1`.
**Impact**: PRE-001 not fully verified by Kani.

---

## Waiver Analysis

No waivers present. All required obligations must have valid proof or explicit waiver with owner, reason, expiry, and compensating evidence.

---

## Required Actions

1. **Fix TLA+ spec**: Add `EXTENDS Naturals, FiniteSets`, define `Null`, fix range operators
2. **Replace vacuous Verus postconditions**: Write real spec/proof for each postcondition
3. **Clarify PRE-001 bound**: Define exact range for id (inclusive/exclusive of u16::MAX-1)
4. **Fix Kani harness**: Remove dead code, verify `attempt_slot` value
5. **Fix WAITKIND proof**: Prove exhaustiveness via Verus, not Rust compiler
6. **Create traceability-matrix.jsonl**: Map each contract clause to obligation ID

---

## Recommendation

**STATUS: REJECTED**

This bead cannot proceed to implementation (State 7+) because:
- 6 of 12 proof obligations are vacuous
- 1 of 12 is syntactically invalid
- Tooling cannot verify the STUB artifacts

Return to State 5 (Proof Writing) with this repair guide. Re-review required before advancing.

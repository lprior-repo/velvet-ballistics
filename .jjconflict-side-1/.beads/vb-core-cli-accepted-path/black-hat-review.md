# Black-Hat Review: vb-core-cli-accepted-path

bead_id: vb-core-cli-accepted-path
phase: 12
runner: black-hat-reviewer
updated_at: 2026-05-16T21:00:00Z

## Isolation Verification

- `pwd -P` → `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path` ✓
- Not nested under source checkout `/home/lewis/src/velvet-ballistics` ✓
- `case` guard confirms isolation ✓

## Missing Required Inputs (State 11 + Test Artifacts)

The following required inputs for State 12 do NOT exist in `.beads/vb-core-cli-accepted-path/`:

| Artifact | Expected Owner | Status |
|---|---|---|
| `formal-verification-report.md` | State 11 | MISSING |
| `verification-ledger.jsonl` | State 11 | MISSING |
| `machine-gate-report.md` | State 11 | MISSING |
| `regression-diff.md` | State 11 | MISSING |
| `test-plan.md` | State 7 | MISSING |
| `test-suite-review.md` | State 9 | MISSING |

**Verdict: CANNOT COMPLETE STATE 12 GATE — Required inputs absent.**

---

## Defect Analysis (Based on Available Evidence)

Despite missing formal verification artifacts, black-hat review of available evidence reveals:

### DEFECT-12-01: LETHAL-2 Production Bypass Untracked

**Severity**: BLOCKING (per go-skill retry_policy_7)

**Classification**: `BLOCK_LOCAL` → Owning State: 10 (Implementation)

**Root Cause**:
`admit_run` (`crates/vb_runtime/src/admission.rs:367-383`) accepts `&dyn ArtifactStore` (presence-only via `compiled_ir_exists()`). For `RuntimePolicy::Strict`/`RuntimePolicy::Journaled`, it only checks `store.compiled_ir_exists(digest)` — which always returns `true` for `AlwaysPresentArtifactStore`. This enables strict policy bypass.

**Evidence**:
- Kani harness `strict_legacy_presence_only_bypass_rejects_required_blocker` FAILS (proof-evidence.md line 485-492)
- `admit_run` at admission.rs:376 checks only `compiled_ir_exists()`, not full artifact validation
- `AlwaysPresentArtifactStore::compiled_ir_exists()` at admission.rs:232 returns `true` unconditionally

**Contract Violation**:
- INV-004: "`AlwaysPresentArtifactStore` is test-only or relaxed-only and cannot satisfy production strict/journaled CLI runtime construction"
- POST-004: "Missing, malformed, digest-mismatched... artifacts MUST reject before run state insertion"

**Waiver Assessment**:
State 6 approved a waiver for LETHAL-2 with compensating evidence (TLA+ PO-001, Verus PO-002/PO-003/PO-004, Kani PO-007 aggregate, Fuzz PO-010). However:

1. **Waiver governance issue**: The waiver was granted in State 6 (proof-review) before State 10 (implementation) properly addressed the finding. This violates repair targeting — the defect should have been routed to State 10 with a blocking classification.

2. **Compensating proof gap**: The compensating Verus/ TLA+ proofs verify protocol-level properties but do NOT prove that `admit_run` with `AlwaysPresentArtifactStore` cannot reach `RunAdmission` for strict/journaled policies. The Kani harness proves the opposite — that this path DOES incorrectly admit.

3. **Missing production issue**: The waiver states "Production fix is tracked as separate issue for ProductionOwner (State 10 implementation)" but no bd issue was created. The bead workflow requires issues for follow-up work.

**Required Action**:
1. Route DEFECT-12-01 to State 10 (Implementation) — NOT State 11 (Formal Verification)
2. Create bd issue for production fix of `admit_run` bypass
3. Re-run State 6 proof-review only after `admit_run` is fixed OR waiver is re-granted with stronger compensating evidence that specifically addresses the `admit_run` code path

---

### DEFECT-12-02: Test Loop Not Executed

**Severity**: NON-BLOCKING (but blocks complete verification)

**Classification**: `DEFERRED_GLOBAL`

**Root Cause**:
Test states (7, 8, 9) were never executed. `test-plan.md` and `test-suite-review.md` do not exist. The test loop is a required part of the go-skill pipeline.

**Evidence**:
```bash
$ ls .beads/vb-core-cli-accepted-path/test-*.md
ls: cannot access '.beads/vb-core-cli-accepted-path/test-*.md': No such file or directory
```

**Required Action**:
1. Route to State 7 (Test Planning) — requires contract.md, traceability-matrix.jsonl, and proof-obligations.planned.jsonl
2. State 7 must produce `test-plan.md`
3. State 8 must write failing-first tests
4. State 9 must produce `test-suite-review.md`
5. Then re-enter State 12 black-hat-review with test artifacts

---

## State Machine Analysis

Current STATE.md shows:
- `current_state: 6` at line 1077 (Proof and contract review)
- `STATUS: STATE_6_COMPLETE` at line 1148

The STATE.md does NOT contain any entry for `current_state: 12`. The highest state reached is 10 (Implementation) based on `implementation.md` existing.

**Pipeline Status**:
```
State 1-6:  COMPLETE (with LETHAL-2 waiver)
State 7-9:  NOT EXECUTED (tests)
State 10:   COMPLETE (implementation.md exists, LETHAL-1 fixed, LETHAL-2 NOT fixed)
State 11:   NOT EXECUTED (formal verification artifacts missing)
State 12:   THIS REVIEW (blocked by missing State 11 + State 7-9)
State 13-15: NOT REACHABLE
```

---

## PHASE 1: Contract & Bead Parity

**Assessment**: PARTIAL

- Contract clauses (PRE-001 through POST-006, INV-001 through INV-007) are well-specified in `contract.md`
- Implementation addresses LETHAL-1 (digest equality check) ✓
- Implementation does NOT address LETHAL-2 (`admit_run` bypass) ✗
- INV-004 explicitly prohibits `AlwaysPresentArtifactStore` for strict/journaled — violation exists

**Verdict**: Contract parity FAIL for INV-004 and POST-004 due to `admit_run` bypass.

---

## PHASE 2: Farley Engineering Rigor

**Assessment**: PASS (for the parts that were implemented)

- `admit_artifact_run` function: ~67 lines, acceptable complexity
- Digest equality check (LETHAL-1 fix): 6 lines ✓
- `Shard::new_with_journal` artifact store selection: ~14 lines, acceptable
- No functions exceed 25 lines significantly
- No functions have >5 parameters

**Verdict**: Farley PASS for implemented code.

---

## PHASE 3: Holzman Rust (The Big 6)

**Assessment**: PARTIAL

- `ArtifactEnvelopeError` and `AdmissionError` are proper enums ✓
- `RuntimePolicy` enum with `Strict`, `Journaled`, `Relaxed` variants ✓
- `ArtifactStore` vs `AcceptedArtifactStore` trait separation creates illegal-state risk: calling `admit_run` with `AlwaysPresentArtifactStore` for strict policy produces incorrect behavior, not a compile error ✗
- `REQUIRED_GATE_COUNT = 15` as a named constant ✓

**Verdict**: Holzman PARTIAL — trait design allows incorrect usage to compile.

---

## PHASE 4: Ruthless Simplicity & DDD

**Assessment**: PASS

- No `unwrap()`, `expect()`, `panic!()` in production code ✓
- All fallible operations return `Result<T, Error>` ✓
- `#[forbid(unsafe_code)]` at admission.rs:1 ✓

**Verdict**: Ruthless simplicity PASS.

---

## PHASE 5: Bitter Truth (Velocity & Legibility)

**Assessment**: PASS

- Code is readable and straightforward
- No clever abstractions or over-engineering
- Comments explain intent (e.g., "INV-002: digest binding must be total")

**Verdict**: Bitter Truth PASS.

---

## Completion Evidence

```
Black-Hat Review for vb-core-cli-accepted-path — STATE 12

VERDICT: REJECTED

DEFECT-12-01 (BLOCKING): LETHAL-2 `admit_run` bypass not fixed
  - Owner: State 10 (Implementation)
  - Route: Route to State 10, fix `admit_run` to use AcceptedArtifactStore
  - Then: Re-run State 6 proof-review with fixed implementation

DEFECT-12-02 (DEFERRED_GLOBAL): Test loop not executed
  - Owner: State 7 (Test Planning)
  - Route: Execute test states 7→8→9 before re-entering State 12

MISSING ARTIFACTS BLOCK STATE 12 COMPLETION:
  - formal-verification-report.md
  - verification-ledger.jsonl
  - machine-gate-report.md
  - regression-diff.md
  - test-plan.md
  - test-suite-review.md

Pipeline must reach State 11 completion before State 12 black-hat review can pass.
```

---

STATUS: STATE_12_REJECTED
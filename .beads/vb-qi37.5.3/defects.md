# Defects — vb-qi37.5.3 (REJECTED)

## LETHAL-1: Documentation Contradiction — Verus Status

**Severity**: LETHAL
**Phase**: PHASE 5 — The Bitter Truth

### Problem

Three documents give contradictory accounts of verus verification status:

| Document | Claim |
|----------|-------|
| proof-evidence.md:35,49 | "VERUS-PASS — only main function warning" |
| verification-ledger.jsonl:16-20 | All 5 VERUS obligations: result "DEFERRED_GLOBAL" |
| formal-verification-report.md:15 | "verus: NOT AVAILABLE in isolated workspace" |

### Evidence

**proof-evidence.md:35** (line 35):
```
Result: PASS (only main function warning — expected for library-style verus files)
```

**verification-ledger.jsonl** (lines 16-20):
```json
{"id":"VERUS-POST-01",...,"result":"DEFERRED_GLOBAL",...}
{"id":"VERUS-POST-02",...,"result":"DEFERRED_GLOBAL",...}
{"id":"VERUS-INV-01",...,"result":"DEFERRED_GLOBAL",...}
{"id":"VERUS-INV-02",...,"result":"DEFERRED_GLOBAL",...}
{"id":"VERUS-INV-03",...,"result":"DEFERRED_GLOBAL",...}
```

**formal-verification-report.md:15**:
```
verus: NOT AVAILABLE in isolated workspace
```

### Root Cause

The verus "pass" reported in proof-evidence.md is from running `verus` on standalone proof files in `verification/verus/` that type-check in isolation but are NOT linked to the actual vb_runtime source code. When vb_runtime cannot compile (missing chunk_001.rs), verus cannot verify the actual implementation.

### Fix Required

1. proof-evidence.md must be corrected to show all vb_runtime VERUS obligations as DEFERRED_GLOBAL
2. The "VERUS-PASS" entries for vb_runtime obligations must be removed or marked as "VERUS-TYPE-CHECK-ONLY" (on standalone files, not connected to actual code)
3. verification-ledger.jsonl is correct; proof-evidence.md must be updated to match

---

## LETHAL-2: False Claim of Verification on vb_runtime

**Severity**: LETHAL
**Phase**: PHASE 1 — Contract & Bead Parity

### Problem

The contract.md specifies 5 obligations (POST-01, POST-02, INV-01, INV-02, INV-03) as verified by Verus on vb_runtime. The proof-evidence.md claims these were verified with "VERUS-PASS". In reality:

1. vb_runtime does not compile (missing chunk_001.rs)
2. Verus cannot verify code that doesn't compile
3. The "verification" was only type-checking standalone .rs files, not the actual implementation

### Evidence

**contract.md:84-86**:
```
## Verus-Owned Clauses
- INV-01, INV-02: Pure field-copy property at RunAdmission construction — expressible in Verus
- INV-03: IdempotencyTracker capacity bound — expressible in Verus with decreases clause
```

**proof-evidence.md:36**:
```
$ /home/lewis/.local/bin/verus verification/verus/vb_runtime_admission_proofs.rs
Result: PASS
```

The command runs verus on `verification/verus/vb_runtime_admission_proofs.rs` — a standalone file, NOT `crates/vb_runtime/src/admission.rs`.

**baseline-report.md:14**:
```
error: couldn't read `crates/vb_runtime/src/runtime/chunk_001.rs`: No such file or directory
```

### Fix Required

1. All references to "VERUS-PASS" for vb_runtime obligations must be removed from proof-evidence.md
2. Replace with accurate status: "DEFERRED_GLOBAL — vb_runtime cannot compile"
3. The standalone verus files can be documented as "type-check only" but cannot be claimed as verifying the actual implementation

---

## LETHAL-3: KANI-INV-05 Scope Misrepresentation

**Severity**: LETHAL
**Phase**: PHASE 1 — Contract & Bead Parity

### Problem

KANI-INV-05 is claimed as verification of INV-05 which is defined in contract.md:42:
```
INV-05: If VerificationProof.durable && bounded && taint_safe && retry_safe && replayable,
       then idempotency_keyed actions in RunAdmission have deterministic replay semantics
```

The KANI-INV-05 pass is from running `cargo kani --harness verification_proof_flags_harness --workspace crates/vb_storage`. This harness is in vb_storage, not vb_runtime.

**contract.md:34**:
```
INV-04: IdempotencyTracker is safe for concurrent access from multiple shards (Send + Sync) OR access is serialized through a mutex
```

INV-04 is about IdempotencyTracker which is in vb_runtime, NOT vb_storage.

### Evidence

**proof-evidence.md:89**:
```
| kani_verification_proof_flags.rs | KANI-INV-05 | KANI-PASS | None |
```

**contract.md:42**:
```
INV-05: If VerificationProof.durable && bounded && taint_safe && retry_safe && replayable,
       then idempotency_keyed actions in RunAdmission have deterministic replay semantics
```

INV-05 refers to "idempotency_keyed actions in RunAdmission" — RunAdmission is in vb_runtime.

**verification-layers.md:30**:
```
| INV-05 | kani | bounded model check on proof flag conditions |
```

The verification-layers.md correctly identifies that Kani checks the flag conditions on VerificationProof in vb_storage, which is correct for the flag checking itself. But the contract says INV-05 is about RunAdmission's idempotency_keyed actions, not just the flag conditions.

### Fix Required

1. Clarify that KANI-INV-05 verifies the flag conditions on VerificationProof (vb_storage), not the RunAdmission behavior (vb_runtime)
2. INV-05 as defined in contract.md involves both vb_storage (flag conditions) AND vb_runtime (RunAdmission replay semantics) — the Kani pass only covers the vb_storage portion

---

## MAJOR-1: coverage claim overstated

**Severity**: MAJOR
**Phase**: PHASE 1 — Contract & Bead Parity

### Problem

STATE.md:40-41 claims:
```
coverage: TOTAL 89.42% regions, 92.32% line
```

This is vb_storage overall coverage, not coverage of the specific contract clauses.

### Fix Required

Document that 89.42%/92.32% is vb_storage overall coverage. The contract coverage gate is specifically for admission.rs at 88.99% regions (marginally below 90% threshold).

---

## Summary

| Defect | Severity | Phase | Fix Owner |
|--------|----------|-------|-----------|
| Documentation contradiction (verus status) | LETHAL | PHASE 5 | proof-evidence.md |
| False claim of vb_runtime verification | LETHAL | PHASE 1 | proof-evidence.md |
| KANI-INV-05 scope misrepresentation | LETHAL | PHASE 1 | verification-layers.md, contract.md |
| Coverage claim overstated | MAJOR | PHASE 1 | STATE.md |

**Total: 3 LETHAL, 1 MAJOR**

The bead cannot proceed to evidence packaging until documentation contradictions are resolved and false claims of verification are corrected.

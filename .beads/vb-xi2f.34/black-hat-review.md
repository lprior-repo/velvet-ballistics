# Black-Hat Adversarial Review — vb-xi2f.34: Finish Digest Semantics (RETRY 2)

**Reviewer:** black-hat-reviewer  
**Date:** 2026-05-25  
**Scope:** Mandatory remediation verification for E-1 (Kani unwind annotation) and E-4 (stale evidence file)  
**Workspace:** `/home/lewis/src/vb-workspaces/vb-xi2f.34`  

---

## STATUS: REJECTED — MANDATORY REMEDIATION INCOMPLETE

---

## Executive Summary

The previous review (2026-05-25) REJECTED with two mandatory fixes. This RETRY inspection verifies whether both were applied. **They were not.** E-1 was only partially applied (the harness annotation was fixed, but three dependent artifacts were not). E-4 was not applied at all. The production code remains correct and satisfies all 10 contract clauses, but evidence integrity remains broken. This black-hat reviewer does not approve code with unreproducible evidence — no matter how clean the production code is.

---

## Mandatory Remediation Audit

### E-1: Kani Unwind Annotation Mismatch (Previous: BF-001)

**Original finding:** Harness `#[kani::unwind(3)]` annotation did not match the evidence captured with `--unwind 8`. The prior review mandated:

> 1. Change `#[kani::unwind(3)]` to `#[kani::unwind(8)]` to match the actual evidence command
> 2. **AND** update `rust-refinement-obligations.jsonl` RRO-FINISH-KANI-002 `evidence_command` to match
> 3. **AND** update the verification ledger to reflect the passing evidence

**What was fixed (1 of 4):**

| Item | Status | Details |
|------|--------|---------|
| Harness annotation `kani_finish_digest.rs` line 240 | ✅ **FIXED** | `#[kani::unwind(3)]` → `#[kani::unwind(8)]` |
| `evidence/proof-evidence.md` line 36 | ✅ **ALREADY OK** | Evidence captured at `--unwind 8` |
| `rust-refinement-obligations.jsonl` RRO-FINISH-KANI-002 line 2 | ❌ **NOT FIXED** | Still says `"evidence_command":"cargo kani ... --unwind 3"` |
| Source file doc comment `kani_finish_digest.rs` line 63 | ❌ **NOT FIXED** | Still documents `--unwind 3` for this harness |
| `verification-ledger.jsonl` line 50 | ❌ **NOT FIXED** | Still shows `result: "FAIL_LOCAL"` at `--unwind 3` |

**Why this matters:** Anyone following the `rust-refinement-obligations.jsonl` evidence command (`--unwind 3`) will override the `#[kani::unwind(8)]` annotation with `--unwind 3` on the CLI and hit the same `memcmp` unwinding failure that the verification ledger honestly records. The evidence file at `evidence/proof-evidence.md` says `--unwind 8`, the harness says `unwind(8)`, but the canonical machine-readable specification (`rust-refinement-obligations.jsonl`) says `--unwind 3`. That is a three-way disagreement across the artifact chain.

**What the mandatory fix required and what was delivered:**

```
REQUIRED: annotation → 8, obligations JSONL → 8, ledger → PASS at 8
DELIVERED: annotation → 8, obligations JSONL → 3, ledger → FAIL_LOCAL at 3
```

**Severity: HIGH** — Evidence integrity. Unchanged from prior review. The annotated evidence command (`--unwind 3`) does not reproduce the claimed evidence (`VERIFICATION:- SUCCESSFUL at --unwind 8`). This is not a fix — it is 25% of a fix.

### E-4: Stale FAILED Evidence File on Disk (Previous: BF-002)

**Original finding:** `.beads/vb-xi2f.34/verification/proof-evidence.md` (2026-05-24, pre-REPAIR-2) shows PO-KANI-FINISH-003 as **FAILED (COUNTEREXAMPLE FOUND)**. The correct evidence is at `evidence/proof-evidence.md` (2026-05-25, REPAIR-2) showing it as **VERIFIED**.

**Required fix:** "Remove or rename the stale file to prevent future reviewers from reading the wrong evidence."

**Status:** ❌ **NOT FIXED.** The file is still on disk at 5.0K, with the old date (2026-05-24), old counterexample content, and no `REPAIR-2` header.

**Why this matters:** An agent, reviewer, or automated pipeline that crawls `.beads/vb-xi2f.34/verification/` before `evidence/` will find FAILED evidence for PO-KANI-FINISH-003 and may reject the bead or trigger unnecessary repairs. The stale evidence is a landmine.

**Severity: MEDIUM** — Not a production code defect, but a confounding artifact that any subsequent review pipeline will trip over.

---

## Production Code: Still Correct (No Change)

The Finish arm at `part_05.rs:150-157` has not changed since the prior review:

```rust
vb_yaml::ast::StepPrimitive::Finish { result } => {
    hasher.update(b"finish");
    match result {
        vb_yaml::ast::ScalarValue::String(value) => hasher.update(value.as_bytes()),
        vb_yaml::ast::ScalarValue::Integer(value) => hasher.update(&value.to_le_bytes()),
        _ => hasher.update(b"unsupported"),
    };
}
```

All 10 contract clauses (C1–C10) remain satisfied. Holzman Rust compliance remains 10/10. DDD principles upheld. Tests thorough. No new defects introduced.

---

## Complete Finding Summary

| Finding | Severity | Prior ID | Status | Blocker? |
|---------|----------|----------|--------|----------|
| E-1: Kani unwind mismatch (annotation fixed, 3 artifacts stale) | HIGH | BF-001 | **PARTIALLY FIXED** | **YES — MANDATORY** |
| E-4: Stale FAILED evidence on disk | MEDIUM | BF-002 | **NOT FIXED** | **YES — MANDATORY** |
| E-2: Kani models replicate production code | MEDIUM | PF-REP2-001 | No change (accepted-for-p1) | No |
| E-3: Proptest misnamed | LOW | PF-REP2-003 | No change (accepted-for-p1) | No |
| E-5: 894 lines dead code | LOW | PF-REP2-004 | No change (accepted-for-p1) | No |
| E-6: `canonical_primitive_name` `_` collision | INFO | None | No change | No |

---

## Evidence Cross-Reference (Current State)

| Artifact | Expected Content | Actual Content | Match? |
|----------|-----------------|----------------|--------|
| `kani_finish_digest.rs:240` | `#[kani::unwind(8)]` | `#[kani::unwind(8)]` | ✅ |
| `kani_finish_digest.rs:63` (doc comment) | Should say `--unwind 8` | Says `--unwind 3` | ❌ |
| `rust-refinement-obligations.jsonl` RRO-FINISH-KANI-002 | Should say `--unwind 8` | Says `--unwind 3` | ❌ |
| `evidence/proof-evidence.md:36` | `--unwind 8` | `--unwind 8` | ✅ |
| `verification-ledger.jsonl:50` | Should show `PASS` at `--unwind 8` | Shows `FAIL_LOCAL` at `--unwind 3` | ❌ |
| `.beads/vb-xi2f.34/verification/proof-evidence.md` | Should not exist | Exists at 5.0K with FAILED content | ❌ |

---

## Verdict

**REJECTED.** Two mandatory findings from the prior review remain unaddressed or only partially addressed:

1. **E-1 (PARTIAL):** The harness annotation is fixed (`unwind(8)`), but three dependent artifacts — `rust-refinement-obligations.jsonl`, the source doc comment, and the verification ledger — still reference `--unwind 3` and `FAIL_LOCAL`. A fix must touch all artifacts in the evidence chain, not just one.

2. **E-4 (NO FIX):** The stale FAILED evidence file at `.beads/vb-xi2f.34/verification/proof-evidence.md` is still on disk. Remove it.

The production code is production-ready. The evidence package is not. I will not approve evidence that cannot be reproduced by following the machine-readable specification.

---

### Mandatory Remediation (All required for approval)

1. **Complete E-1 fix across all four locations:**
   - ✅ `kani_finish_digest.rs:240` → `#[kani::unwind(8)]` (DONE)
   - ❌ `kani_finish_digest.rs:63` doc comment → `cargo kani ... --unwind 8` (NOT DONE)
   - ❌ `rust-refinement-obligations.jsonl` RRO-FINISH-KANI-002 `evidence_command` → `--unwind 8` (NOT DONE)
   - ❌ `verification-ledger.jsonl:50` → update to `result: "PASS"` with evidence at `--unwind 8` (NOT DONE)

2. **Fix E-4:** Remove or rename `.beads/vb-xi2f.34/verification/proof-evidence.md`

---

*"Fixing one line out of four is not a fix. It's a start. Come back when the evidence chain is consistent end-to-end."* — black-hat-reviewer

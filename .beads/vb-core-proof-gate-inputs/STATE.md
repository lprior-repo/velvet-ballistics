# State 11 — vb-core-proof-gate-inputs (Formal Verification — Partially Resolved)

| Field | Value |
|-------|-------|
| **bead_id** | vb-core-proof-gate-inputs |
| **state** | 11 |
| **source_checkout** | /home/lewis/src/velvet-ballistics |
| **isolated_workspace** | /tmp/vb-ws/vb-core-proof-gate-inputs |
| **workspace_path_proof** | /tmp/vb-ws/vb-core-proof-gate-inputs IS NOT nested under source → ISOLATED_OK |
| **attempt** | 2 |
| **previous_state** | 5 (proof-writer repair) |
| **next_gate** | Kani workspace fix OR waiver for K-G2-001 |

---

## Formal Verification Result

**STATUS: PARTIAL PASS** — 6 Verus obligations PASS; 1 Kani obligation BLOCKED by workspace issue

---

## Verus Proof Files (NEW — Created in State 5 repair)

| Obligation | File | Proofs | Status |
|------------|------|--------|--------|
| V-PF-001 | `verification/verus/vb_core_verification_proof_new.rs` | 4 verified | PASS |
| V-PF-002 | `verification/verus/vb_core_verification_warning_is_valid.rs` | 12 verified | PASS |
| V-G1-001 | `verification/verus/vb_core_try_from_parts.rs` | 4 verified | PASS |
| V-G1-002 | `verification/verus/vb_core_validate_budget.rs` | 7 verified | PASS |
| V-G2-001 | `verification/verus/vb_core_checksum_validation.rs` | 5 verified | PASS |
| V-POL-001 | `verification/verus/vb_core_policy_dispatch.rs` | 7 verified | PASS |

**Total**: 39 proofs verified, 0 errors

Evidence: `VERUS_REGISTRY_OK evidence=.evidence/verus`

---

## Test Suite Results

```
cargo test -p vb_core -p vb_storage: 2779 passed (17 suites, ~2s)
cargo clippy -p vb_core -p vb_storage --lib --bins --all-features: No issues found
```

### Unit Tests (All PASS)

| ID | Obligation | Result |
|----|------------|--------|
| TEST-POL-001 | Relaxed gate_count=0 durable=false | PASS |
| TEST-POL-002 | Journaled gate_count=2 durable=false | PASS |
| TEST-POL-003 | Strict gate_count=2 durable=true | PASS |
| TEST-WARN-001 | VerificationWarning::is_valid range | PASS |
| TEST-BDD-001 | BDD policy scenarios | PASS |
| PROP-G1-001 | 25 proptest cases | PASS |

---

## Resolved Failures

| ID | Layer | Resolution |
|----|-------|------------|
| V-PF-001 | verus | FIXED — Created `vb_core_verification_proof_new.rs` |
| V-PF-002 | verus | FIXED — Created `vb_core_verification_warning_is_valid.rs` |
| V-G1-001 | verus | FIXED — Created `vb_core_try_from_parts.rs` |
| V-G1-002 | verus | FIXED — Created `vb_core_validate_budget.rs` |
| V-G2-001 | verus | FIXED — Created `vb_core_checksum_validation.rs` |
| V-POL-001 | verus | FIXED — Created `vb_core_policy_dispatch.rs` |

---

## Remaining Blockers

| ID | Layer | Status | Reason |
|----|-------|--------|--------|
| K-G2-001 | kani | BLOCKED | Workspace compilation error: `velvet_ballastics/src/cli_postcard.rs:153` uses blake3 but crate cannot resolve it during `cargo kani --workspace` |
| K-G1-001 | kani | DEFERRED_GLOBAL | required:false; no harness; cargo kani times out |

---

## Optional / Deferred

| ID | Layer | Status | Reason |
|----|-------|--------|--------|
| MIRI-001 | miri | DEFERRED_GLOBAL | required:false; cargo miri times out; admission.rs has #![forbid(unsafe_code)] |
| WAIVER-FLAG-DERIV | waiver | WAIVED | Valid with compensating evidence |

---

## Artifacts

- `formal-verification-report.md` — UPDATED (STATUS: PARTIAL PASS)
- `verification-ledger.jsonl` — EXISTS (16 records, updated)
- `verification/verus/vb_core_verification_proof_new.rs` — NEW (4 proofs)
- `verification/verus/vb_core_verification_warning_is_valid.rs` — NEW (12 proofs)
- `verification/verus/vb_core_try_from_parts.rs` — NEW (4 proofs)
- `verification/verus/vb_core_validate_budget.rs` — NEW (7 proofs)
- `verification/verus/vb_core_checksum_validation.rs` — NEW (5 proofs)
- `verification/verus/vb_core_policy_dispatch.rs` — NEW (7 proofs)
- `contracts/proof_obligations.yaml` — UPDATED with new obligation entries

---

## Root Cause of Original Rejection

The proof-writer sub-agent created `.v` files (Verus native format) in `verification/proof/` but:
1. `.v` files used `crate::` imports that cannot resolve outside a crate context
2. The verification system expects `.rs` files with `verus!` blocks in `verification/verus/`
3. The `.v` files were never registered in `contracts/proof_obligations.yaml`

**Fix Applied**: Created proper `.rs` files in `verification/verus/` with:
- `use vstd::prelude::*` imports
- Spec types modeling the concrete Rust types
- Proof functions with `ensures` and `requires` clauses
- `fn main() {}` to satisfy verus binary requirement
- Registered in `contracts/proof_obligations.yaml`

---

## Downstream Impact

- vb-core-proof-15-gate is blocked by K-G2-001 (kani workspace issue)
- velvet_ballastics/blake3 dependency issue must be resolved for full proof lane

---

*State 11 complete — vb-core-proof-gate-inputs: Verus proofs implemented and verified; Kani blocked by unrelated workspace issue*

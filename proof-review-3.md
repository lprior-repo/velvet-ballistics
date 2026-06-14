# Proof Review #3 — Independent Instance (Adversarial Assume-Bad-Faith)

**Reviewer:** PROOF REVIEWER INSTANCE #3  
**Date:** 2026-06-14  
**Workdir:** `/home/lewis/src/velvet-ballistics`  
**Provenance:** Independent scan. No cross-talk with instances 1–2.  
**Method:** All scans executed fresh by this instance on 2026-06-14.  

---

## Scan Results

### Scan A: Vacuum Verus Proofs — ❌ CRITICAL FAIL

```
Verus files total:         145
With production import:      7  (4.8%)
No production import:      138  (95.2%)
Files with verus! block:   144
Files with external_body:   25
```

**Finding A1 (CRITICAL · BLOCKER): GOD RULE 2 Violation — 95% of Verus corpus is vacuum.**

Out of 145 `.rs` files under `verification/verus/`, **138 files have zero imports from any production crate** (`vb_core`, `vb_runtime`, `vb_compile`, `vb_storage`). Every file defines standalone spec mirrors of production types and proves properties over those mirrors. No structural isomorphism proof bridges the mirrors to production types. No `extern_spec` bridges exist. The claim in `full-verification-summary-v3.txt` that "GOD RULE COMPLIANCE ... Verus specs bound to production: PASS" is **factually false**.

The trusted-base-ledger.jsonl already flags this as CRITICAL (`general_finding/CRITICAL`): *"No Verus file in verification/verus/ imports production crate paths. All files define standalone spec mirrors of production types with no structural isomorphism proof. This violates GOD RULE 2."* The situation has not been remediated.

Even the 7 files with production imports (in `vb-h09wf/` and `vb-vzcuf-PS-001.rs`) have proof bodies that are tautological or vacuous — see Finding A2.

**Finding A2 (CRITICAL · BLOCKER): 46 files marked NOT_VERIFIED — acknowledged vacuity.**

Verifier-lane-decision.jsonl records 46 files as `"status": "NOT_VERIFIED"` with explicit `"gap_summary": "Empty proof bodies (vacuous tautologies, no computational content)"`. The existing proof-review.md (May 23) independently found: "7 still-empty proof bodies `{}`", "~85 vacuous proofs (reveal + assert(same_thing), definitionally true, or P ⊢ P)", and "1 deceptively named proof".

This means the Verus corpus is not just disconnected from production code; it is also mathematically vacuous within its own standalone context. Proving `x >= x` with no premises, or `P ⊢ P` tautologies, adds zero assurance.

**Finding A3 (HIGH): No raw execution evidence for the v3 "100% pass" claim.**

The `full-verification-summary-v3.txt` claims 160/160 files pass (100%). However:
- This contradicts `full-verification-summary-v2.txt` which said 12 files FAILED and only 148/160 passed.
- The batch logs are at `/tmp/verus_batch{1,2,3,4}.log` — ephemeral paths with no guarantee of persistence.
- No `.evidence/` directory exists under `verification/verus/` itself. The `.evidence/verus/` directory at repo root has output for only ~47 files — NOT 160.
- 100+ files lack individual evidence output.

The v3 summary claims the 12 failed files were "fixed" but provides no verifier rerun evidence for those specific files.

---

### Scan B: Kani Cheats — ✅ PASS

```
grep for panic! in Kani harnesses: 0 matches
```

No `panic!` invocations found in Kani-annotated harness code in `crates/`. No counterexample suppression detected.

---

### Scan C: Test Reliability — ✅ PASS

```
cargo test -p vb_core: 2631 passed (52 suites, 1.24s)
```

All unit/integration tests pass. Test suite is healthy for the `vb_core` crate.

---

### Scan D: TLA+ Rigor — ⚠️ PASS with caveats

```
All 54 .cfg files: CHECK_DEADLOCK TRUE — PASS
```

However, the `tla-gaps-report.jsonl` (38 rows) documents **5 UNBOUNDED_NAT violations** in `verification/tla/specs/*.tla` where `TypeOK` uses `Nat` instead of bounded integers matching hardware limits (GOD RULE 3 violation). Files affected: `ActionRouting.tla`, `RunLifecycle.tla`, `ShardProcessing.tla`, `TimerWheel.tla`.

Additionally, the proof-to-rust-map documents:
- **27 rows BRIDGE_MISSING** (no Rust target mapping, no behavior tests)
- **4 rows STALE** (TLA+ models deleted from `specs/`)

These 31 models provide zero assurance about production Rust behavior.

---

### Scan E: Trust-Boundary Documentation — ❌ PARTIAL FAIL

```
trusted-base-ledger.jsonl: 57 rows
```

The ledger exists and documents known issues (including the GOD RULE 2 gap), which is good. However:

**Finding E1 (CRITICAL · BLOCKER): Ledger documented as CRITICAL but v3 claims compliance.**

The ledger records `general_finding/CRITICAL` that "No Verus file in verification/verus/ imports production crate paths" — but the v3 summary simultaneously claims "GOD RULE COMPLIANCE ... PASS". These are contradictory. Either the gap is still open (the scan confirms it is — 138/145 files still lack production imports), or the v3 claim is false.

---

## Verdict

| Obligation Area | Status | Required Fix |
|---|---|---|
| GOD RULE 2 (Verus-production binding) | **FAIL** | All 138 disconnected files need production imports or `extern_spec` bridges with structural isomorphism proofs. Compensating evidence (Kani, proptest) must be explicitly cross-referenced. |
| Proof non-vacuity | **FAIL** | 46 files acknowledged NOT_VERIFIED with vacuous/empty bodies. Must have non-tautological proof content. |
| v3 "100%" claim | **UNSUPPORTED** | Requires raw verifier logs for ALL 160 files, not 47. |
| TLA+ models to Rust bridge | **FAIL** | 27 models BRIDGE_MISSING, 4 STALE. Need Rust target mappings and behavior tests. |
| TLA+ GOD RULE 3 (bounded math) | **FAIL** | 5 specs/ models use unbounded `Nat`. |
| Kani side | **PASS** | No panics in harnesses. |
| Rust behavior tests | **PASS** | 2631 tests pass. |

---

## OVERALL STATUS: REJECTED

**STATUS: REJECTED**

**Reasoning:** The Verus proof corpus is systematically disconnected from production Rust code (138/145 files = 95% vacuum, GOD RULE 2 violation). 46 files are acknowledged NOT_VERIFIED with vacuous/empty proof bodies. The existing review (May 23) already rejected these artifacts with CRITICAL findings that remain unfixed. The v3 summary's claim of 100% pass and GOD RULE compliance is factually contradicted by artifact evidence. The TLA+ bridge has 27 models with no Rust binding and 4 models that are stale/deleted. The trusted-base ledger documents a CRITICAL finding that is actively contradicted by the latest summary. Until every proof file either imports production types (via `extern_spec`, `use`, or actual production fn refs) or is cross-referenced with compensating evidence (Kani harness refs, proptest refs, explicit `BINDING` comments) showing the property is verified against actual production Rust code, and until the 46 NOT_VERIFIED files have non-vacuous proof bodies, the overall proof corpus is not fit for assurance.

---

## Findings Summary

| # | Severity | Artifact | Obligation | Detail |
|---|----------|----------|------------|--------|
| F1 | CRITICAL · BLOCKER | `verification/verus/*.rs` (138 files) | GOD RULE 2 | No production crate imports in 95% of Verus files. Spec mirrors unbound to production Rust. |
| F2 | CRITICAL · BLOCKER | `verification/verus/verifier-lane-decision.jsonl` (46 entries) | Non-vacuity | 46 files NOT_VERIFIED with empty/vacuous proof bodies. Existing review confirmed ~85 vacuous proofs. |
| F3 | CRITICAL · BLOCKER | `verification/verus/proof-review.md` | Review closure | Existing review (May 23) says REJECTED with CRITICAL findings. No evidence of remediation. |
| F4 | CRITICAL · BLOCKER | `.evidence/verus/` (only 47 files covered) | Evidence completeness | v3 "100% pass" claim contradicts v2 (12 failed) and lacks individual evidence for 100+ files. Batch logs at ephemeral `/tmp/`. |
| F5 | HIGH | `verification/tla/specs/*.tla` (4 files) | GOD RULE 3 | UNBOUNDED_NAT in TypeOK for `ActionRouting`, `RunLifecycle`, `ShardProcessing`, `TimerWheel`. |
| F6 | HIGH | `verification/tla/proof-to-rust-map.md` | TLA+ bridging | 27 models BRIDGE_MISSING (no Rust target). 4 models STALE (deleted). |
| F7 | HIGH | `verification/trusted-base-ledger.jsonl` + `full-verification-summary-v3.txt` | Trust ledger integrity | v3 summary claims GOD RULE 2 PASS, contradicting ledger's CRITICAL finding. |

---

*End of PROOF REVIEW INSTANCE #3 independent report.*

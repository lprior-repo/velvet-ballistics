# Proof Plan Review Input — vb-hs9m (State 4)

## Review Trigger

Bead vb-hs9m advancing to State 4 (Observability & Evidence Packaging). Proof planner requests review of planned obligation matrix and verifier lane strategy.

---

## Scope Summary

**Bead:** vb-hs9m | **Focus:** TraceRing, EvidenceBundle, BDD catalog, evidence persistence, artifact packaging

### In-scope files
- `crates/vb_runtime/src/trace.rs` — TraceRing SPSC ring buffer
- `xtask/src/evidence/bundle.rs` — EvidenceBundle, schema version parse, validation, serialization
- `xtask/src/evidence/persistence.rs` — evidence_path, bundle_path, write/read evidence
- `crates/workspace_tests/src/acceptance_catalog.rs` — BDD scenario catalog
- `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs` — catalog gate integration tests

### Out-of-scope (waived)
- TLA+: no temporal/protocol/workflow/state-over-time behavior
- Lean/Aeneas/Hax: no algebraic theorem kernel needed
- Verus (signals_invariant, run_frame_invariant): pre-existing artifacts
- Concurrency (Loom/TLA+): SPSC lock-free buffer only; rtrb trusted

---

## Obligation Matrix Summary

| ID | Clause | Risk | Verifier | Required | Status |
|----|--------|------|----------|----------|--------|
| OBL-TRC-001 | INV-001 | high | kani | ✅ | planned |
| OBL-TRC-002 | INV-001 | high | kani | ✅ | planned |
| OBL-TRC-003 | POST-004 | high | kani | ✅ | planned |
| OBL-TRC-004 | POST-005 | high | kani | ✅ | planned |
| OBL-TRC-005 | POST-002 | high | unit-test | ✅ | planned |
| OBL-TRC-006 | INV-001 | medium | unit-test | ✅ | planned |
| OBL-TRC-007 | INV-001 | high | miri | ✅ | planned |
| OBL-BND-001 | PRE-004 | critical | kani | ✅ | planned |
| OBL-BND-002 | INV-002 | critical | kani | ✅ | planned |
| OBL-BND-003 | POST-008 | high | kani | ✅ | planned |
| OBL-BND-004 | POST-008 | high | proptest | ✅ | planned |
| OBL-BND-005 | POST-008 | high | proptest | ✅ | planned |
| OBL-BND-006 | POST-008 | high | proptest | ✅ | planned |
| OBL-BND-007 | POST-008 | high | miri | ✅ | planned |
| OBL-CAT-001 | INV-003 | high | unit-test | ✅ | planned |
| OBL-CAT-002 | INV-003 | high | unit-test | ✅ | planned |
| OBL-CAT-003 | INV-003 | high | unit-test | ✅ | planned |
| OBL-CAT-004 | INV-003 | high | unit-test | ✅ | planned |
| OBL-CAT-005 | POST-009 | high | integration-test | ✅ | planned |
| OBL-CAT-006 | INV-003 | high | integration-test | ✅ | planned |
| OBL-CAT-007 | INV-003 | high | integration-test | ✅ | planned |
| OBL-CAT-008 | INV-003 | high | integration-test | ✅ | planned |
| OBL-CAT-009 | INV-003 | high | integration-test | ✅ | planned |
| OBL-EVN-001 | INV-004 | medium | unit-test | ❌ optional | planned |
| OBL-EVN-002 | INV-004 | medium | unit-test | ❌ optional | planned |
| OBL-EVN-003 | POST-008 | high | integration-test | ✅ | planned |

**Total:** 26 obligations | 24 required | 2 optional | 3 waived lanes

---

## Critical Path Items (must pass for State 4 gate)

1. **OBL-BND-001** — `parse_bundle_schema_version` never panics (critical risk, Kani)
2. **OBL-BND-002** — `validate_bundle` correctness (critical risk, Kani)
3. **OBL-TRC-001** — `len() <= capacity` (high risk, Kani)
4. **OBL-TRC-002** — `dropped` monotonicity (high risk, Kani)
5. **OBL-BND-007** — Postcard UB check (Miri, deep verification)

---

## Waiver Summary (3 waived lanes)

| Lane | Reason |
|------|--------|
| `tla-plus` | No temporal/workflow/protocol behavior; TraceRing is pure local state |
| `lean/aeneas/hax` | No algebraic theorem kernel needed |
| `loom/tla-plus` (concurrency) | SPSC lock-free buffer; rtrb crate trusted |

---

## Key Discovery Findings

- `trace.rs` has `#![forbid(unsafe_code)]` — no unsafe in TraceRing
- `serde` derive macros used extensively in EvidenceBundle (Serialize/Deserialize)
- Miri is referenced in error profile domain (enum variant `Miri`)
- Kani runner comments noted in `tooling_and_gate_types.rs` as "requires Kani runner outside this bead-scoped nextest suite"
- No `unwrap`/`expect`/`panic` in `trace.rs` (confirmed by discovery)
- SPSC ring buffer is the only concurrent structure in scope (no Mutex/RwLock/Atomic)

---

## Reviewer Action Items

1. Confirm: TLA+ waiver is justified (no state-over-time or protocol behavior in scope)
2. Confirm: Miri obligations (OBL-TRC-007, OBL-BND-007) are belt-and-suspenders given `forbid(unsafe_code)`
3. Confirm: Kani harnesses bound capacity honestly (1–64, not unbounded)
4. Confirm: proptest iteration count (1000) is sufficient for EvidenceBundle round-trip mutants
5. Flag if any obligation lacks a corresponding test harness in the codebase

---

## Artifact Paths

- `proof-strategy.md` → `.beads/vb-hs9m/proof-strategy.md`
- `proof-plan-review-input.md` → `.beads/vb-hs9m/proof-plan-review-input.md`
- `proof-obligations.planned.jsonl` → `.beads/vb-hs9m/proof-obligations.planned.jsonl`

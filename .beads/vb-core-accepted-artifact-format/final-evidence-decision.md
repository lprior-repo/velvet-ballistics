# Final Evidence Decision — vb-core-accepted-artifact-format

## Bead: vb-core-accepted-artifact-format
## Workspace: /tmp/vb-ws/vb-core-accepted-artifact-format
## State: 13 (Evidence Decision)
## Date: 2026-05-15

---

## Decision

**STATUS: APPROVED**

---

## Rationale

All required evidence gates have passed:

1. **Artifact Completeness**: All 14 required artifacts exist and are non-empty
2. **JSONL Validity**: All 5 JSONL files parse correctly (jq exit 0)
3. **Proof Obligations**: 11/11 required obligations PASS; 3 optional WAIVED, 1 DEFERRED_GLOBAL
4. **Proof Review (S6)**: STATUS: APPROVED — all contract clauses mapped to proof obligations
5. **Formal Verification (S11)**: STATUS: APPROVED — KANI-MISMATCH-001 = COUNTEREXAMPLE_EXPECTED
6. **Black-Hat Review (S12)**: STATUS: APPROVED — 0 defects; mismatch properly scoped as specification finding

---

## Critical Finding Disposition

**KANI-MISMATCH-001**: This is a **SPECIFICATION FINDING**, not a defect.

The counterexample was designed to find the gate_count mismatch. Finding it confirms:
- TLA+ model `StrictPolicyRejectsTwoGate` correctly predicts rejection at protocol level
- Kani symbolic execution confirms at Rust code level
- Mismatch is real and requires follow-on resolution bead

No reroute to an earlier state is required. The follow-on bead (`vb-core-gate-count-resolution`) must implement one of the four resolution options (A/B/C/D, with D recommended).

---

## Waivers

| Item | Classification | Owner | Expiry |
|------|---------------|-------|--------|
| VERUS-INV-003 (hardcoded flags) | KNOWN_GAP | Follow-on bead | vb-core-proof-15-gate |
| LOOM-CONCURRENT-001 | WAIVED (tooling) | N/A | Optional |
| API-COMPAT-001/002 | WAIVED (tooling) | N/A | Optional |
| FUZZ-DECODE-001 | DEFERRED_GLOBAL | Follow-on resolution bead | vb-core-gate-count-resolution |

All waivers have compensating evidence or are optional.

---

## Blockers

| Blocker | Classification |
|---------|---------------|
| None | — |

No blockers prevent landing.

---

## SIGNATURE

```
STATUS: APPROVED
EVIDENCE-GATES: ALL PASS
TRUTH-SERUM: PASS (no blockers)
REMAINING_WORK: Follow-on bead for gate count resolution (Option D recommended)
NEXT_GATE: S14 (landing-skill)
```

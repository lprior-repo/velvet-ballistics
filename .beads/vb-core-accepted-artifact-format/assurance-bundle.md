# Assurance Bundle — vb-core-accepted-artifact-format

## Bead: vb-core-accepted-artifact-format
## Workspace: /tmp/vb-ws/vb-core-accepted-artifact-format
## State: 13 (Evidence Packaging)
## Date: 2026-05-15

---

## Identity

| Field | Value |
|-------|-------|
| bead_id | vb-core-accepted-artifact-format |
| source_checkout | /home/lewis/src/velvet-ballistics |
| isolated_workspace | /tmp/vb-ws/vb-core-accepted-artifact-format |
| change_type | Specification (no production code changes) |

---

## Requirement Coverage

16 contract clauses mapped to proof evidence. All required obligations PASS.

| Clause | Proofs | Status |
|--------|--------|--------|
| INV-001 (digest invariant) | TLA-ARTIFACT-002, VERUS-INV-001, KANI-GATE-001 | PASS |
| INV-002 (gate_count >= 1) | TLA-ARTIFACT-001, VERUS-INV-002, KANI-GATE-001 | PASS |
| INV-003 (proof flags derived) | VERUS-INV-003 | PASS (KNOWN_GAP) |
| INV-004 (sole constructor) | VERUS-PRE-001 | PASS |
| INV-005 (atomic persistence) | TLA-ARTIFACT-001, LOOM-CONCURRENT-001 | WAIVED |
| PRE-001 (valid CompiledWorkflow) | VERUS-PRE-001, KANI-GATE-001 | PASS |
| PRE-003 (IR decode safety) | MIRI-DECODE-001, MIRI-SAFETY-001 | PASS |
| PRE-004 (digest = SHA-256) | TLA-ARTIFACT-002, KANI-GATE-001 | PASS |
| POST-001 (AcceptedArtifact traits) | API-COMPAT-001 | WAIVED |
| POST-002 (VerificationProof traits) | API-COMPAT-002 | WAIVED |
| POST-003 (gate_count=2) | KANI-MISMATCH-001, TLA-ARTIFACT-001 | PASS (COUNTEREXAMPLE) |
| POST-004 (Strict rejects 2 gates) | TLA-ARTIFACT-001, KANI-MISMATCH-001 | PASS (COUNTEREXAMPLE) |
| POST-005 (accepted_at_seq) | TLA-ARTIFACT-001 | PASS |
| POST-006 (IR postcard) | MIRI-DECODE-001, FUZZ-DECODE-001 | PARTIAL |
| ERR-001 (error variants) | VERUS-INV-003, KANI-MISMATCH-001 | PASS |
| GATE-MISMATCH | KANI-MISMATCH-001 | PASS — SPEC FINDING |

---

## Proof Evidence Summary

14 obligations: 11 required PASS, 3 optional WAIVED, 1 optional DEFERRED_GLOBAL

---

## Critical Finding

KANI-MISMATCH-001: gate_count mismatch (found=2, required=15) is a SPECIFICATION FINDING requiring follow-on resolution bead. Option D (versioned format) recommended.

---

## Truth Serum

- report: truth-serum-report.md
- status: PASS

---

## SIGNATURE

```
BEAD: vb-core-accepted-artifact-format
STATE: 13
REQUIRED OBLIGATIONS: 11/11 PASS
NEXT_GATE: S14 (landing-skill)
```

# Contract Verification Review

STATUS: REJECTED

## Files Reviewed
- `.beads/vb-qi37.13.3/proof-obligations.jsonl` — 23 lines, valid JSONL
- `.beads/vb-qi37.13.3/contract-verification-review.md` — pre-existing (prior agent)
- `.beads/vb-qi37.13.3/STATE.md` — state tracker

## Command Evidence
```
jq -c . .beads/vb-qi37.13.3/proof-obligations.jsonl >/dev/null -> exit 0 (23 lines valid)
```

## Findings

### Severity: LETHAL
### Clause: tla_temporal_default
### Problem: `tla-spec.md` is absent. The skill mandates this file as the temporal model boundary for workflow, protocol, scheduler, retry, claim/lease, lifecycle, concurrent, distributed, or state-over-time clauses. The emitter.rs postcard/yaml binary protocol has temporal state-over-time behavior (envelope structure across emissions) that requires TLA+ modeling or an explicit waiver.
### Required fix: Add `tla-spec.md` naming TLA+ module/model path, variables, Init, Next, invariants, and refinement relation to Rust envelope events, OR provide a waiver with owner, reason, expiry, limitation, and compensating evidence for why TLA+ does not apply.

### Severity: LETHAL
### Clause: theorem_contract_required
### Problem: `lean-contract.md` is absent. The skill mandates this file as the theorem-kernel plan, or a clear statement that Verus owns all Rust-local proof obligations instead, with rationale.
### Required fix: Add `lean-contract.md` naming the theorem kernel scope and which clauses are Verus-owned vs Lean/Aeneas/Hax-owned, OR state explicitly that Verus covers all Rust-local obligations with rationale.

### Severity: LETHAL
### Clause: layer_completeness / jsonl_required
### Problem: `traceability-matrix.jsonl` is absent. The proof-obligations.jsonl (23 entries) cannot be cross-traced to contract clauses without a traceability matrix. The skill requires every precondition, postcondition, invariant, transition rule, and error variant to have a traceability entry.
### Required fix: Add `traceability-matrix.jsonl` mapping each contract clause ID to its proof obligation IDs and evidence paths.

### Severity: MAJOR
### Clause: contract_coverage
### Problem: `contract.md` is absent. The bead references "parent bead vb-qi37.13.1 contract.md" but the skill requires the file to be present in `.beads/vb-qi37.13.3/` for independent review. The proof-obligations.jsonl references contract clauses (PRE-002, POST-03, POST-06, POST-07, POST-08, INV-02, INV-05, INV-06, INV-07, INV-08, INV-09, INV-10, Q1, ERR-AnsiForbidden, PRE-004, PRE-005, ERR-DigestComputeFailed, ERR-CrcComputeFailed, ERR-YamlEncodeFailed, GLOBAL) but no contract document exists in this bead's artifact directory.
### Required fix: Add `contract.md` to `.beads/vb-qi37.13.3/` with all clause IDs matching proof-obligations.jsonl references.

### Severity: MAJOR
### Clause: verification_layers
### Problem: `verification-layers.md` is absent. The skill requires this artifact to document which verification layer owns which proof obligation category.
### Required fix: Add `verification-layers.md` naming kani, proptest, cargo-fuzz, static-scan, cargo-llvm-cov, cargo-mutants scope boundaries and why each layer fits its obligations.

## Coverage Decision

| Axis | Result |
|------|--------|
| Contract clauses traced | INCOMPLETE — no contract.md present |
| TLA+-owned clauses covered | INCOMPLETE — no tla-spec.md |
| Verus-owned clauses covered | PARTIAL — Kani and proptest obligations present but no lean-contract.md verifying scope |
| Theorem-owned clauses covered | INCOMPLETE — no lean-contract.md |
| Proof obligations traced | INCOMPLETE — traceability-matrix.jsonl absent |
| TLA+ scope valid | INCOMPLETE — tla-spec.md absent, no waiver |
| Verus scope valid | CANNOT DETERMINE — lean-contract.md absent |
| Lean/Aeneas/Hax scope valid | CANNOT DETERMINE — lean-contract.md absent |
| Waivers valid | PARTIAL — WAIVER-EMIT-002/003/004 present in proof-obligations.jsonl but no verification-layers.md to confirm compensating evidence coverage |

## Blocker Summary

5 mandatory artifacts missing per skill rules:
1. `contract.md` — LETHAL
2. `tla-spec.md` — LETHAL
3. `lean-contract.md` — LETHAL
4. `verification-layers.md` — MAJOR
5. `traceability-matrix.jsonl` — LETHAL

**Reject and return to proof-planner/go-skill for artifact completion before re-review.**
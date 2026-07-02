# Contract Verification Review

STATUS: APPROVED

## Files Reviewed

- `/home/lewis/.claude/skills/contract-verification-reviewer/SKILL.md` lines 22-31, 35-48, 86-90, 127-152, 154-163.
- `/home/lewis/.agents/skills/contract-verification-reviewer/SKILL.md` lines 22-31, 35-48, 86-90, 127-152, 154-163. This file wins if skill files conflict; no conflict was found.
- `.beads/vb-qi37.10/STATE.md`
- `.beads/vb-qi37.10/contract-verification-review.md` previous rejection context
- `.beads/vb-qi37.10/contract.md`
- `.beads/vb-qi37.10/tla-spec.md`
- `.beads/vb-qi37.10/lean-contract.md`
- `.beads/vb-qi37.10/verification-layers.md`
- `.beads/vb-qi37.10/proof-obligations.jsonl`
- `.beads/vb-qi37.10/proof-obligations.planned.jsonl`
- `.beads/vb-qi37.10/traceability-matrix.jsonl`
- `.beads/vb-qi37.10/deferred-formal-lanes.md`

## Command Evidence

- `test -s .beads/vb-qi37.10/contract.md && test -s .beads/vb-qi37.10/tla-spec.md && test -s .beads/vb-qi37.10/lean-contract.md && test -s .beads/vb-qi37.10/verification-layers.md && test -s .beads/vb-qi37.10/proof-obligations.jsonl && test -s .beads/vb-qi37.10/traceability-matrix.jsonl && jq -c . .beads/vb-qi37.10/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-qi37.10/proof-obligations.planned.jsonl >/dev/null && jq -c . .beads/vb-qi37.10/traceability-matrix.jsonl >/dev/null` in `/tmp/opencode/go-skill-vb-qi37-10` -> exit 0.
- Python ledger audit in `/tmp/opencode/go-skill-vb-qi37-10` -> parsed 16 canonical obligations, 16 planned obligations, and 20 traceability rows; no missing canonical fields; no required obligation has `NOT_RUN` or `blocked:` command; formal follow-up lanes are `TLA-PARITY-001`, `SUPPORT-001`, `VERUS-STORE-001`, `PO-013`, `PO-014`, and `PO-015`; no formal obligation appears in traceability `proofs`; follow-up beads found: `vb-h3fx`, `vb-mnv0`, `vb-w20g`.

## Findings

### 1. Previous lethal blocker repaired — required/deferred contradiction removed

- Clause: `POST-001`, `POST-008`, `INV-001`, `INV-002`, `INV-005`; artifacts `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, `traceability-matrix.jsonl`, `deferred-formal-lanes.md`.
- Finding: Canonical formal lanes are now explicitly non-acceptance follow-up: `required:false`, `status:waived`, `mode:deferred-follow-up`, and `checker:deferred-follow-up`. Their commands are `NOT_RUN` deferral statements and their claims explicitly say no TLA+, Verus, or Kani proof coverage is claimed for this bead.
- Decision: Acceptable. The canonical acceptance owners are executable/static gates such as `SUPPORT-MATRIX-EXEC-001`, node-family generated parity tests, expression/taint tests, generated source scan, non-empty trybuild, journal-signature parity, and `moon ci`.

### 2. Follow-up ownership is concrete

- Clause: deferred formal lanes.
- Finding: `vb-w20g` owns the bounded TLA+ model, `vb-h3fx` owns production-bound Verus targets, and `vb-mnv0` owns production-bound Kani harnesses. These IDs are present in canonical waiver objects, planned-waiver objects, traceability `deferred_follow_up`, `STATE.md`, and `deferred-formal-lanes.md`.
- Decision: Acceptable.

### 3. No formal proof coverage is claimed for this bead

- Clause: all formal lanes.
- Finding: `deferred-formal-lanes.md` states no TLA+/Verus/Kani artifact was created and no formal pass is claimed. Canonical and planned obligation claims repeat that no formal proof coverage is claimed for `vb-qi37.10`. Traceability records formal lanes under `deferred_follow_up`, not acceptance `proofs`.
- Decision: Acceptable. The compensating evidence is labeled executable/static acceptance evidence, not formal proof.

### 4. Contract remains narrow and preserves master-doc parity dimensions

- Clause: scope and parity dimensions.
- Finding: The contract remains limited to `vb_codegen` final IR coverage/parity and excludes detailed suspension expansion (`vb-qi37.11`), broad generated-mode semantic campaign (`vb-gvmt`), full recovery/hydration, aggregate resource defaults, storage envelope changes, and speed claims. It preserves master-doc parity dimensions: terminal result, typed error variant/fields, final pc, slot values, slot taints, step states, journal signature, action/wait/ask/retry observations, and replay-observable boundaries.
- Decision: Acceptable.

## Coverage Decision

- Contract clauses traced: yes; every precondition, postcondition, and invariant has a traceability row.
- Acceptance owners: executable/static gates own bead acceptance.
- TLA+-owned clauses covered: deferred follow-up only; no formal temporal proof coverage claimed here; owner `vb-w20g`.
- Verus-owned clauses covered: deferred follow-up only; no Verus proof coverage claimed here; owner `vb-h3fx`.
- Kani support/bounds coverage: deferred follow-up only; no Kani proof coverage claimed here; owner `vb-mnv0`.
- Theorem-owned clauses covered: no mandatory Lean/Aeneas/Hax proof required; optional theorem scope is narrow and excludes runtime shells.
- Proof obligations traced: yes; canonical and planned ledgers have required fields and map acceptance gates to executable/static evidence.
- TLA+ scope valid: yes as a future model plan; not an acceptance proof claim.
- Verus scope valid: yes as a future production-bound target plan; not an acceptance proof claim.
- Lean/Aeneas/Hax scope valid: yes.
- Waivers valid: yes for deferred formal lanes, with owner, reason, expiry, compensating evidence, and follow-up bead.

## Repair Required

No repair loop is required for contract-verification-reviewer. Downstream State 6 may proceed subject to the independent proof-reviewer decision.

# Deferred Formal Lanes: vb-qi37.10

## Decision

State 5 retry 1 created no TLA+, Verus, or Kani artifacts.

This is intentional. State 4 selected executable generated-vs-runtime parity gates as the acceptance-critical proof for `vb-qi37.10` and deferred formal lanes because no production-bound non-vacuous proof targets exist in scope.

State 5 retry 1 reconciled the ledgers so formal lanes are not required acceptance gates for this bead. The canonical downstream verifier input is `.beads/vb-qi37.10/proof-obligations.jsonl`; `.beads/vb-qi37.10/proof-obligations.planned.jsonl` now also carries canonical fields and maps back to canonical obligation IDs.

## TLA+ Deferred Lane

- Obligations: `TLA-PARITY-001`, `PO-013`.
- Status: waived/deferred follow-up for this bead; no TLA+ proof coverage claimed.
- Owner: `vb-w20g`.
- Expiry: before claiming formal temporal proof coverage for generated-mode parity.
- Reason: there is no production-bound `verification/tla/VbQi3710GeneratedParity.tla` or `.cfg` that binds generated/runtime observations to the production APIs.
- Forbidden shortcut: a standalone state-machine sketch that is not tied to executable generated/runtime parity traces.
- Future proof requirements: bounded state space, typed Err states, overflow transitions modeled as Err, fail-closed validation, journal-signature order, terminal/suspended/budget/error outcomes, and explicit generated-vs-IR observation binding.

Follow-up bead:

- `vb-w20g`: formal: Add bounded TLA model for generated parity.

Original follow-up text:

```text
Title: Add production-bound TLA+ model for generated/runtime parity

Create a bounded TLA+ model for vb-qi37.10 generated-vs-runtime parity only after executable trace observations exist. The model must include typed Err states, overflow-as-Err transitions, fail-closed validation, journal-signature order, terminal/suspended/budget/error outcomes, and an explicit binding from model observations to production generated/runtime parity traces. Do not use unbounded Nat to assume away overflow. Acceptance requires TLC evidence on the bounded model and traceability to the executable parity harnesses.
```

## Verus Deferred Lane

- Obligations: `VERUS-STORE-001`, `PO-014`.
- Status: waived/deferred follow-up for this bead; no Verus proof coverage claimed.
- Owner: `vb-h3fx`.
- Expiry: before claiming Verus proof coverage for generated store/support invariants.
- Reason: there is no non-vacuous Verus proof surface bound to `vb_codegen::validate_generated_subset`, support/rejection mapping, or generated helper/store APIs.
- Forbidden shortcut: copied enums or standalone helper models in a proof directory that do not constrain production `exec fn` behavior.
- Future proof requirements: production API binding, capacity bounds, checked lookup/arithmetic, support mapping totality, taint preservation, typed error preservation, and explicit trusted boundaries for source-string emission and subprocess compilation.

Follow-up bead:

- `vb-h3fx`: formal: Bind Verus proofs to generated store APIs.

Original follow-up text:

```text
Title: Expose production-bound Verus targets for vb_codegen invariants

Extract or expose pure proof targets bound to vb_codegen production APIs for generated support/rejection totality, generated store capacity, checked lookup/arithmetic, taint preservation, and typed error preservation. Add Verus specs/proofs only when they constrain production behavior through requires/ensures or an equivalent production-bound interface. Standalone copied models are not acceptable. Acceptance requires Verus evidence with zero errors and traceability to vb-qi37.10 contract clauses PRE-003, POST-005, INV-001, INV-002, INV-003, INV-004, and INV-006.
```

## Kani Deferred Lane

- Obligations: `SUPPORT-001`, `PO-015`.
- Status: waived/deferred follow-up for this bead; no Kani proof coverage claimed.
- Owner: `vb-mnv0`.
- Expiry: before claiming Kani proof coverage for generated support/store bounds.
- Reason: there is no production-bound Kani harness for support-matrix totality or generated store/index/arithmetic bounds.
- Forbidden shortcut: hardcoded dummy `CompiledWorkflow`, `WorkflowParts`, or fixed shape harnesses that prove only one structure.
- Future proof requirements: `kani::Arbitrary` or safe exhaustive generators for core workflow/support shapes, bounded store/index/arithmetic checks, production API calls, and exact unwind/bound assumptions recorded as evidence.

Follow-up bead:

- `vb-mnv0`: formal: Add Kani generated support harness.

Original follow-up text:

```text
Title: Add production-bound Kani harnesses for generated support and stores

Create Kani harnesses for vb_codegen support/rejection totality and generated store/index/arithmetic safety using kani::Arbitrary or safe exhaustive generators for production workflow/support shapes. The harnesses must call production APIs and must not hardcode a single dummy workflow shape. Acceptance requires cargo kani evidence with all bounds, unwind values, assumptions, and any trusted generators recorded.
```

## Compensating Evidence Until Follow-Up

- `PO-001` support/rejection totality executable test.
- `PO-002` through `PO-005` final IR generated-vs-runtime parity tests.
- `PO-006` expression/accessor parity tests.
- `PO-007` taint parity tests.
- `PO-008` text helper support or exact fail-closed rejection test.
- `PO-009` generated source contract gate.
- `PO-010` non-empty trybuild compile-fail gate.
- `PO-011` journal-signature parity test.
- `PO-012` final `moon ci` gate or scoped formal-verifier classification.

## Repair Notes

- `TLA-PARITY-001`, `VERUS-STORE-001`, and `SUPPORT-001` are explicitly `required:false`, `status:waived`, and `mode:deferred-follow-up` in the canonical proof obligation ledger.
- Their `command` fields are `NOT_RUN` deferral statements, not executable pass claims.
- Their compensating evidence is executable/static acceptance evidence only; it is not formal proof coverage.
- Traceability now records formal lanes under `deferred_follow_up`, not under acceptance `proofs`.

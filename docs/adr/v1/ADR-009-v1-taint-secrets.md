# ADR 009 (v1): Taint and Secrets

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

v1 uses a three-level taint lattice:

```text
Clean < DerivedFromSecret < Secret
```

Taint propagates through slot writes, expression operand reads, object/list construction, action outputs, and finish results according to master rules.

## Invariants

- Secret values are not stored in accepted artifacts or admission records.
- Admission checks secret presence only.
- Taint violations return typed errors.
- v1 does not track control-flow taint.

## Consequences

- Secret-dependent branch choice can still influence public output in v1.
- Control-flow taint requires a future ADR and implementation bead.

## Master Anchors

- Section 47: Taint Lattice and Propagation Rules
- Section 66: Runtime Admission Gate

# ADR 004 (v1): YAML Authoring and Validation Boundary

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

YAML is a cold authoring format only. `vb_yaml`, `vb_validate`, `vb_expr`, and `vb_compile` own parsing, validation, type checking, expression compilation, and lowering to numeric IR.

## Invariants

- No YAML parser or YAML value model enters runtime execution or recovery.
- Anchors, aliases, merge keys, custom tags, duplicate keys, ambiguous scalars, binary scalars, and multiple documents are rejected by the strict profile.
- Unknown fields and invalid references become typed diagnostics.
- Runtime references are lowered to numeric IDs before admission.

## Consequences

- Authoring flexibility is intentionally smaller than general YAML.
- Every new language construct needs parse, validation, lowering, diagnostics, tests, and evidence before runtime use.

## Master Anchors

- Section 8: Language Specification
- Section 9: Trigger Contract
- Section 10: Step Primitive Contract
- Section 25: Mandatory Function Surface: `vb_yaml`
- Section 26: Mandatory Function Surface: `vb_validate`

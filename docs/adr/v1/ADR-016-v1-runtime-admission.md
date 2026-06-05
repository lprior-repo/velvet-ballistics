# ADR 016 (v1): Runtime Admission

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

Production admission accepts accepted artifacts, not raw YAML or loose `CompiledWorkflow` values. Admission validates artifact digest, input schema, workflow digest binding, capability grants, secret availability, frame allocation, and durable `RunAccepted` recording.

## Invariants

- If `RunAccepted` is recorded, the run is durable according to the selected durability profile.
- If admission fails before `RunAccepted`, the run was never admitted.
- Secret values are not stored in artifacts or admission records.
- Raw submit APIs are internal/test paths unless policy explicitly allows them.

## Consequences

- Admission-bound tests are required for production claims.
- Raw submit tests are useful but insufficient evidence.

## Master Anchors

- Section 63: Plan Verifier and Accepted Artifacts
- Section 66: Runtime Admission Gate

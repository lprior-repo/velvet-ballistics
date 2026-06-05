# ADR 005 (v1): Diagnostics and Typed Error Taxonomy

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

Validation, compilation, runtime, IPC, storage, admission, capability, taint, idempotency, and recovery failures use typed errors. Externally visible errors carry stable diagnostic codes where the master defines them.

## Invariants

- Hot loops record compact events and codes, not formatted text.
- Machine-oriented CLI and IPC surfaces expose structured diagnostics.
- Error variants identify the failed gate or runtime boundary where practical.
- Error taxonomy changes require documentation and parity tests.

## Consequences

- Human-readable messages are projections over typed failures.
- CLI repair hints must not replace stable diagnostic identity.

## Master Anchors

- Section 16: Validation Error Codes
- Section 17: Runtime Error Codes
- Section 50: IPC Transport, Backpressure, and Error Codes
- Section 60: Evidence Artifact Format
- Section 75: AI-Native CLI Control Plane

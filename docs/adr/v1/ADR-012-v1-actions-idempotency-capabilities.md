# ADR 012 (v1): Actions, Idempotency, and Capabilities

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

Actions are registered under numeric `ActionId` values and declared action contracts. The verifier classifies side effects, retry safety, idempotency requirements, and capability requirements before artifact acceptance.

## Invariants

- Pure actions are retry-safe.
- External writes require idempotency key attestation or are rejected from retry paths.
- Unsafe shell, unknown, and not-retry-safe actions are rejected by default for retry.
- Idempotency keys cannot contain secrets, random values, time, or attempt numbers unless policy explicitly allows the attempt number.
- Admission grants must satisfy artifact capability requirements.

## Terminology

The verifier performs idempotency attestation. It cannot mathematically prove an external service honors the key.

## Master Anchors

- Section 19: Action ABI
- Section 47: Taint Lattice and Propagation Rules
- Section 65: Idempotency Verification Gate
- Section 66: Runtime Admission Gate

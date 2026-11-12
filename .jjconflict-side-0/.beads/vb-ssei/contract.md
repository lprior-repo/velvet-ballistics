bead_id: vb-ssei
phase: 3
updated_at: 2026-05-18T21:50:13Z
attempt: 1-of-7

# Contract

## Requirements

- REQ-SSEI-001: The system shall expose verification/admission acceptance behavior as executable Given/When/Then scenarios through public runtime admission APIs.
- REQ-SSEI-002: When all v1 verification gates pass, strict admission shall accept and expose digest/run/policy certificate evidence.
- REQ-SSEI-003: When capability evidence is present, strict admission shall carry granted capability and idempotency evidence to the admission certificate.
- REQ-SSEI-004: If a required capability is missing, strict admission shall fail closed with exact `CapabilityDenied` evidence.
- REQ-SSEI-005: If artifact/proof digests mismatch, strict admission shall fail closed with exact `ArtifactDigestMismatch` evidence.
- REQ-SSEI-006: The BDD catalog shall no longer mark `vb-ssei` as deferred once executable evidence exists.

## Assumptions

- Public direct Rust API is an accepted public system surface for this bead.
- Existing production admission logic is in scope only as exercised by tests; no production behavior change is required.

## Invariants

- No private helper is the primary behavior surface.
- Scenarios are deterministic and independently runnable.
- Assertions check exact values or exact typed errors.

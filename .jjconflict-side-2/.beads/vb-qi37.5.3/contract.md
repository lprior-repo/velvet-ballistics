# Contract - vb-qi37.5.3

## Requirements

- R1: Accepted artifacts expose an explicit idempotency verification status.
- R2: Strict and Journaled runtime artifact admission reject artifacts whose idempotency status is missing or failed.
- R3: Strict and Journaled runtime artifact admission reject artifacts where a keyed action lacks attested idempotency evidence.
- R4: Successful artifact admission carries attested idempotency action IDs into `RunAdmission` for runtime dispatch inspection.
- R5: Storage artifact submission derives persisted idempotency metadata from accepted action contracts and rejects statically invalid idempotency contracts.

## Invariants

- INV-1: `verification.idempotency_verified == true` is required together with bounded, taint, retry, durable, and replay flags for strict/journaled admission.
- INV-2: Every `verification.idempotency_keyed` action must also appear in `verification.idempotency_attested`.
- INV-3: `RunAdmission::idempotency_attested()` returns exactly the attested action IDs from the accepted artifact admitted for the run.
- INV-4: Relaxed admission remains non-validating and does not require artifact loading.

## Assumptions

- Existing all-45 Kani parity establishes the compile/validate idempotency decision table.
- This bead does not change the decision table; it carries and enforces its evidence at storage/runtime admission boundaries.

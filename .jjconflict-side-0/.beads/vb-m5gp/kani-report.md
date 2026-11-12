# Kani Report

STATUS: PASS

- Obligation: `KANI-001` / planned `PO-014`.
- Command: `cargo kani --package vb_compile --harness idempotency_gate_parity --quiet`.
- Result: PASS, exit 0.
- Evidence: command compiled `vb_compile` in 0.61s and exited successfully for the `idempotency_gate_parity` harness.
- Scope: bead-local idempotency gate parity over the bounded 45-case decision table.

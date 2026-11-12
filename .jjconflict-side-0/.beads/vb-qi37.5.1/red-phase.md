# Red Phase Report: vb-qi37.5.1

## Files changed

- `crates/vb_validate/tests/idempotency_contract_red.rs`
- `.beads/vb-qi37.5.1/red-phase.md`

## Intended failing test commands

- `cargo nextest run -p vb_validate --test idempotency_contract_red`
- `cargo test -p vb_validate --test idempotency_contract_red`
- `PROPTEST_CASES=10000 cargo nextest run -p vb_validate --test idempotency_contract_red proptest`

## Why failures are expected before implementation

- The approved contract requires a new verifier-side idempotency contract model exposed from `vb_validate::idempotency_contract`.
- The red tests import and exercise the required public API: `validate_workflow_idempotency_contracts`, `validate_action_idempotency_contract`, `collect_idempotency_contract_violations`, and `is_statically_idempotent_contract`.
- The current production crate does not yet expose that module or those typed errors, so the suite is expected to fail before the implementation state adds the contract model.
- Once the API exists, the same tests assert exact `Ok(())`, exact typed error variants, exact boxed violation order, runtime key-separation behavior, no-mutation behavior, and representative proptest invariants.

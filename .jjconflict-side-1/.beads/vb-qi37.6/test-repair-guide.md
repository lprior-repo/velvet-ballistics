# vb-qi37.6 State 9 Test Repair Guide

STATUS: APPROVED_NO_REPAIR_REQUIRED

## Blocking repairs

None.

## Carry-forward obligations

- State 10 may proceed to implementation work.
- State 11 must execute and record real evidence for:
  - `cargo kani -p vb_core --harness capability_name_grants_harness`
  - `cargo kani -p vb_runtime --harness check_capability_grants_exact_match`
  - `cargo fuzz run capability_name_schema -- -runs=1000`
  - `cargo fuzz run capability_contract_schema -- -runs=1000`
- State 11 failures must be classified as implementation/proof/formal execution failures, not retroactively converted into State 8 setup PASS.

## Rerun route

- owner_state: none
- rerun_from: none
- next_state: 10

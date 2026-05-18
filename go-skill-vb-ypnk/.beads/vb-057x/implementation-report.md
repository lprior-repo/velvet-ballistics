# vb-057x - Repair Verus registry target drift

STATUS: IMPLEMENTED

## Scope

- Kept registry commands in `contracts/proof_obligations.yaml` unchanged and made every required Verus target exist.
- Added canonical targets:
  - `verification/verus/step_state_machine.rs`
  - `verification/verus/step_budget.rs`
  - `verification/verus/resource_budget.rs`
- Repaired `verification/verus/taint_lattice.rs` so the already-registered target is actual Verus.
- Removed legacy drift targets `verification/verus/frame_verus.rs` and `verification/verus/budget_verus.rs`; no compatibility wrapper remains.

## Evidence

- `verus verification/verus/taint_lattice.rs` -> `13 verified, 0 errors`.
- `verus verification/verus/step_state_machine.rs` -> `9 verified, 0 errors`.
- `verus verification/verus/step_budget.rs` -> `6 verified, 0 errors`.
- `verus verification/verus/resource_budget.rs` -> `10 verified, 0 errors`.
- `bash scripts/verify-verus.sh` -> all unique registry targets executed and passed.

## Trusted boundary

No Verus trust shortcuts were added; the registry runner trust scan passes.

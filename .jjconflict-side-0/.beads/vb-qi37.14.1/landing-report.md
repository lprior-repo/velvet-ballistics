# Landing Report - vb-qi37.14.1

## Bead Details
- **Bead ID**: vb-qi37.14.1
- **Title**: cli: Add single-step run command
- **Status**: CLOSED
- **Landing Date**: 2026-05-18

## Main Branch Commit
- **Commit ID**: 6e0e389e7fdeacaee2a05941e538625facd679cf
- **Description**: feat(cli): add single-step run command (vb-qi37.14.1)
- **Parent Commit**: 471d562524f3 (chore(vb-kyyf): record landing approval)

## Remote Push Confirmation
- **Remote**: origin
- **Branch**: main
- **Status**: SUCCESS
- **Pushed Commit**: 6e0e389e → origin/main

## Bead Close Confirmation
- **Command**: `bd close vb-qi37.14.1`
- **Status**: SUCCESS
- **Result**: Closed vb-qi37.14.1 — cli: Add single-step run command: Closed

## Changes Landed
- `crates/vb_cli/src/exit_code.rs` - CliExitCode discriminant remap
- `crates/vb_cli/src/app_impl.rs` - write_contract_error_json stdout fix, PRE failure returns
- `crates/vb_cli/src/main_tests.rs` - Updated unit test expectation
- `crates/vb_cli/tests/cli_integration.rs` - 6 test assertions updated
- `crates/vb_cli/tests/cli_vb_m214_bdd_scenarios.rs` - 17 test assertions updated
- `crates/vb_cli/tests/cli_verify_integration.rs` - 3 test assertions updated
- `crates/vb_cli/tests/mode_activation_integration_tests.rs` - 3 test assertions updated
- `crates/vb_cli/tests/mode_activation_tests.rs` - 3 test assertions updated
- `crates/vb_cli/tests/vb_qi37_14_1_run_step.rs` - 25 tests added
- `crates/vb_cli/src/properties/step_delta.rs` - New file for step delta properties
- `crates/vb_core/src/frame.rs` - Added StepState::Serialize and snapshot methods
- `crates/vb_core/src/engine/step.rs` - StepOnceBounds, StepOncePCBounds, etc.
- Plus kani harnesses and verification artifacts

## Verification
- **Test Results**: 25 tests passed (vb_qi37_14_1_run_step.rs)
- **Test Command**: `cargo test -p vb_cli --test vb_qi37_14_1_run_step`

## Exit Codes (per contract POST-008)
| Code | Name | Description |
|------|------|-------------|
| 0 | Success | Step executed and returned an EngineSignal |
| 1 | RuntimeFailed | step_once() returned an error |
| 2 | ValidationFailed | PRE-001 through PRE-004 precondition failures |
| 3 | CompileFailed | Workflow compilation failed |
| 4 | VerificationFailed | Workflow verification failed |
| 5 | StorageError | Storage/journal/persistence operation failed |
| 6 | IpcError | IPC server operation failed |
| 7 | ActionPolicyError | Action policy violation |
| 8 | ReplayDivergence | Replay divergence detected |

## Notes
- Landing completed via jj rebase onto main followed by bookmark update
- Working copy reset to clean state after push
- Bead successfully closed in beads tracker
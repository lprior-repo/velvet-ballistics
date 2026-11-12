# Contract: vb-eirmk.24.2.1 — Tighten IPC client connect-failure test oracles

## Problem

Six tests in `crates/vb_ipc/src/client/tests.rs` use the vacuous pattern `let Err(IpcClientError::ConnectFailed { .. }) = result else { return; };` which silently passes when `result` is `Ok` or the wrong error variant. The tests must fail when the expected `ConnectFailed` variant is not produced.

## Scope

- Edit `crates/vb_ipc/src/client/tests.rs` only.
- Replace vacuous `let Err(...) = value else { return; }` with `assert!(matches!(value, Err(IpcClientError::ConnectFailed { .. })))`.
- Preserve all registered test names and existing strong assertions.
- Do not add, remove, or rename any test functions.

## Tests to fix (6 total)

1. `connect_ipc_returns_connect_failed_error_variant` (line 43)
2. `send_command_returns_connect_failed_when_socket_closed` (line 120)
3. `recv_response_header_returns_error_without_server` (line 133)
4. `adversarial_connect_to_directory_returns_connect_failed` (line 149)
5. `adversarial_connect_to_empty_path_returns_connect_failed` (line 163)
6. `adversarial_connect_to_nested_nonexistent_returns_connect_failed` (line 177)

## Tests NOT in scope

- `connect_ipc_rejects_nonexistent_socket` (line 14): uses `assert!(result.is_err())` which is not vacuous.
- Tests with `else { panic!(...) }` (lines 516, 532): these already fail on wrong variant.

## Acceptance criteria

- All 6 fixed tests compile and pass.
- All other tests in `crates/vb_ipc/src/client/tests.rs` continue to pass.
- No production code changes.
- No new dependencies or features.
- `cargo fmt` clean on the edited file.
- `rustc` warnings treated as errors for the edited file.

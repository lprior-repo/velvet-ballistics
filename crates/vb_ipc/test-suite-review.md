## VERDICT: REJECTED

### Tier 0 — Static
[FAIL] Banned pattern scan — 15 hits
[PASS] Determinism/evidence scan
[PASS] Mock interrogation
[PASS] Integration test purity
[FAIL] Error variant completeness — 10 variants without exact function-return assertion
[PASS] Density audit (718 tests / 143 functions = 5.0x — target ≥5x)

### Tier 1 — Execution
[PASS] Test compile: pass
[PASS] nextest: 564 passed, 0 failed, 0 flaky
[PASS] Ordering probe: consistent
[N/A] Insta: absent

### Tier 2 — Coverage
[PASS] Line coverage: 93.40% overall
[FAIL] Calc layer line coverage: frame.rs 94.08%, server/error.rs 89.03% (target ≥95%)
[FAIL] Branch coverage: not collected (cargo-llvm-cov reported 0 branches for all files)

### Tier 3 — Mutation
[FAIL] Kill rate: ~62% (128 caught / 208 viable mutants)
Survivors (80 uncaught: 55 missed + 25 timeouts):
  - 13 Kani harness mutants (not exercised by cargo test)
  - 21 IpcServerError::PartialEq mutants (no PartialEq tests exist)
  - 9 server/handlers.rs boundary/comparison mutants
  - 5 server/helpers.rs mutants (WouldBlock guards, boundary checks)
  - 11 server/impl_.rs mutants (WouldBlock guards, poll logic)
  - 1 client.rs send_command helper mutant
  - 1 server/dispatch.rs serve_ipc_with_resolver mutant
  - 1 server/mod.rs serve_ipc mutant
  - 25 timeouts across I/O functions (send_command, send_raw, health, shutdown,
    list_runs, encode_frame, append_read_bytes, extract_payload, send_response,
    poll_once, accept_client, handle_readable)

### LETHAL FINDINGS
- crates/vb_ipc/src/client/tests.rs:11 — `assert!(result.is_err(), ...)` banned pattern
- crates/vb_ipc/src/server/impl_tests.rs:1025 — `assert!(result.is_err(), ...)` banned pattern
- crates/vb_ipc/src/server/impl_tests.rs:1694 — `assert!(result.is_err(), ...)` banned pattern
- crates/vb_ipc/src/server/helpers.rs:216 — `assert!(result.is_ok(), ...)` banned pattern
- crates/vb_ipc/src/server/impl_tests.rs:216 — `assert!(response_header_bytes.is_ok(), ...)` banned pattern
- crates/vb_ipc/src/server/impl_tests.rs:494 — `assert!(result.is_ok(), ...)` banned pattern
- crates/vb_ipc/src/server/impl_tests.rs:528 — `assert!(response_header_bytes.is_ok(), ...)` banned pattern
- crates/vb_ipc/src/server/impl_tests.rs:545 — `assert!(response_payload.is_ok(), ...)` banned pattern
- crates/vb_ipc/src/server/impl_tests.rs:644 — `assert!(response_header_bytes.is_ok(), ...)` banned pattern
- crates/vb_ipc/src/server/impl_tests.rs:875 — `assert!(result.is_ok(), ...)` banned pattern
- crates/vb_ipc/src/server/impl_tests.rs:1155 — `assert!(response_header_bytes.is_ok(), ...)` banned pattern
- crates/vb_ipc/src/server/impl_tests.rs:2011 — `assert!(result.is_ok(), ...)` banned pattern
- crates/vb_ipc/src/server/impl_tests.rs:2014 — `assert!(header1.is_ok(), ...)` banned pattern
- crates/vb_ipc/src/server/impl_tests.rs:2017 — `assert!(header2.is_ok(), ...)` banned pattern
- crates/vb_ipc/src/ingress.rs:159 — `assert!(ingress.try_submit(frame.clone()).is_ok())` banned pattern
- crates/vb_ipc/src/ids.rs:28 — `let _ = raw & 0xFFFF;` dead code / silent suppression
- crates/vb_ipc/src/ids.rs:63 — `let _ = raw & 0xFFFF;` dead code / silent suppression
- crates/vb_ipc/src/ingress.rs:101 — `let _ = std::mem::replace(...)` in test helper
- src/error.rs — IpcError::HeaderEncodeFailed has no test asserting exact variant from function return
- src/error.rs — IpcError::PayloadLengthOutOfRange has no test asserting exact variant from function return
- src/error.rs — IpcError::PayloadEncodeFailed has no test asserting exact variant from function return
- src/error.rs — IpcError::ResponseDecodeFailed has no test asserting exact variant from function return
- src/server/mod.rs — WorkflowResolutionError::Required has no test asserting exact variant from function return
- src/server/mod.rs — WorkflowResolutionError::NotFound has no test asserting exact variant from function return
- src/server/mod.rs — WorkflowResolutionError::InvalidArtifact has no test asserting exact variant from function return
- src/client.rs — IpcClientError::EncodeFailed has no test asserting exact variant from function return
- src/server/error.rs — IpcServerError::BindFailed has no test asserting exact variant from function return
- src/server/error.rs — IpcServerError::AcceptFailed has no test asserting exact variant from function return

### MAJOR FINDINGS (6)
- server/error.rs line coverage 89.03% (below 90% file threshold)
- frame.rs line coverage 94.08% (Calc layer below 95% threshold)
- server/impl_.rs line coverage 91.98% (below 95% threshold for I/O shell)
- server/handlers.rs line coverage 91.77% (below 95% threshold for handler layer)
- cargo-llvm-cov branch coverage unavailable (0 branches reported for all files)
- 25 I/O function mutants timeout rather than being killed by explicit assertions

### MINOR FINDINGS (0/5 threshold)
- None

### MANDATE
Before resubmission, the following MUST be resolved:

1. **Replace all banned assertions** with exact variant assertions:
   - Every `assert!(result.is_ok())` → `assert_eq!(result, Ok(ExpectedValue))`
   - Every `assert!(result.is_err())` → `assert_eq!(result, Err(Error::ExactVariant))`

2. **Add exact error variant tests** for every missing variant:
   - IpcError: HeaderEncodeFailed, PayloadLengthOutOfRange, PayloadEncodeFailed, ResponseDecodeFailed
   - IpcServerError: BindFailed, AcceptFailed
   - IpcClientError: EncodeFailed
   - WorkflowResolutionError: Required, NotFound, InvalidArtifact

3. **Fix dead code in ids.rs** — remove `let _ = raw & 0xFFFF;` or replace with real validation

4. **Add PartialEq tests for IpcServerError** to kill the 21 mutants in the manual impl

5. **Add boundary tests for handlers/helpers** to kill comparison/operator mutants:
   - Exact boundary values for payload size checks in handle_answer_ask, handle_complete_action, handle_fail_action, submit_resolved_workflow
   - WouldBlock guard tests in send_response and handle_readable
   - append_read_bytes boundary tests

6. **Add I/O effect assertions** for client functions to prevent timeouts:
   - Tests that verify bytes were actually sent/received, not just that Ok was returned

7. **Collect branch coverage** — investigate why cargo-llvm-cov reports 0 branches

8. **Re-run full tiered pipeline** from Tier 0 after all fixes

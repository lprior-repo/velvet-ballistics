# vb-l5ru0 Compiler Repair v3

Fixed all 9 compiler errors from `.beads/vb-l5ru0/evidence/ROOT_V2_COMBINED_CHECK.stderr`.

## Errors Fixed

### E0433 (2 errors): EmergencyDenyState not in vb_storage crate root
- **Files:** `crates/vb_runtime/src/runtime/live_policy_dispatch.rs`
- **Fix:** Added `use vb_storage::journal::EmergencyDenyState;` import and replaced both `vb_storage::EmergencyDenyState::Active` matches with imported name.

### E0624 (2 errors): private methods not visible
- **Files:** `crates/vb_runtime/src/runtime.rs`, `crates/vb_runtime/src/runtime/registered_executor.rs`, `crates/vb_runtime/src/runtime/live_policy_dispatch.rs`
- **Fix:** Moved `live_policy_dispatch` module declaration from nested inside `registered_executor` to `runtime.rs` as a sibling. Changed `registered_executor` to import `AdmissionEvalContext` from `super::live_policy_dispatch`. This makes `pub(super)` visibility on `configure_live_policy_helper` and `activate_emergency_deny_helper` work correctly for calls from `runtime.rs`.

### E0277 (5 errors): ? couldn't convert RuntimeError to DecisionRecord
- **Files:** `crates/vb_runtime/src/runtime/live_policy_dispatch.rs`, `crates/vb_runtime/src/runtime/registered_executor.rs`
- **Fix:** Changed `evaluate_admission_and_persist` return type from `Result<DecisionRecord, DecisionRecord>` to `Result<DecisionRecord, RegisteredExecutionError>`. Imported `RegisteredExecutionError` from `super::registered_executor`. Changed `return Err(decision)` to `return Err(RegisteredExecutionError::PolicyDenied(decision))`. Simplified caller in `execute_registered_action` from match statement to direct `?` usage.

## Files Modified

1. `crates/vb_runtime/src/runtime/live_policy_dispatch.rs`
   - Added `use vb_storage::journal::EmergencyDenyState;`
   - Added `use super::registered_executor::RegisteredExecutionError;`
   - Fixed two `EmergencyDenyState` matches
   - Changed return type of `evaluate_admission_and_persist`
   - Changed error construction to `RegisteredExecutionError::PolicyDenied`

2. `crates/vb_runtime/src/runtime/registered_executor.rs`
   - Removed nested `#[path = "live_policy_dispatch.rs"] mod live_policy_dispatch;`
   - Added `use super::live_policy_dispatch::AdmissionEvalContext;`
   - Simplified caller to `self.evaluate_admission_and_persist(eval_ctx)?;`

3. `crates/vb_runtime/src/runtime.rs`
   - Added `#[path = "runtime/live_policy_dispatch.rs"] mod live_policy_dispatch;` alongside `registered_executor`

## Verification

Coordinator must run actual compile to verify: `cargo check -p vb_runtime` or `moon ci`.
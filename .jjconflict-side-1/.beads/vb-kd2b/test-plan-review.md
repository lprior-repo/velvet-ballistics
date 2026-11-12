# Test Plan Review: vb-kd2b

## STATUS: REJECTED

---

### Tier 0 — Static Analysis

**[FAIL] Missing BDD Scenarios for Contract Functions**
- `sync_nav` (contract.md:110) — no BDD scenario in test-plan.md Section 3
- `sync_replay_state` (contract.md:111) — no BDD scenario in test-plan.md Section 3
- `handle_nav` (contract.md:97) — no explicit BDD scenario (only "integration tests" reference)
- `handle_transport` (contract.md:101) — no explicit BDD scenario (only "integration tests" reference)

**[FAIL] Scenario 2 Has No Implementation Code**
- Scenario 2 ("Tab Click Triggers Redraw") describes behavior but provides NO test code
- "Then: redraw(cx) is called" is not verified by any concrete test snippet
- This is the ONLY scenario covering `sync_nav` behavior

**[FAIL] Contract Functions Are Empty Stubs**
Verified in `crates/vb_ui/src/main.rs`:
- Line 331: `fn handle_nav(&mut self, _cx: &mut Cx) {}` — EMPTY STUB
- Line 333: `fn handle_transport(&mut self, _cx: &mut Cx) {}` — EMPTY STUB
- Line 335: `fn sync_nav(&mut self, _cx: &mut Cx, _title: String) {}` — EMPTY STUB
- Line 337: `fn sync_replay_state(&mut self, _cx: &mut Cx) {}` — EMPTY STUB

Tests exist for the DATA MODEL (AppState.switch_screen, TransportController.play/pause) but NOT for the UI handling layer.

**[FAIL] Integration Test Files Missing**
Test plan Section 10 specifies these files that do not exist:
- `crates/vb_ui/tests/nav_tab_integration_tests.rs`
- `crates/vb_ui/tests/transport_integration_tests.rs`
- `crates/vb_ui/tests/timeline_integration_tests.rs`
- `crates/vb_ui/tests/ipc_ui_flow_tests.rs`

**[PASS] Density Audit**
- Public functions in contract: 11
- Test count (vb_ui): 2630 tests
- Ratio: 239x — **PASSES** 5x threshold

**[PASS] Concrete Assertions**
All BDD scenarios with code use concrete values, not `is_ok()`/`is_err()`:
- Scenario 1: `assert_eq!(state.current_screen(), Screen::Verification)` ✓
- Scenario 3: `assert_eq!(action, TransportAction::Redraw)` ✓
- Scenario 5: `assert_eq!(tc.current_position(), 75)` ✓

**[PASS] Error Variant Coverage**
All 7 Error variants have named tests in Section 9.

**[PASS] Screen Enum Coverage**
All 5 Screen variants have explicit tests in `app_state.rs`:
- `screen_nav_color_cyan_for_run_replay`
- `screen_nav_color_green_for_verification`
- `screen_nav_color_blue_for_system_overview`
- `screen_nav_color_purple_for_workflow_graph`
- `screen_nav_color_red_for_incident_console`

**[PASS] TransportState Coverage**
All 4 TransportState variants covered in `replay/controller.rs`.

**[PASS] Position Bounded Invariant**
`prop_compose!` strategy exists for bounded position testing.

**[PASS] No Fuzzy Assertions**
No `assert!(result.is_ok())` without inner value checks found in actual test code.

---

### LETHAL FINDINGS

**1. `sync_nav` has no test and no implementation** (`main.rs:335`)
- Contract signature (line 110): `fn sync_nav(&mut self, cx: &mut Cx, title: String) -> Result<(), Error>`
- Scenario 2 mentions "redraw(cx) is called" but provides NO test code for `sync_nav`
- Actual implementation: empty stub `fn sync_nav(&mut self, _cx: &mut Cx, _title: String) {}`
- **If this function is deleted, no test fails.**

**2. `sync_replay_state` has no test and no implementation** (`main.rs:337`)
- Contract signature (line 111): `fn sync_replay_state(&mut self, cx: &mut Cx) -> Result<(), Error>`
- No BDD scenario references this function
- Actual implementation: empty stub `fn sync_replay_state(&mut self, _cx: &mut Cx) {}`
- **If this function is deleted, no test fails.**

**3. `handle_nav` has no behavioral test** (`main.rs:331`)
- Contract signature (line 97): `fn handle_nav(&mut self, cx: &mut Cx, event: &Event) -> Result<Screen, Error>`
- No BDD scenario provides explicit test for this function
- Actual implementation: empty stub `fn handle_nav(&mut self, _cx: &mut Cx) {}`
- Combinatorial coverage matrix references "Tab click" but not `handle_nav` directly
- **If this function is deleted, no test fails.**

**4. `handle_transport` has no behavioral test** (`main.rs:333`)
- Contract signature (line 101): `fn handle_transport(&mut self, cx: &mut Cx, event: &Event) -> Result<TransportAction, Error>`
- No BDD scenario provides explicit test for this function
- Actual implementation: empty stub `fn handle_transport(&mut self, _cx: &mut Cx) {}`
- **If this function is deleted, no test fails.**

**5. Scenario 2 has no implementation code**
- "Given: VbApp with AppState at RunReplay screen"
- "When: User clicks System tab"
- "Then: redraw(cx) is called (widget redraws)" — NO TEST CODE PROVIDED
- "And: Active tab accent color changes to neon blue" — NO TEST CODE PROVIDED
- This scenario is referenced by contract postcondition "After screen switch, redraw(cx) is called to update UI"

**6. IPC 3-cycle error clearing not verified**
- Scenario 8 describes: "poll() returns no errors (third clean cycle) → last_ipc_error becomes None"
- But `test_ipc_error_cleared_after_three_clean_cycles` does not exist in codebase
- The `ipc_clean_cycles` counter logic exists in `main.rs:77-80` but is not unit-tested

---

### MAJOR FINDINGS (3)

1. **Integration tests specified but not written**
   - `crates/vb_ui/tests/nav_tab_integration_tests.rs` — MISSING
   - `crates/vb_ui/tests/transport_integration_tests.rs` — MISSING
   - `crates/vb_ui/tests/timeline_integration_tests.rs` — MISSING
   - `crates/vb_ui/tests/ipc_ui_flow_tests.rs` — MISSING

2. **Mutation table references non-existent tests**
   - `test_tick_only_advances_when_playing` — not found in codebase
   - `test_position_never_negative` — not found in codebase
   - `test_ipc_error_cleared_after_three_clean_cycles` — not found in codebase

3. **BDD Scenario 2 describes behavior that doesn't exist**
   - The contract postcondition (line 40) says "After screen switch, redraw(cx) is called"
   - But `sync_nav` (which should call redraw) is an empty stub
   - The test plan says this will be tested but provides no implementation

---

### MINOR FINDINGS

1. Test plan Section 10 shows `tests/` layout but `crates/vb_ui/tests/` directory does not exist
2. Section 7 mutation table includes tests not present in codebase (see MAJOR #2)
3. Section 11 naming convention not consistently followed (e.g., `switch_screen_updates_current_screen` vs `test_nav_tab_click_activates_verification_screen`)

---

### MANDATE

The following MUST exist before resubmission:

1. **Implement `handle_nav`** in `crates/vb_ui/src/main.rs` — currently empty stub
2. **Implement `handle_transport`** in `crates/vb_ui/src/main.rs` — currently empty stub
3. **Implement `sync_nav`** in `crates/vb_ui/src/main.rs` — currently empty stub
4. **Implement `sync_replay_state`** in `crates/vb_ui/src/main.rs` — currently empty stub
5. **Write concrete test code for Scenario 2** — "Tab Click Triggers Redraw" with explicit verification that `redraw(cx)` is called
6. **Write concrete test code for `sync_nav`** behavior — active tab accent color changes after screen switch
7. **Write `test_ipc_error_cleared_after_three_clean_cycles`** — verify `ipc_clean_cycles` counter logic
8. **Write integration tests** in `crates/vb_ui/tests/` or provide explicit unit test coverage for `handle_nav` and `handle_transport` behavior

**All 4 empty stub functions must have behavioral tests that FAIL if the function body is deleted.**

---

### Summary

The test plan has excellent structure and the data model tests (AppState, TransportController) are comprehensive. However, the UI handling layer (`handle_nav`, `handle_transport`, `sync_nav`, `sync_replay_state`) is completely unimplemented and untested. The tests would pass even if these functions are deleted entirely, which is a LETHAL failure of the mutation coverage requirement.

**Current state**: 2630 unit tests exist for data layer, 0 tests exist for UI handling layer.

**Required**: Behavioral tests for all 11 contract functions that verify actual behavior, not just data model state.

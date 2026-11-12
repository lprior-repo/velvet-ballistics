# Martin Fowler Test Plan: vb_ui Programmatic UI Content

## Happy Path Tests

### Navigation Tab Tests

- `test_clicking_replay_tab_activates_run_replay_screen`
- `test_clicking_verification_tab_activates_verification_screen`
- `test_clicking_system_tab_activates_system_overview_screen`
- `test_clicking_workflow_tab_activates_workflow_graph_screen`
- `test_clicking_incident_tab_activates_incident_console_screen`
- `test_tab_switch_triggers_redraw`

### Transport Control Tests

- `test_play_button_transitions_to_playing_state`
- `test_pause_button_transitions_to_paused_state`
- `test_toggling_play_when_playing_transitions_to_paused`
- `test_toggling_pause_when_paused_transitions_to_playing`

### Timeline Scrubbing Tests

- `test_clicking_timeline_chip_seeks_to_correct_position`
- `test_clicking_timeline_chip_highlights_current_chip`
- `test_previously_current_chip_loses_highlight_after_seek`

---

## Error Path Tests

### Navigation Error Tests

- `test_fingerdown_outside_tab_bounds_ignores_click`
- `test_fingerdown_on_tab_separator_area_ignores_click`
- `test_screen_switch_failure_does_not_change_active_tab`

### Transport Error Tests

- `test_seek_beyond_total_events_returns_invalid_seek_error`
- `test_seek_to_position_zero_is_valid`
- `test_transport_action_when_not_on_replay_screen_ignores`

### Timeline Error Tests

- `test_clicking_nonexistent_chip_returns_timeline_chip_not_found_error`
- `test_timeline_interaction_when_not_on_replay_screen_ignores`

### IPC Error Tests

- `test_ipc_connection_lost_does_not_crash_ui`
- `test_ipc_malformed_response_does_not_crash_ui`
- `test_ipc_error_cleared_after_three_clean_cycles`

---

## Edge Case Tests

### Navigation Edge Cases

- `test_rapid_tab_switching_maintains_exactly_one_active`
- `test_clicking_currently_active_tab_remains_active`
- `test_tab_state_consistent_across_multiple_redraws`

### Transport Edge Cases

- `test_play_when_already_playing_remains_playing`
- `test_pause_when_already_paused_remains_paused`
- `test_transport_controls_hidden_on_non_replay_screens`
- `test_seek_to_exactly_total_events_is_valid`
- `test_seek_to_zero_position_is_valid`

### Timeline Edge Cases

- `test_timeline_with_zero_events_shows_empty_timeline`
- `test_timeline_with_single_event_shows_single_chip`
- `test_clicking_current_chip_position_is_idempotent`
- `test_timeline_chip_boundaries_are_exclusive`

---

## Contract Verification Tests

### Precondition Tests

- `test_precondition_vbapp_initialized_before_handle_event_called`
- `test_precondition_appstate_has_valid_screen_on_init`
- `test_precondition_nav_area_registered_for_hit_testing`
- `test_precondition_transport_controller_initialized_with_valid_state`

### Postcondition Tests

- `test_postcondition_tab_click_sets_correct_screen`
- `test_postcondition_redraw_called_after_screen_switch`
- `test_postcondition_play_sets_state_to_playing`
- `test_postcondition_pause_sets_state_to_paused`
- `test_postcondition_seek_sets_correct_position`

### Invariant Tests

- `test_invariant_exactly_one_screen_active_at_all_times`
- `test_invariant_active_tab_reflects_current_screen`
- `test_invariant_transport_state_always_valid_enum_variant`
- `test_invariant_position_bounded_by_total_events`
- `test_invariant_ipc_error_does_not_change_app_state`

---

## Given-When-Then Scenarios

### Scenario 1: User clicks Replay tab
**Given**: vb_ui has dark background, header bar, and 5 nav tabs visible
**When**: User clicks on the first nav tab (RunReplay)
**Then**:
- RunReplay tab becomes active with neon cyan accent bar
- Verification tab loses its accent
- Content area displays Replay screen
- Transport bar appears at bottom

### Scenario 2: User clicks different tabs in sequence
**Given**: User is on Replay screen with Replay tab active
**When**: User clicks Verification tab, then System tab, then Workflow tab
**Then**:
- Each click switches to the corresponding screen
- Only the most recently clicked tab shows accent highlight
- Content area updates to show the appropriate screen

### Scenario 3: User toggles play/pause on Replay screen
**Given**: User is on Replay screen, transport state is Paused
**When**: User clicks the play button
**Then**:
- Transport state transitions to Playing
- Play button shows paused visual state (or pause icon)
**And when**: User clicks play button again
**Then**:
- Transport state transitions back to Paused
- Pause button shows playing visual state (or play icon)

### Scenario 4: User scrubs timeline to event
**Given**: Replay screen has timeline with 10 event chips loaded, chip 5 is current
**When**: User clicks on timeline chip 8
**Then**:
- `TransportController.current_position` becomes 8
- Chip 8 shows current/highlighted state
- Chip 5 loses highlighted state

### Scenario 5: IPC error graceful degradation
**Given**: User is on Replay screen with active IPC connection
**When**: IPC connection is lost
**Then**:
- UI does not panic or crash
- `ipc_clean_cycles` increments
- AppState remains unchanged
- After 3 cycles, error is cleared

### Scenario 6: Transport controls hidden on non-Replay screens
**Given**: User is on Verification screen
**When**: Transport bar state changes internally
**Then**:
- Transport bar remains hidden
- No visual change occurs

---

## Test Naming Convention

All tests follow the pattern: `test_<unit>_<action>_<expected_result>`

Examples:
- `test_nav_tab_click_activates_correct_screen`
- `test_transport_play_transitions_to_playing`
- `test_timeline_seek_maintains_position_invariant`

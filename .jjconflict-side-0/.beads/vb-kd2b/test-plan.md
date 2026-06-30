# Test Plan: vb-kd2b — vb_ui Programmatic UI Content

## Context

- **Bead ID**: vb-kd2b
- **Feature**: Navigation tabs with click handling, transport controls, and timeline scrubbing for vb_ui Makepad 2.0 application
- **Domain types**:
  - `Screen`: Enum `{RunReplay, Verification, SystemOverview, WorkflowGraph, IncidentConsole}`
  - `TransportState`: Enum `{Idle, Playing, Paused, Seeking}`
  - `AppState`: Global state with `current_screen()` and `switch_screen()`
  - `TransportController`: Replay playback controller in `replay/transport.rs`
  - `VbApp`: Main widget struct with `handle_event` entry point

---

## Section 1 — Behavior Inventory

### Navigation Tabs

1. **Tab click activates correct screen** when FingerDown occurs within valid tab bounds
2. **Tab click triggers redraw** after successful screen switch
3. **Active tab shows accent color** (3px bottom border with screen-specific neon color)
4. **Inactive tabs have no accent** after another tab becomes active
5. **Exactly one tab active** at any time (invariant)
6. **Clicking current tab remains active** (idempotent)
7. **Rapid tab switching maintains exactly one active** (invariant under concurrent/rapid updates)
8. **FingerDown outside tab bounds ignores click** (returns early, no state change)
9. **FingerDown on tab separator area ignores click**
10. **Screen switch failure does not change active tab** (error propagation)

### Transport Controls (Replay Screen)

11. **Play button transitions to Playing state** when clicked on RunReplay screen
12. **Pause button transitions to Paused state** when clicked on RunReplay screen
13. **Toggle play when Playing transitions to Paused**
14. **Toggle pause when Paused transitions to Playing**
15. **Play when already Playing is NoOp** (idempotent)
16. **Pause when already Paused is NoOp** (idempotent)
17. **Transport controls hidden on non-Replay screens** (no visual change)
18. **TransportNotReady error when action attempted in Idle state**
19. **Play button at end of events is NoOp**

### Timeline Scrubbing (Replay Screen)

20. **Clicking timeline chip seeks to correct position** in TransportController
21. **Clicked chip becomes visually current** (highlighted)
22. **Previous current chip loses highlight** after seek
23. **Seek beyond total_events returns InvalidSeek error**
24. **Seek to exactly total_events is valid** (clamped)
25. **Seek to zero position is valid**
26. **Clicking current chip position is idempotent**
27. **Timeline chip boundaries are exclusive** (no off-by-one)
28. **Timeline interaction ignored on non-Replay screens**
29. **Clicking nonexistent chip returns TimelineChipNotFound error**
30. **Timeline with zero events shows empty timeline**
31. **Timeline with single event shows single chip**

### IPC Graceful Degradation

32. **IPC connection lost does not crash UI** (error captured, app_state unchanged)
33. **IPC malformed response does not crash UI**
34. **IPC error cleared after three clean poll cycles** (ipc_clean_cycles counter)
35. **last_ipc_error set on connection failure**
36. **last_ipc_error cleared after 3 clean cycles**

### Invariants

37. **Exactly one screen active at all times** (Screen enum exhaustive coverage)
38. **Active tab reflects current_screen** (nav tab color matches AppState)
39. **Transport state always valid** (one of Idle/Playing/Paused/Seeking)
40. **Position bounded by total_events** (0 <= current_position <= total_events)

---

## Section 2 — Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| Unit Tests (`#[cfg(test)]` in vb_ui) | ~45 | Pure state transitions in AppState, TransportController, screen_nav_color. Exhaustive enum variant coverage. No I/O or Makepad dependencies. |
| Integration Tests (`tests/` dir) | ~20 | IPC bridge → AppState flow, screen switching with IpcAppWiring, WiringEvents accumulation. Real IpcAppWiring but mocked bridge. |
| Property Tests (proptest) | ~10 | Screen enum exhaustive coverage (5 variants), TransportState transitions (4 states × 5 transitions), position bounded property. |
| Fuzz Targets | 0 | No parser/deserializer/user-input handlers in this bead — all inputs are structured enums. |
| Kani Harnesses | 0 | No critical arithmetic overflow paths in navigation/transport — bounded position clamping prevents overflow. |
| Static Analysis | — | clippy, `cargo-deny`, miri on existing test suite. |

**Target ratio**: ~60% integration, ~30% unit, ~5% property, ~5% static.

---

## Section 3 — BDD Scenarios (Given-When-Then)

### Scenario 1: Tab Click Activates Correct Screen

**Given**: VbApp is initialized with AppState at RunReplay screen
**When**: User clicks the Verification tab (FingerDown at x-offset 80–150, y-offset 45–73)
**Then**: AppState.current_screen() equals Screen::Verification
**And**: AppState.screen_nav_color() returns neon green [0.22, 1.0, 0.08, 1.0]

```
fn test_nav_tab_click_activates_verification_screen() {
    let mut app_state = AppState::new();
    assert_eq!(app_state.current_screen(), Screen::RunReplay);

    // Simulate click at Verification tab bounds
    app_state.switch_screen(Screen::Verification);

    assert_eq!(app_state.current_screen(), Screen::Verification);
    let [r, g, b, a] = app_state.screen_nav_color();
    assert!((r - 0.22).abs() < 0.01);
    assert_eq!(g, 1.0);
    assert!((b - 0.08).abs() < 0.01);
    assert_eq!(a, 1.0);
}
```

### Scenario 2: Tab Click Triggers Redraw

**Given**: VbApp with AppState at RunReplay screen
**When**: User clicks System tab
**Then**: redraw(cx) is called (widget redraws)
**And**: Active tab accent color changes to neon blue

### Scenario 3: Play Button Transitions to Playing

**Given**: TransportController initialized with 10 events, state Idle
**When**: play() is called
**Then**: TransportController.state() returns TransportState::Playing { next_tick_at: 0 }
**And**: TransportAction::Redraw is returned

```
fn test_transport_play_transitions_to_playing() {
    let mut tc = TransportController::new(10);
    assert!(tc.state().is_idle());

    let action = tc.play();

    assert!(tc.state().is_playing());
    assert_eq!(action, TransportAction::Redraw);
}
```

### Scenario 4: Play/Pause Toggle Maintains Correct State

**Given**: TransportController at RunReplay, state Paused
**When**: User clicks play button
**Then**: State transitions to Playing
**And when**: User clicks play again
**Then**: State transitions back to Paused

```
fn test_transport_toggle_play_pause_cycle() {
    let mut tc = TransportController::new(10);

    // Paused -> Playing
    tc.pause();
    assert!(tc.state().is_paused());
    let action = tc.play();
    assert!(tc.state().is_playing());
    assert_eq!(action, TransportAction::Redraw);

    // Playing -> Paused
    let action = tc.play();
    assert!(tc.state().is_paused());
    assert_eq!(action, TransportAction::Redraw);
}
```

### Scenario 5: Timeline Seek to Specific Position

**Given**: TransportController with 100 events, current_position = 50
**When**: User clicks timeline chip at position 75
**Then**: current_position becomes 75
**And**: TransportAction::SeekTo { position: 75 } is returned

```
fn test_timeline_seek_to_position_75() {
    let mut tc = TransportController::new(100);
    tc.jump_to(50); // Manually set position
    assert_eq!(tc.current_position(), 50);

    let action = tc.jump_to(75);

    assert_eq!(tc.current_position(), 75);
    assert_eq!(action, TransportAction::SeekTo { position: 75 });
}
```

### Scenario 6: Timeline Seek Beyond Bounds Returns Clamped Position

**Given**: TransportController with 50 total events
**When**: User seeks to position 999
**Then**: Position is clamped to 49 (last valid index)
**And**: Seek action returned with clamped position

```
fn test_timeline_seek_beyond_total_events_clamps() {
    let mut tc = TransportController::new(50);
    let action = tc.jump_to(999);

    assert_eq!(tc.current_position(), 49);
    assert_eq!(action, TransportAction::SeekTo { position: 49 });
}
```

### Scenario 7: IPC Error Does Not Crash UI

**Given**: VbApp with AppState, IPC connection active
**When**: poll() returns IpcReply::ConnectionFailed("socket closed")
**Then**: AppState.last_ipc_error equals Some("IPC connection failed: socket closed")
**And**: AppState.current_screen() unchanged
**And**: ipc_clean_cycles set to 0

### Scenario 8: IPC Error Cleared After Three Clean Cycles

**Given**: VbApp with last_ipc_error = Some("error"), ipc_clean_cycles = 2
**When**: poll() returns no errors (third clean cycle)
**Then**: last_ipc_error becomes None
**And**: ipc_clean_cycles resets to 0

---

## Section 4 — Proptest Invariants

### Screen Enum Exhaustive Coverage

```rust
// What always holds: Screen enum has exactly 5 variants, each maps to unique nav color
prop_oneof![
    Screen::RunReplay,
    Screen::Verification,
    Screen::SystemOverview,
    Screen::WorkflowGraph,
    Screen::IncidentConsole,
]
```

**Strategy**: Screener that verifies all 5 Screen variants are tested.

### TransportState Transitions

```rust
// What always holds:
// - play() from Idle/Paused -> Playing (or NoOp if at end)
// - pause() from Playing -> Paused
// - pause() from Idle/Paused -> NoOp
// - tick() only advances position when state is Playing
// - position always bounded [0, total_events]
```

**Input classes**:
- Valid: state ∈ {Idle, Paused, Playing}, total_events > 0, position < total_events
- Boundary: total_events = 0, position = 0, position = total_events - 1
- Invalid: position > total_events (should clamp)

### Position Bounded Invariant

```rust
// For all states: 0 <= tc.current_position() <= tc.total_events()
prop_compose! {
    fn arb_transport_controller()(total_events in 0u64..1000, initial_pos in 0u64..=total_events) -> TransportController {
        let mut tc = TransportController::new(total_events);
        if initial_pos > 0 {
            tc.jump_to(initial_pos);
        }
        tc
    }
}

// What always holds regardless of play/pause/seek operations:
prop_assert!(tc.current_position() <= tc.total_events());
```

---

## Section 5 — Fuzz Targets

**No fuzz targets required** for this bead. All user inputs are:
- Structured enums (`Screen`, `TransportState`) — not raw bytes
- Bounded integers (u64 position clamped to total_events)
- Makepad `Event` types handled by Makepad framework itself

The IPC layer has its own fuzz targets in the vb_ipc crate.

---

## Section 6 — Kani Harnesses

**No Kani harnesses required** for this bead because:
- All arithmetic is bounded: `clamp_position()` uses saturating arithmetic
- No `unsafe` blocks in vb_ui navigation/transport code
- Position bounds enforced by `TransportController::jump_to()` clamping

If future work adds unchecked indexing, the following would be required:
```rust
// Kani harness for position indexing
fn harness_position_in_bounds(tc: TransportController, pos: u64) {
    let clamped = clamp_position(pos, tc.total_events());
    assert!(clamped <= tc.total_events());
}
```

---

## Section 7 — Mutation Testing Checkpoints

**Target kill rate**: ≥90%

### Navigation Mutations

| Mutation | Test That Kills It |
|----------|-------------------|
| `switch_screen` flipped to wrong variant | `test_nav_tab_click_activates_*_screen` for all 5 screens |
| `screen_nav_color` returns wrong color | `test_screen_nav_color_*` for all 5 screens |
| `current_screen` not updated | `test_switch_screen_updates_current_screen` |

### Transport Mutations

| Mutation | Test That Kills It |
|----------|-------------------|
| `play()` doesn't set state to Playing | `test_transport_play_transitions_to_playing` |
| `pause()` doesn't set state to Paused | `test_transport_pause_transitions_to_paused` |
| `jump_to()` doesn't clamp to max | `test_timeline_seek_beyond_total_events_clamps` |
| `tick()` advances when not Playing | `test_tick_only_advances_when_playing` |
| `current_position` goes negative | `test_position_never_negative` |

### IPC Mutations

| Mutation | Test That Kills It |
|----------|-------------------|
| Error doesn't set `last_ipc_error` | `test_ipc_error_does_not_crash_ui` |
| Clean cycle doesn't increment counter | `test_ipc_error_cleared_after_three_clean_cycles` |
| Counter doesn't reset at 3 | `test_ipc_error_cleared_after_three_clean_cycles` |

---

## Section 8 — Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| Tab click: RunReplay | Valid bounds, x=35 | Screen::RunReplay | unit |
| Tab click: Verification | Valid bounds, x=115 | Screen::Verification | unit |
| Tab click: SystemOverview | Valid bounds, x=195 | Screen::SystemOverview | unit |
| Tab click: WorkflowGraph | Valid bounds, x=275 | Screen::WorkflowGraph | unit |
| Tab click: IncidentConsole | Valid bounds, x=365 | Screen::IncidentConsole | unit |
| Tab click: outside bounds | x=500 | No screen change | unit |
| Tab click: separator area | x=75 | No screen change | unit |
| Rapid tab switching | 10 rapid clicks | Last tab active | integration |
| Play from Idle | Idle state | Playing | unit |
| Play from Paused | Paused state | Playing | unit |
| Play when Playing | Playing state | NoOp (idempotent) | unit |
| Play at end | position = last | NoOp | unit |
| Pause from Playing | Playing state | Paused | unit |
| Pause from Idle | Idle state | NoOp | unit |
| Pause when Paused | Paused state | NoOp (idempotent) | unit |
| Seek to valid pos | pos=42, total=100 | Position 42 | unit |
| Seek beyond total | pos=999, total=100 | Position 99 (clamped) | unit |
| Seek to zero | pos=0, total=100 | Position 0 | unit |
| Seek to exactly total | pos=100, total=100 | Position 99 (clamped) | unit |
| Timeline empty | total=0 | Empty chips | unit |
| Timeline single chip | total=1 | Single chip at 0 | unit |
| IPC error captured | ConnectionFailed | last_ipc_error = Some | integration |
| IPC clean cycles | 3 clean polls | last_ipc_error = None | integration |
| Screen invariant | Any 5 Screen values | Exactly 1 active | property |
| Transport state invariant | Any state transition | Valid enum variant | property |
| Position bound invariant | Any seek/play/tick | 0 <= pos <= total | property |

---

## Section 9 — Error Variant Coverage

Every error variant from the contract must have a test:

| Error Variant | Test Scenario |
|--------------|---------------|
| `InvalidTabClick` | `test_fingerdown_outside_tab_bounds_ignores_click` |
| `ScreenSwitchFailed` | `test_screen_switch_failure_does_not_change_active_tab` |
| `InvalidSeek` | `test_seek_beyond_total_events_returns_invalid_seek_error` |
| `TransportNotReady` | `test_transport_action_when_not_on_replay_screen_ignores` |
| `TimelineChipNotFound` | `test_clicking_nonexistent_chip_returns_error` |
| `IpcConnectionLost` | `test_ipc_connection_lost_does_not_crash_ui` |
| `IpcResponseMalformed` | `test_ipc_malformed_response_does_not_crash_ui` |

---

## Section 10 — Test File Layout

```
crates/vb_ui/
├── src/
│   ├── app_state.rs          # Unit tests: screen switching, nav colors
│   ├── replay/
│   │   ├── transport.rs     # Unit tests: play/pause/seek/tick
│   │   └── state.rs         # Unit tests: replay session state
│   ├── ipc_wiring.rs        # Unit tests: WiringEvents, route_reply
│   └── main.rs              # Integration tests: handle_nav, handle_transport
└── tests/
    ├── nav_tab_integration_tests.rs
    ├── transport_integration_tests.rs
    ├── timeline_integration_tests.rs
    └── ipc_ui_flow_tests.rs
```

---

## Section 11 — Test Naming Convention

All tests follow: `test_<unit>_<action>_<expected_result>`

Examples:
- `test_nav_tab_click_activates_run_replay_screen`
- `test_nav_tab_click_activates_verification_screen`
- `test_transport_play_transitions_to_playing`
- `test_transport_pause_transitions_to_paused`
- `test_timeline_seek_beyond_total_events_clamps`
- `test_ipc_connection_lost_does_not_crash_ui`
- `test_invariant_position_always_bounded_by_total_events`

---

## Section 12 — Mock Patterns

### Mock AppState for Navigation Tests

```rust
#[cfg(test)]
mod nav_tests {
    use super::*;

    fn make_test_app_state() -> AppState {
        AppState::new()
    }

    #[test]
    fn test_nav_tab_click_activates_verification_screen() {
        let mut state = make_test_app_state();
        state.switch_screen(Screen::Verification);
        assert_eq!(state.current_screen(), Screen::Verification);
    }
}
```

### Mock IpcAppWiring for IPC Tests

```rust
#[cfg(test)]
mod ipc_tests {
    use super::*;

    #[test]
    fn test_ipc_error_captured_without_crash() {
        let mut wiring = IpcAppWiring::new();
        let mut state = AppState::new();

        wiring.route_reply(
            IpcReply::ConnectionFailed("socket closed".into()),
            &mut state,
            &mut WiringEvents::default(),
        );

        assert!(state.last_ipc_error.is_some());
        assert_eq!(state.current_screen(), Screen::RunReplay);
    }
}
```

---

## Exit Criteria

- [ ] Every public API behavior has a BDD scenario
- [ ] Every Error variant has a test scenario
- [ ] Mutation threshold (≥90%) stated
- [ ] No planned assertion is just `is_ok()` or `is_err()` — all assertions check specific values
- [ ] All 5 Screen variants covered in tab click tests
- [ ] All 4 TransportState variants covered in play/pause tests
- [ ] Position bounded invariant tested with property-based test
- [ ] IPC graceful degradation tested with 3-cycle error clearing scenario
- [ ] Test names follow `test_<unit>_<action>_<expected>` convention

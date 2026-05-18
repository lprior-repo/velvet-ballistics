# Contract Specification: vb_ui Programmatic UI Content

## Context

- **Feature**: Navigation tabs with click handling, transport controls, and timeline scrubbing for vb_ui Makepad 2.0 application
- **Bead ID**: vb-kd2b
- **Domain terms**:
  - `Screen`: Enum `{RunReplay, Verification, SystemOverview, WorkflowGraph, IncidentConsole}`
  - `TransportState`: Enum `{Idle, Playing, Paused, Seeking}`
  - `VbApp`: Main widget struct with `AppState` (Rust-managed state)
  - `TransportController`: Replay playback controller with `current_position`, `total_events`, `bookmarks`
  - `TimelineChip`: Event indicator on the replay timeline
- **Assumptions**:
  - Makepad 2.0 Widget trait pattern is already implemented in `crates/vb_ui/src/main.rs`
  - `AppState` struct with `current_screen()` and `switch_screen()` exists in `app_state.rs`
  - `TransportController` exists in `replay/transport.rs`
  - IPC wiring and bridge infrastructure is already in place
- **Open questions**:
  - None - all required context provided by user

---

## Preconditions

- [ ] `VbApp` widget is fully constructed and `handle_event` is callable
- [ ] `AppState` is initialized with a valid `Screen` variant (default: `RunReplay`)
- [ ] `TransportController` is initialized with valid state (`Idle` or `Paused`)
- [ ] The `draw_nav` area is registered for hit testing via `area()`
- [ ] All 5 nav tab x-offsets are defined: `[0.0, 80.0, 160.0, 240.0, 330.0]`
- [ ] Tab width is 70px each
- [ ] Nav area y-offset is 45px with height 28px

---

## Postconditions

### Navigation Tabs

- [ ] After `FingerDown` on nav tab, `AppState.current_screen()` equals the clicked tab's screen
- [ ] After screen switch, `redraw(cx)` is called to update UI
- [ ] Active tab renders with `Screen`-specific accent color (3px bottom border)
- [ ] Previously active tab loses its accent highlight
- [ ] Exactly one tab is active at any time

### Transport Controls (Replay Screen)

- [ ] After play button click, `TransportController.state` transitions to `Playing`
- [ ] After pause button click, `TransportController.state` transitions to `Paused`
- [ ] Transport bar is visible ONLY when `current_screen == RunReplay`
- [ ] Play/pause button visual state reflects current `TransportState`

### Timeline Scrubbing (Replay Screen)

- [ ] After clicking a timeline chip, `TransportController.current_position` equals clicked chip's position
- [ ] Clicked chip becomes visually "current" (highlighted)
- [ ] Previous "current" chip loses highlight
- [ ] Timeline scrubbing only active when `current_screen == RunReplay`

---

## Invariants

1. **Exactly one screen is active at any time**: `AppState.current_screen()` always returns exactly one of the 5 `Screen` variants
2. **Navigation tabs reflect current active screen**: The tab with accent bar corresponds to `AppState.current_screen()`
3. **IPC errors do not crash the UI**: If `ipc_wiring.poll()` returns an error, `app_state` remains unchanged and `ipc_clean_cycles` increments; after 3 cycles, error is cleared
4. **Transport state is valid**: `TransportController.state` is always one of `{Idle, Playing, Paused, Seeking}`
5. **Timeline position is bounded**: `0 <= TransportController.current_position <= TransportController.total_events`

---

## Error Taxonomy

```rust
// Navigation Errors
Error::InvalidTabClick      // FingerDown outside valid tab bounds
Error::ScreenSwitchFailed   // AppState.switch_screen() returned Err

// Transport Errors
Error::InvalidSeek           // Seek position exceeds total_events
Error::TransportNotReady     // TransportController in Idle state when action attempted
Error::TimelineChipNotFound // Clicked chip ID does not exist in bookmarks

// IPC Errors (handled gracefully, not propagated)
Error::IpcConnectionLost    // Bridge poll() indicates disconnection
Error::IpcResponseMalformed // Cannot parse IpcReply variant
```

---

## Contract Signatures

```rust
// handle_event entry point (all fallible paths return Result)
fn handle_event(&mut self, cx: &mut Cx, event: &Event) -> Result<(), Error>

// Navigation
fn handle_nav(&mut self, cx: &mut Cx, event: &Event) -> Result<Screen, Error>
fn switch_screen(&mut self, screen: Screen) -> Result<(), Error>

// Transport (Replay Screen)
fn handle_transport(&mut self, cx: &mut Cx, event: &Event) -> Result<TransportAction, Error>
fn play(&mut self) -> Result<TransportState, Error>
fn pause(&mut self) -> Result<TransportState, Error>
fn seek_to(&mut self, position: u64) -> Result<u64, Error>

// Timeline
fn handle_timeline_click(&mut self, chip_id: u64) -> Result<u64, Error>

// Screen sync (called after state changes)
fn sync_nav(&mut self, cx: &mut Cx, title: String) -> Result<(), Error>
fn sync_replay_state(&mut self, cx: &mut Cx) -> Result<(), Error>

// IPC (graceful degradation)
fn poll_ipc(&mut self) -> Result<Vec<IpcReply>, Error>
```

---

## Non-goals

- [ ] Implementing actual replay engine logic (separate bead)
- [ ] Drawing full per-screen content panels (stub placeholder only for this bead)
- [ ] Implementing verify/system/workflow/incident screen content (separate beads)
- [ ] IPC server implementation (separate crate)
- [ ] Persistence of UI state across sessions

---

## Acceptance Criteria

1. Clicking each nav tab switches to that screen
2. Active tab shows visual highlight (3px bottom accent bar with screen color)
3. Replay screen shows transport bar with play/pause button
4. Replay screen shows timeline with event chips
5. No `unwrap()`, `expect()`, or `panic()` in `handle_event` path
6. All tests pass (unit tests for state transitions, integration tests for IPC wiring)

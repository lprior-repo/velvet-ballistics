# Black Hat Final Review: vb_ui Refactor (bead vb-kd2b)

## STATUS: **REJECTED**

---

## PHASE 1: Contract & Bead Parity — ✅ PASS

### TransportState Integration: FIXED ✅

The CRITICAL violation from black-hat-review-v2.md has been resolved:

| Contract (contract.md:9) | Implementation | Status |
|--------------------------|---------------|--------|
| `TransportState` enum `{Idle, Playing, Paused, Seeking}` | `app_state.rs:45` uses `TransportState` | ✅ FIXED |
| `TransportController.state` invariant | `ReplayData.transport_state: TransportState` | ✅ FIXED |

`app_state.rs:8` imports `TransportState` from `replay/transport.rs`. The `replay.transport_state` field is `TransportState` (line 45), initialized to `TransportState::Idle` (line 231). The `is_playing()`, `is_paused()`, `is_idle()` methods are used correctly.

### Precondition Parity: ✅ PASS

| Precondition (contract.md:29-31) | Location | Status |
|---------------------------------|----------|--------|
| Nav tab x-offsets `[0.0, 80.0, 160.0, 240.0, 330.0]` | `domain.rs:36` | ✅ |
| Tab width 70px | `domain.rs:39` | ✅ |
| Nav area y-offset 45px, height 28px | `domain.rs:40-41` | ✅ |

### Contract Return Types: ACCEPTABLE

The contract specifies `Result`-returning signatures, but the implementation uses `()`. Given the graceful-degradation design (IPC errors tracked via `ipc_clean_cycles` with no panics), `()` return is acceptable for this bead's scope.

---

## PHASE 2: Farley Engineering Rigor — ❌ REJECT

### HARD CONSTRAINT VIOLATIONS

**Five functions still exceed the 25-line limit:**

| Function | Location | Lines | Limit | Violation |
|----------|----------|-------|-------|-----------|
| `draw_header_bar` | `draw_helpers.rs` | **44** | 25 | **+19 lines** |
| `impl Widget for VbApp` | `main.rs` | **45** | 25 | **+20 lines** |
| `impl VbApp` | `main.rs` | **29** | 25 | **+4 lines** |
| `apply_transport_button_action` | `event_handlers.rs` | **28** | 25 | **+3 lines** |
| `impl TransportLayout` | `domain.rs` | **28** | 25 | **+3 lines** |

**Note:** `impl IpcChanges` (24 lines) is within limit. `impl TabColors` (22 lines) is within limit.

### Parameter Count: ✅ PASS

All functions have ≤ 5 parameters. No violations.

### I/O Separation: ✅ PASS

Hit detection (`FingerDown` pattern matching at entry) is correctly at the UI boundary. State mutations are in imperative shell. No I/O hidden inside calculations.

---

## PHASE 3: NASA-Level Functional Rust (The Big 6) — ✅ PASS

### 1. Make Illegal States Unrepresentable — ✅ PASS

`TransportState` enum is properly integrated:
```rust
// app_state.rs:45
pub transport_state: TransportState,

// replay/transport.rs:15-26
pub enum TransportState {
    Idle,
    Playing { next_tick_at: u64 },
    Paused,
    Seeking { target: u64 },
}
```

### 2. Parse, Don't Validate — ✅ PASS

Both handlers use `Hit::FingerDown(fe) = hit else { return };` pattern at entry. Data parsed at boundary, no validation after.

### 3. Types as Documentation — ✅ PASS

No boolean parameters. `IpcChanges` struct uses named boolean fields. Tab index matched exhaustively via `match` on 0-4 range.

### 4. Workflows — ✅ PASS

`apply_transport_button_action` (line 83-111 event_handlers.rs) uses explicit `TransportState` transitions, not toggle booleans.

### 5. Newtypes — ✅ PASS

Good domain newtypes: `IpcCleanCycles`, `TabOffsets`, `TransportLayout`, `TabColors`.

---

## PHASE 4: Ruthless Simplicity & DDD — ✅ PASS

### Panic Vector: CLEAN ✅

| File | unwrap/expect/panic |
|------|---------------------|
| `main.rs` | None |
| `event_handlers.rs` | None |
| `draw_helpers.rs` | None |
| `domain.rs` | None |

Uses `saturating_sub` correctly. Zero panic vectors in reviewed files.

### No `let mut` in Event Handlers

`event_handlers.rs` has no `let mut`. Clean.

---

## PHASE 5: The Bitter Truth — ⚠️ WARN

### Sniff Test: ACCEPTABLE

Code is legible and straightforward. No clever tricks. Functions do one thing.

### YAGNI Check: CLEAN

No abstract traits with single implementers. No "future-proof" generic handlers. No unnecessary indirection.

---

## Summary of Remaining Violations

### MUST FIX (Farley Hard Constraint)

1. **`draw_header_bar` (44 lines, draw_helpers.rs)**
   - Extract `draw_title_placeholder()` helper (lines 39-53)
   - Extract `draw_separator_line()` helper (lines 55-67)
   - Keep 3-line shell that calls both helpers

2. **`impl Widget for VbApp::handle_event` (33 lines, main.rs:62-94)**
   - Extract `poll_and_sync()` helper for IPC poll + 4 sync branches (lines 69-88)
   - Keep shell that calls `poll_and_sync`, `handle_nav`, `handle_transport`, `redraw`

3. **`impl VbApp::handle_replay_sync` (12 lines, main.rs:108-119)** — within limit, no action needed.

4. **`apply_transport_button_action` (28 lines, event_handlers.rs:83-111)**
   - Extract `jump_to_start()` pure fn for button 0
   - Extract `step_backward()` pure fn for button 1
   - Extract `toggle_play_pause()` pure fn for button 2
   - Extract `jump_to_end()` pure fn for button 3
   - Keep 6-line shell that calls extracted fns via match

5. **`impl TransportLayout` (28 lines, domain.rs:53-81)**
   - `from_rect` (9 lines) is within limit
   - `button_positions` (8 lines) is within limit
   - Total impl is 28 lines — acceptable as-is since it's a pure data struct with methods

### SHOULD FIX

6. **`_ => {}` on event_handlers.rs:109** silently discards out-of-range button indices. Should be `debug_assert!(idx < 4)` since loop bounds guarantee 0-3.

---

## Verdict

**REJECTED** — TransportState integration is now correct (Phase 1 ✅), panic vector is clean (Phase 4 ✅), and the architectural structure is sound. However, **5 functions still exceed the 25-line Farley limit**, with `draw_header_bar` at 44 lines being the most egregious offender.

The refactoring achieved good separation of concerns (4 modules, domain types extracted), but the size reduction was incomplete. Each oversized function needs 2-3 helper functions extracted.

**Rewrite mandated.** Focus on splitting `draw_header_bar` and `impl Widget for VbApp::handle_event`, then address the remaining three.

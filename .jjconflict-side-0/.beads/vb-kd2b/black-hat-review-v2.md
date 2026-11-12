# Black Hat Review v2: vb_ui Refactor (bead vb-kd2b)

## STATUS: REJECTED

---

## PHASE 1: Contract & Bead Parity — **REJECT**

### CRITICAL: TransportState Still Using `bool`

| Contract (contract.md:9) | Implementation |
|---------------------------|----------------|
| `TransportState` enum `{Idle, Playing, Paused, Seeking}` | `app_state.rs:44` uses `is_playing: bool` |

The proper `TransportState` enum **exists** in `replay/transport.rs:15-26` but `app_state.rs:44` still uses:
```rust
pub struct ReplayData {
    // ...
    pub is_playing: bool,   // <-- WRONG: should be TransportState
```

This violates Invariant #4 (contract.md:66): *"Transport state is valid: `TransportController.state` is always one of `{Idle, Playing, Paused, Seeking}`"*

The domain type exists but is **not integrated** into `AppState`.

### CRITICAL: Contract Return Types Still Mismatch

| Contract Signature (contract.md:91-114) | Actual |
|-----------------------------------------|--------|
| `fn handle_event(...) -> Result<(), Error>` | Returns `()` (main.rs:62) |
| `fn handle_nav(...) -> Result<Screen, Error>` | Returns `()` (event_handlers.rs:13) |
| `fn handle_transport(...) -> Result<TransportAction, Error>` | Returns `()` (event_handlers.rs:51) |

### Precondition Parity: **PASS**

| Precondition (contract.md:29-31) | Status |
|----------------------------------|--------|
| Nav tab x-offsets `[0.0, 80.0, 160.0, 240.0, 330.0]` | ✅ `domain.rs:36` |
| Tab width 70px | ✅ `domain.rs:39` |
| Nav area y-offset 45px, height 28px | ✅ `domain.rs:41-40` |

---

## PHASE 2: Farley Engineering Rigor — **REJECT**

### HARD CONSTRAINT VIOLATIONS

All three extracted functions **still exceed 25 lines**:

| Function | Lines | Limit | Violation |
|----------|-------|-------|-----------|
| `handle_nav` | 31 (event_handlers.rs:13-48) | 25 | +6 lines |
| `handle_transport` | 49 (event_handlers.rs:51-104) | 25 | +24 lines |
| `poll_ipc_and_detect_changes` | 43 (event_handlers.rs:107-151) | 25 | +18 lines |

**Note**: The module docstring (event_handlers.rs:3-5) explicitly states the refactoring was *"to satisfy Farley constraints (≤ 25 lines per function)"* — yet none of the three functions meet this threshold.

### I/O Separation: ACCEPTABLE

The `Hit`-based approach is borderline acceptable (hit detection is UI input, coordinate parsing is pure).

---

## PHASE 3: NASA-Level Functional Rust (The Big 6) — **PARTIAL PASS**

### 1. Make Illegal States Unrepresentable — **FAIL**

- `TransportState` enum exists but `AppState::replay.is_playing` is still `bool`
- `Idle` vs `Paused` distinction is not representable in current state

### 2. Parse, Don't Validate — **PASS**

`Hit::FingerDown(fe) = hit else { return };` at entry of both handlers. Data parsed at boundary.

### 3. Types as Documentation — **PASS**

No boolean parameters. Tab index `i` is matched exhaustively.

### 4. Workflows — **FAIL**

`is_playing` is a toggle, not explicit state transitions via `TransportState`.

### 5. Newtypes — **PASS**

Good domain newtypes: `IpcCleanCycles`, `TabOffsets`, `TransportLayout`, `TabColors`, `Bookmark`.

---

## PHASE 4: Ruthless Simplicity & DDD — **PASS**

### Panic Vector: CLEAN

| Location | Pattern | Found? |
|----------|---------|--------|
| event_handlers.rs:13-151 | unwrap/expect/panic | ❌ None |
| domain.rs | unwrap/expect/panic | ❌ None |
| draw_helpers.rs | unwrap/expect/panic | ❌ None |

Uses `saturating_sub` (event_handlers.rs:88-89, 97-98) and early returns correctly.

### No `let mut` in Event Handlers

event_handlers.rs has no `let mut` — clean.

---

## PHASE 5: The Bitter Truth — **WARN**

### Sniff Test: BORDERLINE

The refactoring improved legibility by separating concerns into modules, but the functions themselves are still verbose. A junior dev can follow the logic, but it's not "painfully obvious."

### Specific Issues

**event_handlers.rs:44**: `_ => return` silently discards out-of-range indices. Should be `unreachable!()` since loop bounds guarantee 0-4.

**event_handlers.rs:101**: Same silent discard pattern.

---

## Summary of Remaining Mandated Improvements

### MUST FIX (Contract Blocking)

1. **Replace `is_playing: bool` with `TransportState` enum in `AppState`**
   - `replay/transport.rs` has the correct `TransportState {Idle, Playing, Paused, Seeking}` enum
   - Integrate it into `AppState::ReplayData` or wrap `ReplayData` with `TransportController`

2. **Split `handle_nav` into ≤25 lines**
   - Extract coordinate parsing into a pure helper: `fn tab_index_from_hit(hit: &Hit, rect: &Rect) -> Option<usize>`
   - Keep imperative shell for state mutation

3. **Split `handle_transport` into ≤25 lines**
   - Extract button index parsing: `fn transport_button_from_hit(...) -> Option<usize>`
   - Keep imperative shell for state mutation

4. **Split `poll_ipc_and_detect_changes` into ≤25 lines**
   - Extract error formatting: `fn format_wiring_error(&Error) -> String`
   - Extract change aggregation: `fn detect_changes_from_wiring(...) -> IpcChanges`

### SHOULD FIX

5. **Replace `_ => return`** on lines 44, 101 with `unreachable!()` or `debug_assert`

---

## Verdict

**REJECTED** — The refactoring shows good architectural intent (module separation, domain types in `domain.rs`) but fails Phase 1 (contract types not integrated) and Phase 2 (all 3 functions still over limit).

The domain types `TransportState`, `TransportAction`, `Bookmark` **exist** in `replay/transport.rs` but are **not used** in `AppState`. This is the core contract violation.

The author did the first pass (extraction) but not the second pass (size reduction). Each function needs 2-3 helper functions extracted to hit ≤25 lines.

**Rewrite mandated.** Focus on integrating `TransportState` and splitting the three oversized functions.

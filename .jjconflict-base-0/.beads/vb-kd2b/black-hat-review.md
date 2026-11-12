# Black Hat Review: vb_ui (bead vb-kd2b)

## STATUS: REJECTED

---

## PHASE 1: Contract & Bead Parity — **REJECT**

### CRITICAL: Contract Signature Mismatch

| Contract Signature (contract.md:91-114) | Actual Implementation (main.rs:52,330,367) |
|----------------------------------------|-------------------------------------------|
| `fn handle_event(&mut self, cx: &mut Cx, event: &Event) -> Result<(), Error>` | `fn handle_event(&mut self, cx: &mut Cx, event: &Event, _scope: &mut Scope)` returns `()` |
| `fn handle_nav(&mut self, cx: &mut Cx, event: &Event) -> Result<Screen, Error>` | `fn handle_nav(&mut self, _cx: &mut Cx, hit: &Hit)` returns `()` |
| `fn handle_transport(&mut self, cx: &mut Cx, event: &Event) -> Result<TransportAction, Error>` | `fn handle_transport(&mut self, _cx: &mut Cx, hit: &Hit)` returns `()` |

**Line 52**: `handle_event` returns `()` instead of `Result<(), Error>`. All fallibility is via early-return (no `Result` wrapper). This violates the explicit contract on line 94.

**Line 330**: `handle_nav` returns `()` instead of `Result<Screen, Error>`. Contract line 97 is explicit.

**Line 367**: `handle_transport` returns `()` instead of `Result<TransportAction, Error>`. Contract line 101 is explicit.

### CRITICAL: Missing Domain Types

1. **`TransportState` enum NOT implemented** (contract.md:9)
   - Contract specifies: `TransportState`: Enum `{Idle, Playing, Paused, Seeking}`
   - Actual: `app_state.rs:44` uses `is_playing: bool` — a raw boolean, not a sum type
   - This makes illegal states representable (what happens when `is_playing = true` AND user clicks pause? toggle works. But what about `Idle` vs `Paused` distinction? Not representable.)

2. **`TransportAction` type NOT defined**
   - Referenced in contract.md:101 as return type of `handle_transport`
   - No such type exists in the codebase

3. **`TransportController.bookmarks` field MISSING** (contract.md:11)
   - Contract specifies: `TransportController` has `current_position`, `total_events`, `bookmarks`
   - Actual `ReplayData` (app_state.rs:41-51) has `playback_position`, `total_events`, `timeline_strip`
   - `TimelineStrip` exists but `bookmarks` field is not present

### Precondition Parity: PASS

| Precondition (contract.md:29-31) | Implementation | Status |
|----------------------------------|----------------|--------|
| Nav tab x-offsets `[0.0, 80.0, 160.0, 240.0, 330.0]` | Line 336: `[0.0, 80.0, 160.0, 240.0, 330.0]` | ✅ MATCH |
| Tab width 70px | Line 335: `70.0` | ✅ MATCH |
| Nav area y-offset 45px, height 28px | Lines 332-334: `header_height=45.0`, `tab_height=28.0` | ✅ MATCH |

---

## PHASE 2: Farley Engineering Rigor — **REJECT**

### HARD CONSTRAINT VIOLATIONS

**Function length > 25 lines:**

| Function | Lines | Limit | Violation |
|----------|-------|-------|-----------|
| `handle_nav` | 35 (lines 330-365) | 25 | +10 lines |
| `handle_transport` | 60 (lines 367-427) | 25 | +35 lines |

**Lines 330-365 (`handle_nav`):** 35 lines. Must be split into smaller pure functions.

**Lines 367-427 (`handle_transport`):** 60 lines. Must be refactored into a functional core + imperative shell.

### I/O Separation Violation

`handle_nav` (line 330) and `handle_transport` (line 367) are passed `&Hit` which is derived from event hit-testing in `handle_event` (line 54). The hit detection is I/O-adjacent (user input), but the handlers themselves contain pure hit-coordinate logic. This is borderline acceptable, but the 60-line `handle_transport` mixes coordinate calculation (pure) with state mutation (impure IO) in the same function without separation.

---

## PHASE 3: NASA-Level Functional Rust (The Big 6) — **REJECT**

### 1. Make Illegal States Unrepresentable — FAIL

- **`TransportState` is a bool** (app_state.rs:44): `is_playing: bool` instead of `enum TransportState { Idle, Playing, Paused, Seeking }`. The difference between `Idle` (not started) and `Paused` (started then stopped) is indistinguishable.

### 2. Parse, Don't Validate — PASS (for nav/transport)

`handle_nav` and `handle_transport` parse the `Hit` at entry (`Hit::FingerDown(fe) = hit else { return };`). Data is parsed into a trusted type at the boundary. No validation-after-parsing issues in these handlers.

### 3. Types as Documentation — FAIL

- **Line 339**: `let Hit::FingerDown(fe) = hit else { return };` — this is a destructuring let, not a boolean parameter, so it's acceptable.
- Boolean parameter issue: N/A for these functions.

### 4. Workflows — FAIL

- `TransportState` not modeled as explicit state-to-state transitions. `is_playing` is a toggle, not a proper state machine with defined transitions.

### 5. Newtypes — PASS

- `Screen` enum is properly defined (app_state.rs:11-18). 5 variants, exhaustive.
- No unwrapped primitives used as domain models in `handle_nav`/`handle_transport`.

---

## PHASE 4: Ruthless Simplicity & DDD (Scott Wlaschin) — **PASS**

### Panic Vector: CLEAN

| Location | Pattern | Found? |
|----------|---------|--------|
| handle_nav (330-365) | unwrap/expect/panic | ❌ None |
| handle_transport (367-427) | unwrap/expect/panic | ❌ None |

Both functions use `saturating_sub` (lines 409, 419) instead of unchecked subtraction. Early returns for invalid hits. No `panic!`, `unwrap()`, `expect()`, or `todo!`.

### No `let mut` in Reviewed Section

Lines 300-450 show no `let mut` in the nav/transport handlers.

### CUPID Properties

- **Composable**: ✅ `handle_nav` and `handle_transport` are single-responsibility
- **Unix-philosophy**: ✅ Each does one thing — nav tab detection, transport button detection
- **Predictable**: ✅ Given same hit coordinates, same screen transition result
- **Idiomatic**: ✅ Uses pattern matching on `Hit::FingerDown`
- **Domain-based**: ⚠️ Uses Makepad `Hit` type rather than a domain event wrapper

---

## PHASE 5: The Bitter Truth (Velocity & Legibility) — **WARN**

### Sniff Test: BORDERLINE

`handle_nav` (35 lines) and `handle_transport` (60 lines) are readable but overly long. A junior developer could understand them, but they're not "painfully obvious."

### YAGNI: CLEAN

No generic handlers, abstract traits with one implementer, or "future use" code in the reviewed section.

### Specific Issues

**Line 359**: `_ => return` silently discards out-of-range tab indices. Should be `unreachable!()` or logged, as this indicates a programming error (loop range guarantees 0-4).

**Lines 392-397**: Button position array is hand-computed. Could use `Iterator::scan` or a formula `i as f64 * (btn_width + btn_spacing)` for DRYness.

---

## Summary of Mandated Improvements

### MUST FIX (Contract Blocking)

1. **Change return types to `Result<_, Error>`** on `handle_event`, `handle_nav`, `handle_transport` to match contract.md:94,97,101

2. **Define `TransportState` enum** with variants `{Idle, Playing, Paused, Seeking}` and replace `is_playing: bool` in `ReplayData`

3. **Define `TransportAction` enum** and return it from `handle_transport`

4. **Add `bookmarks` field** to `ReplayData` (or `TransportController` wrapper) per contract.md:11

5. **Split `handle_nav`** into pure coordinate-parsing function + imperative shell (target: ≤25 lines)

6. **Split `handle_transport`** into pure coordinate-parsing function + imperative shell (target: ≤25 lines)

### SHOULD FIX (Engineering Quality)

7. **Replace `_ => return`** on line 359 with `unreachable!()` or debug_assert since the loop index is bounded 0-4

8. **Use formula for button positions** (line 392-397) instead of hand-computed array

### Verification

- No unwrap/expect/panic in handle_nav or handle_transport — ✅ CLEAN
- Nav tab offsets match contract preconditions — ✅ CLEAN
- Transport uses saturating arithmetic — ✅ CLEAN

---

## Verdict

**REJECTED** — The implementation is functionally safe (no panics, correct hit detection) but violates the contract in Phase 1 (wrong return types, missing domain types) and Phase 2 (function length violations).

The code passes "does not crash" but fails "is correct according to specification." The contract is the law. Rewrite mandated.

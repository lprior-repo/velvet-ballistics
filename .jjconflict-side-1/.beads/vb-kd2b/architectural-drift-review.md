# Architectural Drift Review: vb_ui/main.rs

**Bead**: vb-kd2b  
**File**: `crates/vb_ui/src/main.rs`  
**Line Count**: 460 lines (LIMIT: 300) ❌

---

## STATUS: REJECTED

---

## Findings

### 1. File Size Violation ❌

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Lines | 460 | 300 | **+160 over** |

**Recommendation**: Split into focused modules:
- `draw_helpers.rs` — Extract all `draw_*` methods (~150 lines)
- `event_handlers.rs` — Extract `handle_nav`, `handle_transport`, and IPC polling (~120 lines)
- `main.rs` — Keep only Widget trait impl, `AppMain` impl, and script_mod! (~190 lines)

---

### 2. Scott Wlaschin DDD Violations

#### 2.1 Primitive Obsession ❌

**Issue**: Raw primitive types used where domain types exist.

```rust
// Line 45: Raw u8 counter
#[rust]
ipc_clean_cycles: u8,  // Should be a domain type

// Lines 206, 336: Magic numbers scattered in drawing code
let tab_x_offsets = [0.0, 80.0, 160.0, 240.0, 330.0];
```

**Recommendation**: Create domain types:
```rust
struct IpcCleanCycles(u8);  // With TryFrom or From trait
struct TabOffsets([f64; 5]);  // Named constant array
```

#### 2.2 Duplicated Hit Detection Pattern ❌

**Issue**: `handle_nav` and `handle_transport` both contain identical `Hit::FingerDown` extraction and primary hit check:

```rust
// handle_nav lines 339-342
let Hit::FingerDown(fe) = hit else { return };
if !fe.is_primary_hit() { return; };

// handle_transport lines 379-382 (IDENTICAL)
let Hit::FingerDown(fe) = hit else { return };
if !fe.is_primary_hit() { return; };
```

**Recommendation**: Extract to helper:
```rust
fn extract_finger_down(hit: &Hit) -> Option<&FingerDown> {
    let Hit::FingerDown(fe) = hit else { return None };
    fe.is_primary_hit().then_some(fe)
}
```

#### 2.3 Empty Stub Methods ❌

**Issue**: Four sync methods are empty stubs (lines 441, 443-447):
```rust
fn ingest_timeline_events(&mut self, _responses: &[vb_ipc::server::IpcResponse]) {}
fn sync_verify_state(&mut self, _cx: &mut Cx) {}
fn sync_system_state(&mut self, _cx: &mut Cx) {}
fn sync_workflow_state(&mut self, _cx: &mut Cx) {}
```

**Recommendation**: Either implement or remove. If planned for future, document with TODO marker in bead.

---

### 3. Structural Cohesion Issues

#### 3.1 Multiple Responsibility Violation ❌

The `VbApp` struct conflates:
- **IPC wiring** (lines 43, 59-115)
- **Drawing** (lines 135-328)
- **Navigation handling** (lines 330-365)
- **Transport handling** (lines 367-427)
- **State sync** (lines 435-447)

#### 3.2 Widget Trait Implementation Too Large

`handle_event` (lines 52-120, ~70 lines) handles:
- IPC polling
- Error handling with 3-cycle clean mechanism
- Conditional state syncs based on wiring events
- Navigation and transport routing

**Recommendation**: Use early returns to separate concerns:
```rust
fn handle_event(&mut self, cx: &mut Cx, event: &Event, _scope: &mut Scope) {
    if let Some(fe) = self.try_poll_ipc(cx) { ... }
    if let Some(hit) = self.try_capture_hit(cx, event) { ... }
}
```

---

### 4. Partial Positives ✓

- Good use of Makepad 2.0 widget pattern
- Screen enum properly modeled in AppState
- IPC error handling with clean cycles is thoughtful
- Modular crate structure (12 submodules in lib.rs)

---

## Required Refactors

1. **Extract `draw_helpers.rs`** (~150 lines)
   - `VbApp::draw_background`
   - `VbApp::draw_header_bar`
   - `VbApp::draw_nav_tabs`
   - `VbApp::draw_content`

2. **Extract `event_handlers.rs`** (~130 lines)
   - `VbApp::handle_nav`
   - `VbApp::handle_transport`
   - Helper: `extract_finger_down`

3. **Extract `ipc_integration.rs`** (~50 lines)
   - IPC polling logic
   - Error handling and clean cycles
   - State sync orchestration

4. **Create domain types**:
   - `IpcCleanCycles(u8)`
   - `TabOffsets([f64; 5])`
   - `TransportButtons([f64; 4])`

5. **Implement or remove stub methods** before next review cycle

---

## Summary

| Category | Status |
|----------|--------|
| File Size | ❌ REJECTED (460 > 300) |
| DDD Principles | ❌ 3 violations |
| Structural Cohesion | ❌ Low |

**Next Action**: Refactor main.rs into focused modules per Section 35's Data-Calc-Actions layering. Target: <300 lines.

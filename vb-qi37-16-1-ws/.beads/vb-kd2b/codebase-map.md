# vb-kd2b: vb_ui Codebase Map

## Bead Overview

**Task**: Implement interactive navigation tabs and screen switching for the vb_ui Makepad 2.0 application.

**Files in scope**: `crates/vb_ui/src/main.rs` (358 lines currently, stub implementations), `crates/vb_ui/src/app_state.rs`, `crates/vb_ui/src/ipc_wiring.rs`, `crates/vb_ui/src/ipc_bridge.rs`, and all screen-specific modules under `crates/vb_ui/src/{replay,verify,system,incident,workflow}/`.

---

## 1. Current State of `vb_ui` main.rs (358 lines, Widget trait pattern)

### File Location
`/home/lewis/src/Velvet-ballistics/crates/vb_ui/src/main.rs`

### Struct Definition (lines 23-48)

```rust
#[derive(Script, ScriptHook, Widget)]
pub struct VbApp {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[walk]
    walk: Walk,
    #[layout]
    layout: Layout,
    #[redraw]
    #[live]
    draw_bg: DrawColor,          // Background draw
    #[live]
    draw_header: DrawColor,     // Header bar draw
    #[live]
    draw_nav: DrawColor,        // Nav tabs draw
    #[rust]
    app_state: AppState,        // Rust state (screen, replay, system, etc.)
    #[rust]
    ipc_wiring: IpcAppWiring,   // IPC bridge wiring
    #[rust]
    ipc_clean_cycles: u8,       // Error recovery counter
    #[rust]
    rect: Rect,                 // Cached widget rect
}
```

### `Widget` Trait Implementation (lines 50-132)

#### `handle_event` (lines 52-121)
- **Focus handling**: `Hit::FingerDown` on `draw_bg` area sets key focus
- **IPC polling**: `self.ipc_wiring.poll(&mut self.app_state)` drains IPC events
- **Error recovery**: 3-cycle debounce for IPC error clearing (`ipc_clean_cycles`)
- **Screen-specific sync**: Routes `WiringEvents` flags to `sync_*` methods:
  - `metrics_updated | connection_changed | health_checked | run_list_updated | has_errors` → `sync_system_state`
  - `verification_updated | taint_report_updated` → `sync_verify_state`
  - `run_accepted | run_cancelled | events_arrived | trace_drained | inspected` → `sync_replay_state`
  - `workflow_graph_updated` → `sync_workflow_state`
- **Nav/transport handlers**: `handle_nav`, `handle_transport` called every frame
- **Redraw**: `self.redraw(cx)` at end of every event

#### `draw_walk` (lines 124-131)
```rust
fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
    self.rect = cx.walk_turtle(walk);
    self.draw_background(cx);   // Full rect background (#0a0a12)
    self.draw_header_bar(cx);  // 44px header with title + separator
    self.draw_nav_tabs(cx);    // 5 tabs at y+45, 28px height
    self.draw_content(cx);      // Content area from y+73
    DrawStep::done()
}
```

### Draw Methods

#### `draw_background` (lines 136-144)
- Fills entire `self.rect` with `#0a0a12` (Vec4f {x:0.039, y:0.039, z:0.071, w:1.0})

#### `draw_header_bar` (lines 147-198)
- **Header rect**: 44px height at top of widget rect
- **Background**: `#12121f` (Vec4f {x:0.071, y:0.078, z:0.122, w:1.0})
- **Title area**: Small rect at (x+16, y+8) with cyan fill (#00f5ff)
- **Separator**: 1px line at header bottom with `#2a2a4a`

#### `draw_nav_tabs` (lines 202-265)
- **Position**: y = rect.y + 45, height 28px
- **5 tab x-offsets**: `[0.0, 80.0, 160.0, 240.0, 330.0]`
- **Tab width**: 70px each
- **Active tab**: `#1a1a2a` background with colored bottom accent (3px)
- **Inactive tab**: `#1a1a2e` background
- **Per-tab accent colors**:
  - Tab 0 (RunReplay): Neon cyan `(0, 0.96, 1.0)`
  - Tab 1 (Verification): Neon green `(0.22, 1.0, 0.08)`
  - Tab 2 (SystemOverview): Neon blue `(0.18, 0.42, 1.0)`
  - Tab 3 (WorkflowGraph): Neon purple `(0.69, 0.30, 1.0)`
  - Tab 4 (IncidentConsole): Neon red `(1.0, 0.03, 0.23)`
- **Current implementation**: Only draws colored accent rectangles, NO text labels

#### `draw_content` (lines 269-328)
- **Content area**: y = rect.y + 73, height = rect.height - 73
- **Background**: `#0a0a12`
- **Panel rect**: (x+20, y+20) with size (width-40, 150px) at `#16162a`
- **Accent bar**: 4px left border on panel with screen-specific color
- **Current implementation**: Only draws a single placeholder panel, NO per-screen content

### Stub Methods (lines 331-345)
```rust
fn handle_nav(&mut self, _cx: &mut Cx) {}           // STUB: no-op
fn handle_transport(&mut self, _cx: &mut Cx) {}     // STUB: no-op
fn sync_nav(&mut self, _cx: &mut Cx, _title: String) {}  // STUB: no-op
fn sync_replay_state(&mut self, _cx: &mut Cx) {}    // STUB: no-op
fn ingest_timeline_events(&mut self, _responses: &[vb_ipc::server::IpcResponse]) {} // STUB
fn sync_verify_state(&mut self, _cx: &mut Cx) {}     // STUB: no-op
fn sync_system_state(&mut self, _cx: &mut Cx) {}     // STUB: no-op
fn sync_workflow_state(&mut self, _cx: &mut Cx) {}   // STUB: no-op
```

### `AppMain` Implementation (lines 348-358)
- Calls `makepad_widgets::script_mod(vm)` + local `script_mod!` macro
- Wraps `Widget::handle_event` with empty `Scope`

---

## 2. Architecture Pattern to Implement

### Pattern Overview: Makepad 2.0 Widget Trait with Rust State

The `VbApp` struct uses Makepad's `Widget` trait pattern combined with Rust state management:

1. **`#[rust]` fields**: Stored in Rust struct, not Makepad's object system
2. **`#[live]` fields**: Makepad-managed draw state (DrawColor, DrawText, etc.)
3. **`handle_event` loop**: Single event handler routes all events
4. **`draw_walk`**: Called during render pass, draws in back-to-front order

### Adding Clickable Navigation Tabs

**Current state**: `draw_nav_tabs` only draws colored rectangles, no hit testing

**To add click handling** in `handle_event`:
```rust
// Pattern for tab hit testing
match event.hits_with_capture_overload(cx, self.draw_nav.area(), true) {
    Hit::FingerDown(fe) if fe.is_primary_hit() => {
        // Determine which tab was clicked based on x position
        let tab_x = fe.loc.x - self.rect.pos.x;
        let new_screen = match tab_x {
            0.0..=80.0 => Screen::RunReplay,
            80.0..=160.0 => Screen::Verification,
            160.0..=240.0 => Screen::SystemOverview,
            240.0..=330.0 => Screen::WorkflowGraph,
            _ => Screen::IncidentConsole,
        };
        self.app_state.switch_screen(new_screen);
        self.redraw(cx);
    }
    _ => {}
}
```

**Key considerations**:
- Tab area is `draw_nav` DrawColor with `area()` for hit testing
- Need to calculate tab boundaries from `self.rect.pos.x` + x_offset
- Call `self.app_state.switch_screen(screen)` to update state
- Trigger redraw after state change

### Implementing Screen Switching via Rust State

**State machine**: `AppState.current_screen: Screen` enum

**Screen enum** (from `app_state.rs`):
```rust
pub enum Screen {
    RunReplay,
    Verification,
    SystemOverview,
    WorkflowGraph,
    IncidentConsole,
}
```

**Switching pattern**:
```rust
// In handle_event, after IPC wiring events
if self.app_state.current_screen() != previous_screen {
    self.sync_nav(cx, self.app_state.screen_title().to_string());
    self.redraw(cx);
}
```

**Screen title lookup**:
```rust
pub fn screen_title(&self) -> &'static str {
    match self.current_screen {
        Screen::RunReplay => "Replay Theater",
        Screen::Verification => "Verification",
        Screen::SystemOverview => "System Overview",
        Screen::WorkflowGraph => "Workflow Graph",
        Screen::IncidentConsole => "Incident Console",
    }
}
```

### Drawing Content Panels with DrawColor, DrawText

**Required draw primitives** (from `makepad_widgets::*`):
- `DrawColor`: Solid color rectangles
- `DrawText`: Text rendering with font/color

**Content drawing pattern** (in `draw_content`):
```rust
fn draw_content(&mut self, cx: &mut Cx2d) {
    let content_y = self.rect.pos.y + 73.0;
    let content_rect = Rect {
        pos: DVec2 { x: self.rect.pos.x, y: content_y },
        size: DVec2 { x: self.rect.size.x, y: self.rect.size.y - 73.0 },
    };

    // Background
    self.draw_bg.color = bg::CANVAS;
    self.draw_bg.draw_abs(cx, content_rect);

    // Per-screen content
    match self.app_state.current_screen() {
        Screen::RunReplay => self.draw_replay_content(cx, content_rect),
        Screen::Verification => self.draw_verification_content(cx, content_rect),
        Screen::SystemOverview => self.draw_system_content(cx, content_rect),
        Screen::WorkflowGraph => self.draw_workflow_content(cx, content_rect),
        Screen::IncidentConsole => self.draw_incident_content(cx, content_rect),
    }
}
```

### Adding Interactive Transport Controls

**Transport state** (from `replay/transport.rs`):
```rust
pub struct TransportController {
    state: TransportState,       // Idle | Playing | Paused | Seeking
    speed: PlaybackSpeed,
    current_position: u64,
    total_events: u64,
    bookmarks: Vec<Bookmark>,
}
```

**Control actions** (from `TransportAction`):
```rust
pub enum TransportAction {
    SeekTo { position: u64 },
    Redraw,
    NoOp,
}
```

**Transport button hit handling** in `handle_transport`:
```rust
fn handle_transport(&mut self, cx: &mut Cx) {
    if self.app_state.current_screen() != Screen::RunReplay {
        return;
    }
    // Transport button areas would need to be defined
    // Match Hit::FingerDown against button rects
    // Call transport_controller.play(), .pause(), .step_forward(), etc.
}
```

---

## 3. UI/UX Specification

### Layout Structure

```
+-------------------------------------------------------------------+
| HEADER BAR (44px)                                          |
| [vb logo] [screen title]                                            |
+-------------------------------------------------------------------+
| NAV TABS (28px, y=45)                                              |
| [RunReplay] [Verification] [SystemOverview] [WorkflowGraph] [Inc] |
+-------------------------------------------------------------------+
| CONTENT AREA (y=73, remaining height)                              |
|                                                                       |
|  +--------------------------------------------------------------+  |
|  |                     Screen-specific content                    |  |
|  |                                                              |  |
|  +--------------------------------------------------------------+  |
|                                                                       |
+-------------------------------------------------------------------+
| TRANSPORT BAR (for RunReplay screen only)                            |
| [|<] [<] [>] [>|]  [1x]  [jump: failure] [action] [done]         |
+-------------------------------------------------------------------+
```

### 5 Screens

| Screen | Accent Color | Content |
|--------|--------------|---------|
| **RunReplay** | Neon cyan `(0, 0.96, 1.0)` | Replay theater with graph, inspector, timeline |
| **Verification** | Neon green `(0.22, 1.0, 0.08)` | Certificate cards (Structure, Bounded, Resources, Taint, Action, Durability) |
| **SystemOverview** | Neon blue `(0.18, 0.42, 1.0)` | System metrics, topology, queue monitor, alerts |
| **WorkflowGraph** | Neon purple `(0.69, 0.30, 1.0)` | Workflow visualization with node overlay |
| **IncidentConsole** | Neon red `(1.0, 0.03, 0.23)` | Incident list, repair actions, console |

### Navigation Tab Specification

- **Position**: y = rect.pos.y + 45, height 28px
- **Width**: 70px per tab
- **X offsets**: `[0, 80, 160, 240, 330]`
- **Active indicator**: 3px bottom border with screen accent color
- **Clickable area**: Entire tab rectangle including accent bar
- **Focus behavior**: Set key focus to nav area on `FingerDown`

### Content Panel Specification

- **Panel background**: `#16162a` (CARD_BG)
- **Left accent bar**: 4px wide, screen accent color
- **Padding**: 20px from content area edges
- **Panel height**: Variable, min 150px

### Transport Bar Specification

- **Position**: Bottom of screen, only visible on RunReplay
- **Controls**: |<<, <, >, >>| buttons
- **Speed indicator**: "1x", "2x", "0.5x", etc.
- **Jump chips**: "jump: failure" (red), "action" (orange), "done" (green)
- **Timeline strip**: Event markers with cursor

---

## 4. File Dependencies

### Core Architecture Files

| File | Purpose | Key Exports |
|------|---------|-------------|
| `src/main.rs` | VbApp Widget impl | `VbApp`, `app_main!` macro |
| `src/lib.rs` | Module declarations | All submodules |
| `src/app_state.rs` | Global state + Screen enum | `AppState`, `Screen`, `ReplayData`, `SystemData`, `VerificationData`, `IncidentData`, `WorkflowData` |
| `src/ipc_wiring.rs` | IPC → AppState routing | `IpcAppWiring`, `WiringEvents` |
| `src/ipc_bridge.rs` | Low-level IPC client thread | `IpcBridge`, `IpcRequest`, `IpcReply` |

### Screen-Specific Modules

| Module | Files | Purpose |
|--------|-------|---------|
| **replay** | `replay/mod.rs`, `screen.rs`, `transport.rs`, `timeline.rs`, `controller.rs`, `engine.rs`, etc. | Run Replay Theater UI |
| **verify** | `verify/mod.rs`, `screen.rs`, `taint.rs`, `certificates.rs`, `action_policy.rs`, etc. | Verification certificates UI |
| **system** | `system/mod.rs`, `screen.rs`, `topology.rs`, `metrics.rs`, `queues.rs`, `alerts.rs`, etc. | System overview UI |
| **workflow** | `workflow/mod.rs`, `canvas.rs`, `node_mapping.rs` | Workflow graph UI |
| **incident** | `incident/mod.rs`, `screen.rs`, `timeline.rs`, `repair.rs`, `console.rs` | Incident console UI |

### Theme System

| File | Purpose |
|------|---------|
| `theme/mod.rs` | Theme module exports |
| `theme/colors.rs` | Color constants (neon, bg, text, state, node_*) |
| `theme/typography.rs` | Font/text styling |
| `theme/animation.rs` | Animation constants |
| `theme/glow.rs` | Glow effects |

### External Dependencies

| Crate | Purpose |
|-------|---------|
| `makepad-widgets` | Makepad 2.0 framework |
| `vb_ipc` | IPC protocol types |
| `vb_core` | Core types (RunId, WorkflowDigest) |
| `vb_storage` | Storage types |

### IPC Bridge Data Flow

```
UI Thread                 Background Thread              Server
   |                            |                           |
   | IpcRequest::Connect        |                           |
   | ─────────────────────────> | IpcClient::connect        |
   |                            | ─────────────────────────> |
   |                            |                           |
   | IpcRequest::InspectRun     |                           |
   | ─────────────────────────> | send_and_recv             |
   |                            | ─────────────────────────> |
   |                            |                           |
   | IpcReply::Inspected       |                           |
   | <───────────────────────── |                           |
   |                            |                           |
   | poll() ─────────────────> |                            |
   | returns Vec<IpcReply>      |                           |
```

**Wiring event routing** (`ipc_wiring.rs`):
1. `poll()` drains all pending `IpcReply` from bridge
2. `route_reply()` classifies replies and sets `WiringEvents` flags
3. `route_inspected()` handles `IpcResponse` variants, updates `AppState`
4. Flags returned to `handle_event` to trigger screen-specific syncs

### Screen Sync Methods (to be implemented)

| Method | Called when | Updates |
|--------|-------------|---------|
| `sync_nav` | `inspected` flag | Tab selection, title text |
| `sync_replay_state` | `run_accepted`, `run_cancelled`, `events_arrived`, `trace_drained` | ReplayData, timeline |
| `sync_verify_state` | `verification_updated`, `taint_report_updated` | VerificationData |
| `sync_system_state` | `metrics_updated`, `connection_changed`, `health_checked` | SystemData |
| `sync_workflow_state` | `workflow_graph_updated` | WorkflowData |

---

## Implementation Checklist for vb-kd2b

- [ ] Add `DrawText` field(s) to `VbApp` for tab labels
- [ ] Implement tab hit testing in `handle_event` using `FingerDown`
- [ ] Implement `handle_nav` to call `app_state.switch_screen()`
- [ ] Add per-screen content drawing methods (6 screen methods)
- [ ] Add transport button hit areas and `handle_transport` implementation
- [ ] Wire up `sync_nav`, `sync_replay_state`, `sync_verify_state`, `sync_system_state`, `sync_workflow_state`
- [ ] Draw screen-specific content in `draw_content` based on `current_screen()`
- [ ] Draw transport bar only when on RunReplay screen

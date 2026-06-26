---
section: 78
title: "Makepad UI Implementation Contract"
parent: velvet-ballistics-MASTER.md
---

## 78. Makepad UI Implementation Contract


> **Removed.** Makepad UI implementation is not part of the current core feature set. Remaining details in Sections 78-83 are historical residue only; no current backend bead may be blocked by Makepad, UI model artifacts, screenshot gates, or UI perf gates.

### Makepad Scope

Makepad is used only for the native UI crate `vb_ui_makepad`. It is forbidden in:

```text
vb_core
vb_runtime
vb_storage
vb_ipc
```

Makepad dependencies must not change runtime semantics, binary IPC semantics, or persistence semantics.

### Makepad Rationale

Makepad is selected for the UI because the design requires a native, GPU-driven desktop application with highly interactive graph, timeline, animation, and custom-rendered visual states. The UI uses Makepad 2.0 Splash (`script_mod!`) for layout/style iteration and Rust widgets for deterministic state handling.

### Crate Roles

| Crate | Role | Runtime-core dependency? |
|------|------|--------------------------|
| `vb_ui_model` | Typed UI artifacts shared by CLI/UI. No Makepad. | Cold path only |
| `vb_ui_makepad` | Native Makepad desktop app. | UI only |
| `velvet_ballistics` | CLI command dispatch, including `ui`. | Cold path command |

### `vb_ui_model` Required Types

```rust
pub enum UiScreenKind {
    ExecutionOverview,
    WorkflowGraphAuthoring,
    ExecutionDetailsGraph,
    VerificationCertificate,
    ReplayTheater,
    IncidentFailureConsole,
    ActionRegistry,
    StorageDoctorAiContext,
}

pub struct UiAppSnapshot {
    pub status: SystemStatusView,
    pub active_runs: Box<[RunSummaryView]>,
    pub selected_run: Option<RunInspectionView>,
    pub selected_workflow: Option<WorkflowGraphView>,
    pub verification: Option<VerificationReportView>,
    pub replay: Option<ReplayReportView>,
    pub incident: Option<IncidentReportView>,
    pub actions: Box<[ActionDescriptionView]>,
    pub storage: Option<StorageDoctorView>,
    pub ai_context: Option<AiContextView>,
}
```

All UI model structs must use bounded collections. Any list returned to the UI must carry a limit/cursor or a fixed bound. Unbounded UI lists are forbidden.

### Data Flow

```text
Compiler / verifier
  -> WorkflowGraph, VerificationReport, AcceptedArtifact
  -> vb_ui_model
  -> Makepad UI

Runtime / storage / replay
  -> RunInspection, RunEvents, ReplayReport, IncidentReport, SystemStatus
  -> vb_ui_model
  -> Makepad UI
```

The UI consumes typed artifacts. It does not parse YAML, does not execute workflows, does not resolve references, and does not dispatch actions by string.

### UI Connection Modes

| Mode | Command | Data source | Purpose |
|------|---------|-------------|---------|
| Embedded | `velvet-ballistics ui --db <path>` | Direct storage/runtime readers | Local desktop app with DB access |
| Attached | `velvet-ballistics ui --socket <path>` | Binary IPC | Operator app connected to running server |
| Demo | `velvet-ballistics ui --demo-fixture <fixture>` | Deterministic fixtures | Design review, screenshot tests, demos |

HTTP and JSON are not required for the UI. If a future streaming adapter is needed, it must be a separate cold-path adapter crate.

### Makepad Structure

Required module structure:

```text
crates/vb_ui_makepad/src/
  app.rs
  shell.rs
  theme.rs
  tokens.rs
  data.rs
  screens/
    execution_overview.rs
    workflow_graph_authoring.rs
    execution_details.rs
    verification_certificate.rs
    replay_theater.rs
    incident_failure.rs
    action_registry.rs
    storage_doctor_ai_context.rs
  widgets/
    app_shell.rs
    status_chip.rs
    metric_card.rs
    graph_canvas.rs
    graph_node.rs
    graph_edge.rs
    packet_dot.rs
    timeline_scrubber.rs
    event_table.rs
    slot_diff_table.rs
    certificate_card.rs
    evidence_card.rs
    action_ticket_card.rs
    taint_overlay.rs
    shard_flow_map.rs
    ai_context_panel.rs
  motion/
    timeline.rs
    easing.rs
    bounded_animation.rs
```

### Makepad 2.0 Splash Rules

Makepad Splash (`script_mod!`) must be used for layout, static style, theme tokens, and component composition. Rust code handles typed state, event routing, selection, filtering, and artifact binding.

Required pattern:

```rust
use makepad_widgets::*;

app_main!(App);

script_mod! {
    use mod.prelude.widgets.*

    startup() do #(App::script_component(vm)) {
        ui: Root {
            main_window := Window {
                window.inner_size: vec2(1920, 1080)
                body +: {
                    app_shell := AppShell {
                        // Sidebar, top action bar, and routed screen content.
                    }
                }
            }
        }
    }
}

impl App {
    fn run(vm: &mut ScriptVm) -> Self {
        crate::makepad_widgets::script_mod(vm);
        App::from_script_mod(vm, self::script_mod)
    }
}

#[derive(Script, ScriptHook)]
pub struct App {
    #[source]
    source: ScriptObjectRef,
    #[live]
    ui: WidgetRef,
    #[rust]
    state: UiRuntimeState,
}
```

Business state is Rust-owned and typed. Splash values may not become implicit workflow state. Old Makepad 1.x macro-based examples are not the implementation contract for this repository.

### Custom Widgets

The following custom widgets are required:

| Widget | Purpose |
|--------|---------|
| `GraphCanvas` | Pan/zoom workflow and runtime graphs. |
| `GraphNode` | Draw node cards with status, badges, selection, taint state. |
| `GraphEdge` | Draw curved edges, branch labels, packet markers. |
| `PacketDot` | Animated progress packets along edges. |
| `TimelineScrubber` | Replay timeline with event dots and selected seq. |
| `CertificateCard` | Verification proof card. |
| `StatusChip` | Compact semantic status display. |
| `EvidenceCard` | Digest, journal, artifact, policy evidence. |
| `SlotDiffTable` | Before/after slot and taint changes. |
| `ShardFlowMap` | Overview shard lanes and queue pressure. |
| `ActionTicketCard` | Action id, attempt, idempotency key, replay safety. |
| `AiContextPanel` | AI-safe context packet and suggested commands. |

### Rendering Rules

- Graph edges, packet dots, timeline dots, glows, and selection halos should be shader-rendered or custom draw widgets, not composed from hundreds of nested generic boxes.
- Text is drawn only where meaningful; animation must not relayout text every frame.
- The graph canvas stores precomputed node positions and edge paths. Per-frame layout recomputation is forbidden.
- Animation loops must be bounded and stop when the view is hidden or the app is idle.
- The UI may allocate during screen load, fixture load, and model update. Continuous per-frame animation should avoid heap allocation.

### Figma-to-Makepad Workflow

1. Figma board defines visual target, spacing, screen taxonomy, and interaction notes.
2. `design/tokens/velvet_ui_tokens.toml` defines implementation tokens.
3. `xtask ui-tokens` generates Makepad Splash token snippets, Figma token import metadata if supported, and Rust constants for layout metrics.
4. Makepad Splash implements app shell and reusable components.
5. `xtask ui-snapshot` captures deterministic screenshots from demo fixtures.
6. Screenshot diff gates catch overlap, alignment, density, and regression issues.

Figma is not the source of runtime data. Makepad is not allowed to scrape Figma assets at runtime.

### Layout and Alignment Rules

Every screen uses a 1920x1080 baseline layout with scalable constraints.

Required frame metrics from the 11:51 design bundle:

```text
Window baseline:       1920 x 1080
Outer margin:          32
Sidebar width:         246
Top bar height:        78
Content gutter:        16
Card radius:           14-22
Small radius:          10
Inspector width:       360-420
Bottom timeline min:   220
Graph canvas min:      720 x 520
```

All component positions use an 8px spacing rhythm. One-off pixel nudges are rejected unless documented in a design-token bead.

### UI Snapshot Gate

Every screen must have a deterministic demo fixture and snapshot:

```text
tests/ui_snapshots/execution_overview.png
tests/ui_snapshots/workflow_graph_authoring.png
tests/ui_snapshots/execution_details.png
tests/ui_snapshots/verification_certificate.png
tests/ui_snapshots/replay_theater.png
tests/ui_snapshots/incident_failure.png
tests/ui_snapshots/action_registry.png
tests/ui_snapshots/storage_doctor_ai_context.png
```

Snapshot diff acceptance:

- No overlapping panels.
- No clipped primary labels.
- No unreadable chips.
- No controls outside safe bounds.
- No hidden selected state.
- No accidental color-system drift.
- No canonical spelling violations.

---

# velvet-ballastics — Makepad UI and Product Design Update

**Patch purpose:** drop-in master-plan update for the white Apple Pro-style Makepad UI, Figma-ready design system, and implementation contract.

**Apply this update by:**

1. Replacing current Section 76 with the replacement Section 76 below.
2. Adding Sections 78–83 after Section 77.
3. Adding the workspace and phase-table edits listed in Section A of this patch.
4. Adding the new bead groups listed in Section B of this patch.

---

## A. Master Plan Structural Edits

### A.1 Workspace structure addition

Update Section 23 workspace target structure to include UI model and Makepad UI crates:

```text
velvet-ballastics/
  Cargo.toml
  rust-toolchain.toml
  clippy.toml
  justfile
  deny.toml
  moon.yml
  supply-chain/
    config.toml
  contracts/
    ui_artifacts.yaml
    ui_tokens.yaml
    ui_motion.yaml
    ui_screens.yaml
  design/
    figma/
      velvet_ballastics_figma_ready_tightened_board.png
      velvet_ballastics_figma_ready_tightened_screens.zip
    tokens/
      velvet_ui_tokens.toml
    reference/
      white_makepad_8_screen_board.png
      screenshots/
  crates/
    vb_core/
    vb_yaml/
    vb_validate/
    vb_expr/
    vb_compile/
    vb_storage/
    vb_runtime/
    vb_ipc/
    vb_codegen/
    vb_ui_model/
    vb_ui_makepad/
    velvet_ballastics/
  benches/
  fuzz/
  tests/
```

`vb_ui_model` is a cold-path typed artifact crate shared by CLI and UI. `vb_ui_makepad` is the native desktop UI crate. Neither crate may introduce Makepad, graphics, windowing, or UI dependencies into `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc`, or generated workflow code.

### A.2 Workspace dependency addition

Makepad is approved only for the UI crate after a dependency-scope bead pins the exact version or git revision and records cargo-audit, cargo-deny, cargo-vet, cargo-geiger, cargo-machete, and license evidence.

```toml
[workspace.dependencies]
# UI crate only. Forbidden in runtime core crates.
makepad-widgets = { version = "1", default-features = false }
```

If a git revision is required for Makepad functionality, the dependency must be pinned by exact commit SHA, not branch name, before release.

### A.3 Workspace members addition

```toml
[workspace]
members = [
  "crates/vb_core",
  "crates/vb_yaml",
  "crates/vb_validate",
  "crates/vb_expr",
  "crates/vb_compile",
  "crates/vb_storage",
  "crates/vb_runtime",
  "crates/vb_ipc",
  "crates/vb_codegen",
  "crates/vb_ui_model",
  "crates/vb_ui_makepad",
  "crates/velvet_ballastics",
  "fuzz",
]
```

### A.4 CLI command additions

Add UI commands to Section 33 and Section 69:

```bash
velvet-ballastics ui --db <path>
velvet-ballastics ui --socket <path>
velvet-ballastics ui --demo-fixture <fixture>
velvet-ballastics graph <workflow.yaml> --emit yaml
velvet-ballastics system status --emit yaml
velvet-ballastics action list --emit yaml
velvet-ballastics action inspect <action-name> --emit yaml
velvet-ballastics incident <run_id> --db <path> --emit yaml
velvet-ballastics ai context <run_id> --db <path> --emit yaml
```

The `ui` command launches the Makepad desktop application. It may run in one of three modes:

1. `--db <path>` embedded local observer mode using storage readers and direct APIs.
2. `--socket <path>` attached mode using binary IPC only.
3. `--demo-fixture <fixture>` deterministic mock mode for design review, demos, screenshots, and UI tests.

No UI mode may require HTTP or JSON. Any future web adapter remains a separate cold-path adapter crate and cannot enter runtime core.

---

## B. New Required Bead Groups

Add these bead groups to Section 42:

```text
ui-model-artifacts
ui-design-tokens
ui-makepad-shell
ui-graph-canvas
ui-execution-observatory
ui-verification-certificate-view
ui-replay-theater
ui-incident-console
ui-action-registry
ui-storage-doctor
ui-ai-context-panel
ui-motion-system
ui-figma-import-export
ui-snapshot-regression
ui-performance-gates
makepad-dependency-scope
```

Required first UI beads:

```text
ui-white-apple-pro-design-system
ui-eight-screen-taxonomy
ui-figma-ready-token-export
ui-makepad-live-design-shell
ui-step-functions-observability-layout
ui-runtime-graph-canvas
ui-replay-timeline-scrubber
ui-certificate-cards
ui-incident-evidence-chain
ui-ipc-observer-mode
```

---

# Replacement Section 76 — Workflow Command-Center Front-End

## 76. Workflow Command-Center Front-End

### Vision

The `velvet-ballastics` front-end is a premium native command center for workflow execution observability, verification, replay, and incident response. It is not a generic SaaS dashboard, not a low-code canvas, and not a decorative graph editor.

The UI product identity is:

> Step Functions observability, but cleaner, sharper, calmer, and more cinematic — a workflow black-box recorder inside an Apple-quality native desktop app.

The UI visualizes operational truth already produced by the backend: `VerificationReport`, `WorkflowGraph`, `RunInspection`, `RunEvents`, `ReplayReport`, `IncidentReport`, `SystemStatus`, `ActionDescription`, storage health, journal evidence, action tickets, resource budgets, and taint paths. The UI does not invent state and does not become a second source of truth.

### Design Direction

The v1 UI uses a crisp Apple Pro-style light shell:

- Ultra-clean off-white surfaces.
- Matte white cards.
- Faint translucent glass panels only where useful.
- Hairline dividers instead of heavy borders.
- Soft, realistic shadows.
- Rounded 14–20px cards.
- Precise 8px spacing rhythm.
- Minimal, high-signal color use.
- Crisp sans-serif typography for labels.
- Monospace only for run IDs, action IDs, digests, slot IDs, timestamps, sequence numbers, and binary/record metadata.
- No cyberpunk treatment, no overuse of neon, no overuse of glass, no thick borders, no 3D effects, and no generic web-dashboard chrome.

The UI may borrow broad observability structure from AWS Step Functions-style execution pages — execution summary, graph/table/event views, selected-step details, event history, recovery controls — but it must reinterpret these into the `velvet-ballastics` product model: accepted artifacts, verification certificates, typed journals, replay safety, idempotency evidence, and taint/resource contracts.

### Presentation Board and Figma Contract

The design review artifact is an 8-screen desktop board:

```text
1. Execution Observatory Overview
2. Workflow Graph Authoring
3. Execution Details Graph View
4. Verification Certificate View
5. Replay Theater
6. Incident Failure Console
7. Action Registry / Contract Inspector
8. Storage / Journal Doctor + AI Context
```

Reference design assets live under:

```text
design/figma/
design/reference/
design/tokens/
```

Figma files, SVGs, and PNG boards are design reference only. The implementation source of truth is Makepad Live Design plus Rust widget code. Any design token divergence between Figma and Makepad is a release blocker.

### Shared App Chrome

Every screen uses one shared shell:

- Left sidebar with `velvet-ballastics` branding.
- Minimal icon navigation:
  - Overview
  - Workflow Graph
  - Executions
  - Verification
  - Replay
  - Incidents
  - Actions
  - Storage
  - AI Context
  - Settings
- Top action bar with compact capsule buttons:
  - Verify
  - Simulate
  - Submit
- Status chips:
  - Strict durability
  - Running
  - Verified
  - Replay safe
  - Needs operator
- Top right utility controls:
  - profile/environment selector
  - local server status
  - notification indicator
  - optional command palette trigger

Shared app chrome must be implemented once as `AppShell`, not copied per screen.

### Color System

Use color only for state and meaning:

| Meaning | Color role |
|--------|------------|
| Verified / succeeded / healthy | Green |
| Running / active / selected | Blue or cyan |
| Retry / warning / queue pressure | Amber |
| Failed / critical incident | Red |
| Taint / secret-sensitive path | Purple |
| Durable / replay-safe | Teal |
| Disabled / pending / muted | Gray |

The default UI is calm white, gray, and black. Accent color should appear as small chips, thin outlines, dots, timeline marks, node glows, graph packet markers, and status text. Large colored surfaces are reserved for rare success/failure banners and must stay visually restrained.

### Screen 1 — Execution Observatory Overview

Purpose: answer what is running, where pressure is building, and whether the local system is healthy.

Required elements:

- KPI row:
  - Active runs
  - Healthy actions
  - Verification pass rate
  - Queue depth
  - Open incidents
- Simplified executions table:
  - run id
  - workflow
  - status
  - started
  - duration
  - shard
  - result
- Shard flow map:
  - shard lanes
  - tiny packet dots moving through active executions
  - queue pressure marks
  - action completion lane
  - timer lane
- Event ticker:
  - last N events
  - `RunAccepted`, `StepStarted`, `ActionScheduled`, `ActionCompleted`, `RunFinished`, `RunFailed`
- System health cards:
  - local server online
  - Fjall store healthy
  - writer queue health
  - IPC socket status

Style: calm, spacious, operational, less dense than the Step Functions console or the previous dark reference board.

### Screen 2 — Workflow Graph Authoring

Purpose: show the compiled workflow graph as a structured projection of YAML/IR, not as a freeform whiteboard.

Required elements:

- State palette on the left:
  - Start
  - Action
  - Branch
  - Parallel
  - Wait
  - Subflow
  - Finish
- Center graph canvas:
  - `Start`
  - `classify`
  - `route_issue`
  - `create_issue`
  - `notify_slack`
  - `build_result`
  - `Finish`
- Node cards:
  - matte white card
  - status dot
  - primitive/action label
  - small badges for strict-safe, idempotency, taint, retry, timeout
- Edges:
  - thin curved lines
  - tiny packet markers
  - branch labels
  - selected path emphasis
- Right step inspector:
  - step name
  - primitive
  - action id
  - resource impact
  - input slots
  - output slot
  - retry policy
  - idempotency key
  - taint state
- Selected node:
  - crisp blue outline
  - subtle glow
  - no large blue fill

YAML source remains authoritative. The canvas may support structured editing only if edits round-trip through the parser/compiler/validator.

### Screen 3 — Execution Details Graph View

Purpose: inspect one active or past run in graph mode.

Required elements:

- Run summary:
  - run id
  - workflow name
  - status
  - started timestamp
  - shard id
  - durability profile
- Runtime graph:
  - succeeded nodes in green
  - selected/running node in blue
  - pending nodes muted gray
  - failed node red outline
  - secret/taint overlay purple only when active
- Event table below graph:
  - seq
  - time
  - step
  - event
  - shard
  - evidence id
- Right step details panel:
  - step name
  - action id
  - action type
  - attempt
  - started time
  - elapsed
  - idempotency key hash
  - input tab
  - output tab
  - details tab

This screen is the closest structural analog to Step Functions execution details, but it must show velvet-native concepts: journal evidence, action tickets, taint, slots, replay safety, and artifact digests.

### Screen 4 — Verification Certificate View

Purpose: pre-flight safety certificate for accepted artifacts.

Required elements:

- Green restrained banner: `Verification passed` or equivalent failure banner.
- Certificate cards:
  - Structure
  - Boundedness
  - Resources
  - Taint / Secrets
  - Action policy
  - Durability
  - Idempotency
  - Capability
- Horizontal verification gate pipeline:
  - Parse
  - Graph check
  - Policy
  - Resources
  - Taint
  - Durability
  - Idempotency
  - Capability
  - Result
- Accepted artifact side panel:
  - artifact version
  - workflow version
  - workflow digest
  - IR digest
  - action ABI digest
  - policy digest
  - verified timestamp
  - warnings
- Proof summary:
  - bounded
  - taint safe
  - retry safe
  - durable
  - replayable

This screen must feel like a safety certificate, not an analytics dashboard.

### Screen 5 — Replay Theater

Purpose: the hero screen. A premium black-box recorder for deterministic workflow replay.

Required elements:

- Runtime graph on the left or center.
- Journal timeline:
  - event dots by sequence number
  - selected event highlight
  - scrubber position
  - jump to failure
  - jump to action
  - jump to divergence
- Playback controls:
  - back
  - play/pause
  - step forward
  - replay speed
  - live/frozen mode
- Selected event panel:
  - seq
  - timestamp
  - shard
  - step
  - event kind
  - evidence id
  - digest summary
- Slot diff table:
  - slot id
  - before
  - after
  - taint before
  - taint after
- Recovery decision panel:
  - strategy
  - max attempts
  - idempotency requirement
  - apply/replay action

This screen should feel like a video editor or flight recorder: calm, precise, replayable, and cinematic. Motion is implied by packet dots, scrubber state, event pulses, and graph overlays.

### Screen 6 — Incident Failure Console

Purpose: incident diagnosis and safe recovery.

Required elements:

- Red restrained banner:
  - `ACTION_TIMEOUT at create_issue`
  - run id
  - action id
  - attempt
  - timestamp
- Compact chips:
  - `Safe to retry: YES`
  - `Same idempotency key required`
  - `Strict durability`
  - `Replay safe`
- Failure path graph:
  - failure node red outline
  - failure path focus
  - muted non-failure nodes
- Evidence chain:
  - scheduled durable
  - completion durable
  - side-effect certainty
  - journal tail
- Recovery controls:
  - retry same key
  - schedule retry
  - cancel run
  - open replay
- Action ticket panel:
  - ticket id
  - action id
  - attempt
  - owner
  - rollback/retry metadata
- Slot and taint diff panels.
- Repair hints:
  - check API status
  - verify token scope
  - increase timeout
  - retry with backoff

Do not flood the screen with red. Red is reserved for the failure node, banner accent, and critical text.

### Screen 7 — Action Registry / Contract Inspector

Purpose: inspect registered native actions, numeric `ActionId` mappings, contracts, capabilities, idempotency, retry safety, and schema/digest metadata.

Required elements:

- Action list:
  - name
  - action id
  - side effect class
  - idempotency
  - retry safety
  - strict safe
  - required capability
- Selected action inspector:
  - `ActionContract`
  - input slot count
  - output slot count
  - max input bytes
  - max output bytes
  - timeout ms
  - idempotency classification
  - side effect classification
  - retry safety
  - action ABI digest
- Capability panel:
  - required permissions
  - granted permissions
  - missing permissions
- Failure code panel:
  - `RateLimited`
  - `Timeout`
  - `PermissionDenied`
  - `InvalidInput`
  - `ExternalUnavailable`
- Example call view:
  - no JSON
  - no HTTP core routing
  - typed binary/postcard schema summary

### Screen 8 — Storage / Journal Doctor + AI Context

Purpose: storage health, journal evidence, replay readiness, and AI-safe operational context.

Required elements:

- Storage health:
  - Fjall keyspaces
  - writer queue
  - journal batch health
  - snapshot status
  - blob store status
  - index health
- Journal doctor:
  - run event count
  - snapshot seq
  - tail seq
  - corrupt record status
  - trim recommendation
  - digest checks
- AI context packet:
  - safe for model
  - secrets redacted
  - blobs summarized
  - suggested next commands
  - failure summary
  - replay safety
- Evidence card:
  - last cert check
  - last replay check
  - last crash lab fixture
  - incomplete evidence warnings

### AI Companion Panel

The AI panel is not a generic chat sidebar. It receives structured artifacts only:

- `WorkflowGraph`
- `VerificationReport`
- `RunInspection`
- `RunEvents`
- `ReplayReport`
- `IncidentReport`
- `SystemStatus`
- `ActionDescription`
- `AiContextPacket`

Prompts are action buttons, not open-ended chat by default:

- Explain this failure.
- Is this safe to retry?
- Show secret-sensitive paths.
- Explain strict-durability failure.
- Generate minimal repro.
- Suggest bounded retry policy.
- Summarize what changed since last good run.

AI output must cite graph nodes, journal events, slot diffs, action tickets, certificates, or diagnostics. AI output must never rely on hidden UI state.

### UI Build Order

| UI Phase | Deliverable | Why first |
|----------|-------------|-----------|
| UI-1 | `vb_ui_model` typed artifacts | Shared truth for CLI and UI. |
| UI-2 | Makepad app shell and design tokens | Common chrome, spacing, color, typography. |
| UI-3 | Replay Theater | Exercises hardest event/timeline/graph mapping first. |
| UI-4 | Verification Certificate View | Product differentiation and accepted-artifact proof surface. |
| UI-5 | Execution Details Graph View | Step Functions-style observability with velvet-native evidence. |
| UI-6 | Incident Failure Console | Operational recovery path. |
| UI-7 | Execution Observatory Overview | Macro health after per-run views work. |
| UI-8 | Workflow Graph Authoring | Structured graph projection and editing. |
| UI-9 | Action Registry / Storage Doctor / AI Context | Operator completeness. |
| UI-10 | Motion/perf/snapshot gates | Release readiness. |

The backend and CLI remain higher priority than decorative UI polish. UI concepts cannot introduce product states not emitted by backend artifacts.

---

# Section 78 — Makepad UI Implementation Contract

## 78. Makepad UI Implementation Contract

### Makepad Scope

Makepad is used only for the native UI crate `vb_ui_makepad`. It is forbidden in:

```text
vb_core
vb_runtime
vb_storage
vb_ipc
vb_codegen generated output
generated workflow code
```

Makepad dependencies must not change runtime semantics, binary IPC semantics, persistence semantics, or generated Rust workflow semantics.

### Makepad Rationale

Makepad is selected for the UI because the design requires a native, GPU-driven desktop application with highly interactive graph, timeline, animation, and custom-rendered visual states. The UI uses Makepad Live Design for layout/style iteration and Rust widgets for deterministic state handling.

### Crate Roles

| Crate | Role | Runtime-core dependency? |
|------|------|--------------------------|
| `vb_ui_model` | Typed UI artifacts shared by CLI/UI. No Makepad. | Cold path only |
| `vb_ui_makepad` | Native Makepad desktop app. | UI only |
| `velvet_ballastics` | CLI command dispatch, including `ui`. | Cold path command |

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
| Embedded | `velvet-ballastics ui --db <path>` | Direct storage/runtime readers | Local desktop app with DB access |
| Attached | `velvet-ballastics ui --socket <path>` | Binary IPC | Operator app connected to running server |
| Demo | `velvet-ballastics ui --demo-fixture <fixture>` | Deterministic fixtures | Design review, screenshot tests, demos |

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

### Live Design Rules

Makepad Live Design must be used for layout, static style, theme tokens, and component composition. Rust code handles typed state, event routing, selection, filtering, and artifact binding.

Required pattern:

```rust
use makepad_widgets::*;

live_design! {
    import makepad_widgets::base::*;
    import makepad_widgets::theme_desktop_light::*;

    App = {{App}} {
        ui: <Window> {
            // AppShell with sidebar/topbar/content
        }
    }
}

app_main!(App);

#[derive(Live, LiveHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    state: UiRuntimeState,
}
```

Business state is Rust-owned and typed. Live Design values may not become implicit workflow state.

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
3. `xtask ui-tokens` generates:
   - Makepad Live Design token snippets
   - Figma token JSON/import metadata if supported
   - Rust constants for layout metrics
4. Makepad Live Design implements app shell and reusable components.
5. `xtask ui-snapshot` captures deterministic screenshots from demo fixtures.
6. Screenshot diff gates catch overlap, alignment, density, and regression issues.

Figma is not the source of runtime data. Makepad is not allowed to scrape Figma assets at runtime.

### Layout and Alignment Rules

Every screen uses a 1920×1080 baseline layout with scalable constraints.

Required frame metrics:

```text
Window baseline:       1920 × 1080
Outer margin:          24
Sidebar width:         220
Top bar height:        64
Content gutter:        16
Card radius:           16
Small radius:          10
Inspector width:       360–420
Bottom timeline min:   220
Graph canvas min:      720 × 520
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

# Section 79 — UI Design System Tokens

## 79. UI Design System Tokens

### Design Token Source

The design token source is:

```text
design/tokens/velvet_ui_tokens.toml
```

Generated outputs:

```text
crates/vb_ui_makepad/src/generated/tokens.rs
crates/vb_ui_makepad/src/generated/tokens.live
contracts/ui_tokens.yaml
```

Manual edits to generated token files are rejected.

### Color Tokens

```toml
[color]
background_board = "#F4F6F8"
shell = "#F8FAFC"
surface = "#FFFFFF"
surface_glass = "#FFFFFFCC"
surface_muted = "#F2F5F8"
line_hair = "#DDE3EA"
line_soft = "#E8EDF2"
text_primary = "#101828"
text_secondary = "#475467"
text_tertiary = "#7A8796"

success = "#16A66A"
running = "#1F7AF5"
active_cyan = "#19A7CE"
warning = "#F59E0B"
failure = "#E5484D"
taint = "#8B5CF6"
durable = "#14B8A6"
pending = "#98A2B3"
```

### Typography Tokens

```toml
[type]
family_sans = "Inter, SF Pro, system-ui"
family_mono = "JetBrains Mono, SF Mono, ui-monospace"
size_11 = 11
size_12 = 12
size_13 = 13
size_14 = 14
size_16 = 16
size_20 = 20
size_24 = 24
weight_regular = 400
weight_medium = 500
weight_semibold = 600
```

Monospace may be used only for:

```text
RunId
ActionId
WorkflowDigest
SeqNo
SlotIdx
StepIdx
timestamps
record kind IDs
IPC frame fields
artifact digests
```

### Spacing Tokens

```toml
[space]
px_4 = 4
px_8 = 8
px_12 = 12
px_16 = 16
px_20 = 20
px_24 = 24
px_32 = 32
px_40 = 40
```

### Radius Tokens

```toml
[radius]
chip = 10
control = 12
card = 16
panel = 20
window = 24
```

### Shadow Tokens

```toml
[shadow]
card = "0 8 24 rgba(16,24,40,0.08)"
window = "0 20 60 rgba(16,24,40,0.14)"
focus = "0 0 0 4 rgba(31,122,245,0.14)"
failure = "0 0 0 4 rgba(229,72,77,0.12)"
taint = "0 0 0 4 rgba(139,92,246,0.12)"
```

### Density Rule

The UI must be spacious. Data tables are allowed, but screen density must not exceed these baseline limits:

| Screen | Max primary panels | Max table rows visible by default |
|--------|--------------------|-----------------------------------|
| Execution overview | 6 | 7 |
| Workflow authoring | 4 | 0 |
| Execution details | 5 | 7 |
| Verification | 6 | 0 |
| Replay theater | 6 | 6 |
| Incident console | 7 | 5 |
| Action registry | 6 | 8 |
| Storage doctor / AI context | 7 | 8 |

If more data is available, use scroll, filters, disclosure, pagination, or drill-in.

---

# Section 80 — UI Motion and Interaction Contract

## 80. UI Motion and Interaction Contract

### Principle

Animation must communicate state, causality, and replay timing. Decorative animation is rejected. Motion must be calm, bounded, and GPU-friendly.

### Required Motion Primitives

| Motion | Purpose | Screens |
|--------|---------|---------|
| Packet dots on edges | Show work moving through workflow graph. | Overview, graph, execution, replay |
| Active node glow | Show selected/running step. | Graph, execution, replay |
| Timeline scrubber | Show replay position. | Replay theater, execution details |
| Selected event pulse | Show current journal event. | Replay theater |
| Failure path focus | Guide attention to failed node and evidence chain. | Incident console |
| Taint overlay | Show secret-sensitive path. | Verification, replay, incident |
| Queue pressure shimmer | Indicate rising queue pressure without noise. | Overview |
| Certificate check cascade | Show verification gate pass sequence. | Verification |

### Motion Budget

```text
Target frame rate:          60fps minimum, 120fps when available
Max animated graph nodes:   256 visible
Max animated packet dots:   512 visible
Max timeline event dots:    2,000 visible before clustering
Max per-frame allocations:  0 in animation loops after warm-up
Max animation tick when hidden: 0
```

### Animation State Rules

- Animation state is UI state only; it never mutates runtime state.
- Animation tickers pause when the screen is not visible.
- Demo/snapshot mode must support deterministic time control.
- Replay scrubber state must bind to `SeqNo`, not wall-clock time.
- Packet animation may interpolate over precomputed edge paths but must not change graph topology.
- Failure pulse and taint overlay must be accessible through static visual indicators as well.

### Interaction Rules

Required interactions:

- Pan and zoom graph canvas.
- Click node to open step inspector.
- Hover node to show compact digest/resource/taint tooltip.
- Click event row to sync graph and timeline.
- Drag replay scrubber to any journal event.
- Filter events by step, event kind, taint, and action id.
- Toggle taint overlay.
- Toggle evidence overlay.
- Open action ticket from event or failed node.
- Copy digest/run/action IDs from monospace fields.
- Open AI context packet from run/incident.

Forbidden interactions:

- Hidden destructive actions without explicit confirmation or `--force` equivalent.
- UI-only retry behavior not represented by CLI lifecycle command.
- Freeform graph edits that bypass validation.
- Unbounded event list rendering.

---

# Section 81 — UI Artifact and Schema Contract

## 81. UI Artifact and Schema Contract

### Shared Artifact Rule

The UI and CLI render the same typed artifacts. A screen cannot display data unless the corresponding CLI command can emit it in structured form.

| UI screen | Required artifact | CLI parity command |
|-----------|-------------------|--------------------|
| Execution Overview | `SystemStatus`, `RunSummaries`, `RunEvents` | `system status --emit yaml`, `events` |
| Workflow Graph Authoring | `WorkflowGraph` | `graph --emit yaml` |
| Execution Details | `RunInspection`, `RunEvents` | `inspect --emit yaml`, `events --emit yaml` |
| Verification Certificate | `VerificationReport`, `AcceptedArtifact` | `verify --emit yaml` |
| Replay Theater | `ReplayReport`, `RunEvents`, `SlotDiffs` | `replay --explain --emit yaml` |
| Incident Console | `IncidentReport` | `incident --emit yaml` |
| Action Registry | `ActionDescription`, `ActionList` | `action list`, `action inspect` |
| Storage Doctor / AI Context | `DoctorReport`, `AiContextPacket` | `doctor --emit yaml`, `ai context --emit yaml` |

### Required UI Model Fields

Every UI artifact must include:

```text
schema_version
kind
generated_at
source
redaction_status
```

Every graph node must include:

```text
step_idx
step_id
kind
status
output_slot
taint
badges
position
```

Every graph edge must include:

```text
from_step_idx
to_step_idx
edge_kind
condition_summary
is_failure_path
is_taint_path
packet_state
```

Every event row must include:

```text
seq
timestamp
run_id
step_idx
event_kind
status
evidence_digest
attempt
```

Every action ticket view must include:

```text
ticket_digest
run_id
step_idx
action_id
attempt
idempotency_key_hash
scheduled_durable
completion_durable
replay_safe
side_effect_certainty
```

### Redaction Rule

The UI must never render raw secret values. Secret-sensitive values are represented by:

```text
redacted: true
taint: Secret | DerivedFromSecret
digest: blake3:<prefix>
summary: <bounded static summary>
```

Any UI path that displays full blobs or raw action details must require an explicit unsafe operator action and must be disabled in AI context mode.

---

# Section 82 — UI Implementation Phases

## 82. UI Implementation Phases

Add these phases after Phase 60 in Section 70:

| Phase | Name | Required delivery |
|-------|------|-------------------|
| 61 | UI model artifacts | `vb_ui_model` crate with typed `WorkflowGraph`, `VerificationReport`, `RunInspection`, `RunEvents`, `ReplayReport`, `IncidentReport`, `SystemStatus`, `ActionDescription`, `DoctorReport`, and `AiContextPacket` views. CLI/UI schema parity tests. |
| 62 | Makepad shell | `vb_ui_makepad` crate, shared app chrome, sidebar, topbar, command buttons, status chips, profile selector, demo fixture loading. |
| 63 | Design tokens and Figma bridge | Token source in `design/tokens`; generated Makepad token files; Figma-ready SVG/PNG references; token drift checker. |
| 64 | Graph canvas | Pan/zoom canvas, nodes, curved edges, packet dots, selection, status color rules, taint overlay, layout fixtures. |
| 65 | Execution observatory | Overview KPIs, shard flow map, active runs table, event ticker, queue pressure indicators, storage/IPC health summary. |
| 66 | Execution details view | Single-run graph view, event table, step details panel, input/output/details tabs, runtime state coloring. |
| 67 | Verification certificate view | Verification banner, certificate cards, gate pipeline, accepted artifact panel, warnings, proof summary. |
| 68 | Replay theater | Journal timeline, playback controls, scrubber, selected event details, slot diffs, recovery decision panel, deterministic replay fixture. |
| 69 | Incident failure console | Failure banner, failure path graph, evidence chain, action ticket, recovery controls, slot/taint diffs, repair hints. |
| 70 | Action registry / contract inspector | Action list, selected `ActionContract`, idempotency/side-effect/retry safety, capability requirements, failure codes. |
| 71 | Storage doctor / AI context | Fjall keyspace health, journal doctor, snapshot/tail status, AI-safe context packet, suggested commands. |
| 72 | UI motion/performance | Shader-based packet dots, active-node glow, timeline pulse, bounded animation loops, no per-frame allocations after warm-up, UI perf benchmark. |
| 73 | UI snapshot and overlap gates | Deterministic screenshots for all eight screens, image diff gate, overlap/clipping scanner, canonical spelling scan. |
| 74 | UI release hardening | Keyboard navigation, accessibility labels, redaction tests, CLI/UI parity tests, demo fixtures, documentation, Makepad dependency audit. |

---

# Section 83 — UI Testing, Benchmarking, and Acceptance Gates

## 83. UI Testing, Benchmarking, and Acceptance Gates

### UI Tests

Required tests:

- `ui_model_schema_versions_are_stable`
- `ui_artifacts_match_cli_output_kinds`
- `workflow_graph_view_has_no_missing_nodes`
- `workflow_graph_edges_reference_valid_nodes`
- `event_rows_are_bounded`
- `ai_context_redacts_secrets`
- `incident_report_has_replay_safety`
- `verification_certificate_maps_all_gates`
- `action_ticket_hides_raw_idempotency_key`
- `ui_tokens_generate_makepad_and_contract_outputs`
- `all_screens_have_demo_fixtures`

### UI Snapshot Tests

Required deterministic snapshot fixtures:

```text
fixtures/ui/execution_overview.fixture
fixtures/ui/workflow_graph_authoring.fixture
fixtures/ui/execution_details.fixture
fixtures/ui/verification_certificate.fixture
fixtures/ui/replay_theater.fixture
fixtures/ui/incident_failure.fixture
fixtures/ui/action_registry.fixture
fixtures/ui/storage_doctor_ai_context.fixture
```

Snapshot command:

```bash
cargo xtask ui-snapshot --all --emit yaml
```

Snapshot report:

```yaml
kind: UiSnapshotReport
status: pass
screens:
  - screen: execution_overview
    png: tests/ui_snapshots/execution_overview.png
    overlap_check: pass
    clipping_check: pass
    spelling_check: pass
    token_check: pass
```

### UI Performance Benchmarks

Required UI benchmarks:

| Benchmark | Requirement |
|----------|-------------|
| `ui_graph_pan_zoom_256_nodes` | Smooth interaction, no unbounded allocation. |
| `ui_graph_packet_animation_512_packets` | Animation remains within frame budget. |
| `ui_timeline_2000_events_clustered` | Timeline remains responsive. |
| `ui_event_table_scroll_10000_bounded` | Virtualized/bounded rendering only. |
| `ui_replay_scrub_1000_events` | Scrub updates selected graph/event without full relayout. |
| `ui_fixture_load_all_screens` | Demo fixtures load under bounded memory. |

### UI Acceptance Commands

```bash
cargo +nightly fmt --all -- --check
cargo +nightly clippy -p vb_ui_model -p vb_ui_makepad --all-targets --all-features -- -D warnings
cargo +nightly nextest run -p vb_ui_model -p vb_ui_makepad
cargo xtask ui-tokens --check
cargo xtask ui-snapshot --all
cargo xtask ui-overlap-check --all
cargo xtask ui-perf-smoke
cargo xtask forbidden-scan --changed
cargo xtask hotpath-scan --changed
```

### UI Definition of Done

The Makepad UI is accepted only when:

1. All eight required screens exist and are reachable from shared app chrome.
2. Every screen consumes typed `vb_ui_model` artifacts.
3. CLI/UI parity exists for all displayed artifact kinds.
4. Figma token source and Makepad token output are synchronized.
5. No UI panel overlap, clipping, or unreadable primary label exists in 1920×1080 baseline screenshots.
6. All secret-sensitive values are redacted or summarized.
7. Graph, replay, incident, and verification views expose journal/digest/evidence concepts accurately.
8. Motion is bounded, meaningful, and can be disabled or frozen for deterministic snapshots.
9. UI code does not introduce Makepad, HTTP, JSON, async runtimes, or web dependencies into runtime core crates.
10. UI snapshot, token, model, parity, redaction, performance-smoke, lint, and test gates pass with evidence.

---
section: 76
title: "Workflow Command-Center Front-End"
parent: velvet-ballistics-MASTER.md
---

## 76. Workflow Command-Center Front-End


> **Removed.** The command-center front-end is not part of the current Backend / IR Interpreter Complete milestone. The remaining section content is historical residue only and not an implementation contract.

### Vision

The `velvet-ballistics` front-end is a premium native command center for workflow execution observability, verification, replay, and incident response. It is not a generic SaaS dashboard, not a low-code canvas, and not a decorative graph editor.

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

The UI may borrow broad observability structure from AWS Step Functions-style execution pages — execution summary, graph/table/event views, selected-step details, event history, recovery controls — but it must reinterpret these into the `velvet-ballistics` product model: accepted artifacts, verification certificates, typed journals, replay safety, idempotency evidence, and taint/resource contracts.

### Presentation Board and Figma Contract

The current canonical intake bundle is the 2026-05-08 23:51 zip at `/home/lewis/Downloads/velvet_ballistics_makepad_ui_master_plan_with_images.zip`. Its extracted repository copy is:

```text
velvet_ballistics_makepad_ui_master_plan_with_images/
  velvet-ballistics-MASTER-makepad-ui-update.md
  design_assets/canonical/
    figma_makepad_notes.md
    velvet_ballistics_figma_ready_tightened_board.png
  design_assets/velvet_ballistics_figma_ready_tightened/png/
  design_assets/velvet_ballistics_figma_ready_tightened/svg/
```

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

Figma files, SVGs, and PNG boards are design reference only. The implementation source of truth is Makepad Splash (`script_mod!`) plus Rust widget code. Any design token divergence between Figma and Makepad is a release blocker.

### Shared App Chrome

Every screen uses one shared shell:

- Left sidebar with `velvet-ballistics` branding.
- Minimal icon navigation: Overview, Workflow Graph, Executions, Verification, Replay, Incidents, Actions, Storage, AI Context, Settings.
- Top action bar with compact capsule buttons: Verify, Simulate, Submit.
- Status chips: Strict durability, Running, Verified, Replay safe, Needs operator.
- Top right utility controls: profile/environment selector, local server status, notification indicator, optional command palette trigger.

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

- KPI row: Active runs, Healthy actions, Verification pass rate, Queue depth, Open incidents.
- Simplified executions table: run id, workflow, status, started, duration, shard, result.
- Shard flow map: shard lanes, tiny packet dots moving through active executions, queue pressure marks, action completion lane, timer lane.
- Event ticker: last N events, `RunAccepted`, `StepStarted`, `ActionScheduled`, `ActionCompleted`, `RunFinished`, `RunFailed`.
- System health cards: local server online, Fjall store healthy, writer queue health, IPC socket status.

Style: calm, spacious, operational, less dense than the Step Functions console or the previous dark reference board.

### Screen 2 — Workflow Graph Authoring

Purpose: show the compiled workflow graph as a structured projection of YAML/IR, not as a freeform whiteboard.

Required elements:

- State palette on the left: Start, Action, Branch, Parallel, Wait, Subflow, Finish.
- Center graph canvas: `Start`, `classify`, `route_issue`, `create_issue`, `notify_slack`, `build_result`, `Finish`.
- Node cards: matte white card, status dot, primitive/action label, small badges for strict-safe, idempotency, taint, retry, timeout.
- Edges: thin curved lines, tiny packet markers, branch labels, selected path emphasis.
- Right step inspector: step name, primitive, action id, resource impact, input slots, output slot, retry policy, idempotency key, taint state.
- Selected node: crisp blue outline, subtle glow, no large blue fill.

YAML source remains authoritative. The canvas may support structured editing only if edits round-trip through the parser/compiler/validator.

### Screen 3 — Execution Details Graph View

Purpose: inspect one active or past run in graph mode.

Required elements:

- Run summary: run id, workflow name, status, started timestamp, shard id, durability profile.
- Runtime graph: succeeded nodes in green, selected/running node in blue, pending nodes muted gray, failed node red outline, secret/taint overlay purple only when active.
- Event table below graph: seq, time, step, event, shard, evidence id.
- Right step details panel: step name, action id, action type, attempt, started time, elapsed, idempotency key hash, input tab, output tab, details tab.

This screen is the closest structural analog to Step Functions execution details, but it must show velvet-native concepts: journal evidence, action tickets, taint, slots, replay safety, and artifact digests.

### Screen 4 — Verification Certificate View

Purpose: pre-flight safety certificate for accepted artifacts.

Required elements:

- Green restrained banner: `Verification passed` or equivalent failure banner.
- Certificate cards: Structure, Boundedness, Resources, Taint / Secrets, Action policy, Durability, Idempotency, Capability.
- Horizontal verification gate pipeline: Parse, Graph check, Policy, Resources, Taint, Durability, Idempotency, Capability, Result.
- Accepted artifact side panel: artifact version, workflow version, workflow digest, IR digest, action ABI digest, policy digest, verified timestamp, warnings.
- Proof summary: bounded, taint safe, retry safe, durable, replayable.

This screen must feel like a safety certificate, not an analytics dashboard.

### Screen 5 — Replay Theater

Purpose: the hero screen. A premium black-box recorder for deterministic workflow replay.

Required elements:

- Runtime graph on the left or center.
- Journal timeline: event dots by sequence number, selected event highlight, scrubber position, jump to failure, jump to action, jump to divergence.
- Playback controls: back, play/pause, step forward, replay speed, live/frozen mode.
- Selected event panel: seq, timestamp, shard, step, event kind, evidence id, digest summary.
- Slot diff table: slot id, before, after, taint before, taint after.
- Recovery decision panel: strategy, max attempts, idempotency requirement, apply/replay action.

This screen should feel like a video editor or flight recorder: calm, precise, replayable, and cinematic. Motion is implied by packet dots, scrubber state, event pulses, and graph overlays.

### Screen 6 — Incident Failure Console

Purpose: incident diagnosis and safe recovery.

Required elements:

- Red restrained banner: `ACTION_TIMEOUT at create_issue`, run id, action id, attempt, timestamp.
- Compact chips: `Safe to retry: YES`, `Same idempotency key required`, `Strict durability`, `Replay safe`.
- Failure path graph: failure node red outline, failure path focus, muted non-failure nodes.
- Evidence chain: scheduled durable, completion durable, side-effect certainty, journal tail.
- Recovery controls: retry same key, schedule retry, cancel run, open replay.
- Action ticket panel: ticket id, action id, attempt, owner, rollback/retry metadata.
- Slot and taint diff panels.
- Repair hints: check API status, verify token scope, increase timeout, retry with backoff.

Do not flood the screen with red. Red is reserved for the failure node, banner accent, and critical text.

### Screen 7 — Action Registry / Contract Inspector

Purpose: inspect registered native actions, numeric `ActionId` mappings, contracts, capabilities, idempotency, retry safety, and schema/digest metadata.

Required elements:

- Action list: name, action id, side effect class, idempotency, retry safety, strict safe, required capability.
- Selected action inspector: `ActionContract`, input slot count, output slot count, max input bytes, max output bytes, timeout ms, idempotency classification, side effect classification, retry safety, action ABI digest.
- Capability panel: required permissions, granted permissions, missing permissions.
- Failure code panel: `RateLimited`, `Timeout`, `PermissionDenied`, `InvalidInput`, `ExternalUnavailable`.
- Example call view: no JSON, no HTTP core routing, typed binary/postcard schema summary.

### Screen 8 — Storage / Journal Doctor + AI Context

Purpose: storage health, journal evidence, replay readiness, and AI-safe operational context.

Required elements:

- Storage health: Fjall keyspaces, writer queue, journal batch health, snapshot status, blob store status, index health.
- Journal doctor: run event count, snapshot seq, tail seq, corrupt record status, trim recommendation, digest checks.
- AI context packet: safe for model, secrets redacted, blobs summarized, suggested next commands, failure summary, replay safety.
- Evidence card: last cert check, last replay check, last crash lab fixture, incomplete evidence warnings.

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

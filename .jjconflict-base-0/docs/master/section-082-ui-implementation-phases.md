---
section: 82
title: "UI Implementation Phases"
parent: velvet-ballistics-MASTER.md
---

## 82. UI Implementation Phases


The UI phase rows in Section 70 define the required delivery sequence after Phase 60:

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

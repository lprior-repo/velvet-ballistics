---
section: 80
title: "UI Motion and Interaction Contract"
parent: velvet-ballistics-MASTER.md
---

## 80. UI Motion and Interaction Contract


> **Removed.** Makepad UI is not part of the current core feature set. This section is historical residue only; no current backend bead may be blocked by UI motion, interaction, or animation gate requirements.

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

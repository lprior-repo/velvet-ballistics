---
section: 79
title: "UI Design System Tokens"
parent: velvet-ballistics-MASTER.md
---

## 79. UI Design System Tokens


> **Removed.** Makepad UI is not part of the current core feature set. This section is historical residue only; no current backend bead may be blocked by UI token, Figma, or Makepad splash gate requirements.

### Design Token Source

The design token source is:

```text
design/tokens/velvet_ui_tokens.toml
```

Generated outputs:

```text
crates/vb_ui_makepad/src/generated/tokens.rs
crates/vb_ui_makepad/src/generated/tokens.splash
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
card_min = 14
card = 16
card_max = 22
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

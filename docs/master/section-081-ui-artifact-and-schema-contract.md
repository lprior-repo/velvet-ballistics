---
section: 81
title: "UI Artifact and Schema Contract"
parent: velvet-ballistics-MASTER.md
---

## 81. UI Artifact and Schema Contract


> **Removed.** Makepad UI is not part of the current core feature set. This section is historical residue only; no current backend bead may be blocked by UI artifact, schema, or CLI parity gate requirements.

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

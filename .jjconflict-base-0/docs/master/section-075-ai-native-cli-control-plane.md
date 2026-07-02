---
section: 75
title: "AI-Native CLI Control Plane"
parent: velvet-ballistics-MASTER.md
---

## 75. AI-Native CLI Control Plane


The CLI is the AI-native control plane for humans and AI agents to operate, verify, repair, replay, and explain the system now.

North star:

1. Anything an adapter can show, the CLI must be able to emit as structured data first.
2. Anything an operator can inspect, an AI agent can inspect safely.
3. Anything that fails produces a machine-readable explanation.

### Dual-Personality Design

The CLI has two modes of output:

**Human mode** — Pretty, readable, fast:

```text
velvet-ballistics verify workflow.yaml
velvet-ballistics run issue_triage --input input.vbin
velvet-ballistics inspect run_123
velvet-ballistics replay run_123
```

Output is colored, summarized, and ergonomic.

**AI mode** — Stable, structured, boring:

```text
velvet-ballistics verify workflow.yaml --emit yaml
velvet-ballistics inspect run_123 --emit yaml
velvet-ballistics replay run_123 --explain --emit yaml
velvet-ballistics incident run_123 --emit yaml
```

No fragile pretty text. No hidden state. No "look at the dashboard." AI mode emits schemas that are documented and versioned.

### Lifecycle Command Surface

Command groups mirror the system lifecycle:

```text
velvet-ballistics validate workflow.yaml
velvet-ballistics verify   workflow.yaml
velvet-ballistics compile  workflow.yaml
velvet-ballistics graph    workflow.yaml
velvet-ballistics simulate workflow.yaml
velvet-ballistics run-compiled workflow.vbir
velvet-ballistics submit   issue_triage
velvet-ballistics inspect  run_123
velvet-ballistics events   run_123
velvet-ballistics replay   run_123
velvet-ballistics incident run_123
velvet-ballistics action list
velvet-ballistics action inspect github.issue.create
velvet-ballistics system status
velvet-ballistics doctor
velvet-ballistics ai context run_123
```

The CLI is not just "run workflow." It is a compiler/debugger/operator interface.

### verify Is the Hero Command

`verify` is the flagship. It answers: *is this workflow safe to run, and if not, what must change?*

```text
velvet-ballistics verify workflow.yaml --profile strict
```

Human output:

```text
✓ structure valid
✓ bounded execution
✓ resource budget computed
✓ no secret-to-result flow
✓ all external actions strict-durable safe
✓ replay policy safe

compiled digest: 8c13...
max transitions: 842
max action calls: 4
max frame bytes: 19.2 KiB
```

AI output (`--emit yaml`):

```yaml
schema_version: velvet-ballistics/cli-output/v1
kind: VerificationReport
workflow:
  name: issue_triage
  source_digest: blake3:...
  compiled_digest: blake3:...
profile: strict
status: pass
certificates:
  structural:
    status: pass
    invalid_edges: []
    unreachable_steps: []
  boundedness:
    status: pass
    max_ir_transitions: 842
    max_action_calls: 4
    max_retries: 3
  resources:
    status: pass
    max_slots: 48
    max_expr_stack: 6
    max_frame_bytes: 19648
    max_result_bytes: 32768
  taint:
    status: pass
    public_result_secret_reachable: false
    forbidden_paths: []
  actions:
    status: pass
    external_actions:
      - action: github.issue.create
        action_id: 7
        idempotency: IdempotentExternal
        strict_safe: true
  durability:
    status: pass
    journal_before_dispatch: true
    completion_before_frame_mutation: true
```

### Structured Diagnostics with Repair Hints

When validation fails, the CLI emits structured repair hints — not just text:

```yaml
schema_version: velvet-ballistics/cli-output/v1
kind: DiagnosticReport
status: fail
diagnostics:
  - code: ACTION_REQUIRES_IDEMPOTENCY
    severity: error
    path: $.steps[2].do
    span:
      line_start: 18
      column_start: 5
      line_end: 29
      column_end: 12
    message: Strict durability requires idempotency for external action http.request.
    repair:
      kind: add_field
      path: $.steps[2].do.idempotency
      value: required
    explanation: The action may be retried after crash recovery, so it needs a durable idempotency key.
```

This lets an AI agent read error → patch YAML → verify again. No guessing.

### explain Command

```text
velvet-ballistics explain workflow.yaml --emit yaml
```

Output includes: what the workflow does, what actions it calls, what secrets it touches, what can fail, what is durable, what is safe to retry, what resource bounds exist.

```yaml
kind: WorkflowExplanation
summary: "Classifies a support ticket, creates a GitHub issue, and sends a Slack notification."
steps:
  - id: classify
    kind: do
    action: ai.classify_ticket
    reads:
      - $input.message
    writes:
      - $classify
    max_calls: 1
    taint:
      input: Clean
      output: Clean
  - id: create_issue
    kind: do
    action: github.issue.create
    idempotency: IdempotentExternal
    strict_durable: true
failure_modes:
  - step: create_issue
    errors:
      - RATE_LIMITED
      - PERMISSION_DENIED
      - TIMEOUT
durability:
  side_effects_journaled_before_dispatch: true
  replay_safe: true
```

### graph Command

```text
velvet-ballistics graph workflow.yaml --emit yaml
```

Emits a graph artifact consumable by AI reasoning, CLI summaries, and documentation generators. One source, many consumers.

```yaml
kind: WorkflowGraph
nodes:
  - step_idx: 0
    id: classify
    kind: do
    action: ai.classify_ticket
    output_slot: 8
    badges:
      strict_safe: true
      secret_sensitive: false
  - step_idx: 1
    id: create_issue
    kind: do
    action: github.issue.create
    output_slot: 15
edges:
  - from: classify
    to: create_issue
    kind: then
  - from: create_issue
    to: done
    kind: then
```

### simulate Command

```text
velvet-ballistics simulate workflow.yaml --input input.vbin --mocks mocks.yaml --emit yaml
```

Runs deterministically with mocked actions. Output:

```yaml
kind: SimulationReport
status: finished
events:
  - seq: 1
    kind: RunAccepted
  - seq: 2
    kind: StepStarted
    step: classify
  - seq: 3
    kind: ActionScheduled
    action: ai.classify_ticket
  - seq: 4
    kind: ActionCompleted
    action: ai.classify_ticket
    source: mock
  - seq: 5
    kind: SlotWritten
    slot: 8
    value_summary:
      type: object
      fields:
        priority: high
result:
  type: object
  fields:
    status: ok
taint:
  public_result_secret_reachable: false
```

This lets AI agents test before running.

### Runtime Commands

**Submit:**

```text
velvet-ballistics submit issue_triage --input-bin input.vbin --emit yaml
```

```yaml
kind: SubmitRunResult
status: accepted
run_id: 123
workflow:
  name: issue_triage
  compiled_digest: blake3:...
durability:
  profile: strict
  run_accepted_durable: true
```

**Inspect:**

```text
velvet-ballistics inspect run_123 --emit yaml
```

```yaml
kind: RunInspection
run_id: 123
status: awaiting_action
current_step:
  idx: 2
  id: create_issue
action_ticket:
  action: github.issue.create
  action_id: 7
  attempt: 1
  idempotency_key_hash: blake3:...
  scheduled_durable: true
  dispatch_state: started
replay:
  safe_to_replay: true
  reason: idempotent_external_action
```

**Events:**

```text
velvet-ballistics events run_123 --tail 20 --emit yaml
```

```yaml
kind: RunEvents
run_id: 123
events:
  - seq: 11
    kind: StepStarted
    step_idx: 2
    timestamp: 1710000000
  - seq: 12
    kind: ActionScheduled
    action_id: 7
    ticket: ...
```

**Replay:**

```text
velvet-ballistics replay run_123 --explain --emit yaml
```

```yaml
kind: ReplayReport
run_id: 123
status: replayed
loaded:
  snapshot_seq: 80
  journal_tail_events: 17
result:
  divergence: false
  reconstructed_pc: 4
  reconstructed_status: awaiting_action
action_recovery:
  pending:
    - action: github.issue.create
      ticket: ...
      policy: retry_with_same_idempotency_key
```

### incident Command

```text
velvet-ballistics incident run_123 --emit yaml
```

Produces the AI-safe black box report:

```yaml
kind: IncidentReport
run_id: 123
status: failed
failure:
  code: ACTION_TIMEOUT
  step: create_issue
  action: github.issue.create
  retryable: true
side_effect_certainty:
  scheduled_durable: true
  completion_durable: false
  external_effect: uncertain
  safe_to_retry: true
  reason: same_idempotency_key
journal_tail:
  - seq: 14
    kind: ActionScheduled
  - seq: 15
    kind: ActionFailed
slot_diffs:
  - slot: 12
    before: null
    after:
      type: object
      redacted: false
taint:
  secret_leak_detected: false
repair_hints:
  - kind: increase_timeout
    path: $.steps[2].do.timeout_ms
    current: 5000
    suggested: 15000
  - kind: add_backoff
    path: $.steps[2].do.retry.backoff_ms
    suggested: 500
```

### Action Discovery

```text
velvet-ballistics action list --emit yaml
```

```yaml
kind: ActionList
actions:
  - name: github.issue.create
    action_id: 7
    idempotency: IdempotentExternal
    strict_safe: true
    input_schema_digest: blake3:...
  - name: ai.classify_ticket
    action_id: 12
    idempotency: DeterministicPure
    strict_safe: true
    input_schema_digest: blake3:...
```

```text
velvet-ballistics action inspect github.issue.create --emit yaml
```

```yaml
kind: ActionDescription
name: github.issue.create
idempotency: IdempotentExternal
strict_safe: true
requires:
  secrets:
    - github_token
input_schema:
  repo:
    type: symbol
    required: true
  title:
    type: symbol
    required: true
output_schema:
  issue_number:
    type: i64
  url:
    type: symbol
failure_codes:
  - RATE_LIMITED
  - PERMISSION_DENIED
  - INVALID_INPUT
examples:
  - name: minimal
    yaml: |
      do:
        action: github.issue.create
        with:
          repo: $input.repo
          title: $input.title
```

### doctor Command

```text
velvet-ballistics doctor --emit yaml
```

Checks: runtime daemon reachable, Fjall DB healthy, action packs loaded, action ABI digest, compiled workflows available, IPC socket permissions, strict durability available, journal writer healthy.

```yaml
kind: DoctorReport
status: pass
checks:
  - name: ipc_socket
    status: pass
  - name: fjall_store
    status: pass
  - name: action_registry
    status: pass
    action_count: 12
  - name: strict_durability
    status: pass
```

### ai context Command

Specifically for AI agents. Emits a compact, redacted packet:

```text
velvet-ballistics ai context run_123 --emit yaml
```

```yaml
kind: AiContextPacket
safe_for_model: true
run:
  id: 123
  status: failed
workflow:
  name: issue_triage
  compiled_digest: blake3:...
failure:
  code: ACTION_TIMEOUT
  step: create_issue
  replay_safe: true
redactions:
  secrets_redacted: 2
  blobs_summarized: 1
suggested_next_commands:
  - velvet-ballistics replay run_123 --explain --emit yaml
  - velvet-ballistics events run_123 --tail 50 --emit yaml
  - velvet-ballistics verify workflow.yaml --profile strict --emit yaml
```

This is a stable AI interface, not a gimmick.

### Output Format Contract

```text
--emit text      # Human-readable (default)
--emit yaml      # AI-structured
--emit postcard  # Machine binary
```

JSON may follow as a cold adapter, but YAML and binary are canonical for v1.

Rules:

1. Every structured output has `schema_version`.
2. Every output has `kind`.
3. Every diagnostic has `code`, `path`, `span`, `message`, `repair`.
4. Secret values are never emitted unless explicit `--unsafe` flag.
5. Large blobs are summarized by digest/type/size.
6. Exit codes are stable and documented.

Stable exit codes:

| Code | Meaning |
|------|---------|
| 0 | success |
| 1 | validation failed |
| 2 | verification failed |
| 3 | compile failed |
| 4 | runtime failed |
| 5 | storage error |
| 6 | IPC error |
| 7 | action policy error |
| 8 | replay divergence |

AI agents can branch on exit codes. No parsing error text.

### Future CLI-UI Parity Rule

No future UI-only truth. If a future UI shows taint graphs, replay timelines, action tickets, queue pressure, certificate status, or incident repair, the CLI must expose it first.

Backend emits typed artifacts:

- `VerificationReport`
- `WorkflowGraph`
- `RunInspection`
- `RunEvents`
- `ReplayReport`
- `IncidentReport`
- `SystemStatus`
- `ActionDescription`

CLI is the current view over those artifacts. Any future UI must consume the same data.

### CLI Build Order

1. `validate --emit yaml`
2. `verify --emit yaml`
3. `compile --emit ir/cert/graph`
4. `simulate --emit yaml`
5. `run`/`submit`
6. `inspect --emit yaml`
7. `events --emit yaml`
8. `replay --explain --emit yaml`
9. `incident --emit yaml`
10. `system status --emit yaml`
11. Future UI consumes the same data after reactivation

Build CLI before any future UI. The UI must not invent concepts — it visualizes proven backend artifacts.

### The Killer Demo

```text
velvet-ballistics verify issue-triage.yaml --profile strict --emit yaml
velvet-ballistics simulate issue-triage.yaml --input example.vbin --mocks mocks.yaml --emit yaml
velvet-ballistics submit issue_triage --input-bin prod.vbin --emit yaml
velvet-ballistics incident run_123 --emit yaml
```

Then hand the output to an AI and ask: *What failed, is it safe to retry, and what should I change?* If the AI can answer correctly from the CLI packet, the design works.

---

---
section: 69
title: "Operator CLI Contract"
parent: velvet-ballistics-MASTER.md
---

## 69. Operator CLI Contract


The CLI is the primary interface for operators and AI agents. It must provide the same operational affordances as mature orchestrators without cargo-culting their branding.

### Canonical Command Surface

```text
velvet-ballistics validate <workflow.yaml>
velvet-ballistics compile  <workflow.yaml> --emit ir --out <file>
velvet-ballistics explain  <workflow.yaml> [--emit yaml|postcard]
velvet-ballistics diff     <workflow.yaml> [--against <old.yaml>] [--emit yaml|postcard]
velvet-ballistics run      <workflow.yaml> --input-bin <file> --durability <mode> [--db <path>]
velvet-ballistics run      <workflow.yaml> --step <step-id> --step-input <file> [--durability <mode>]
velvet-ballistics run-compiled <workflow.vbir> --input-bin <file> --durability <mode> [--db <path>]
velvet-ballistics inspect <run-id> --db <path> [--emit yaml|postcard]
velvet-ballistics events  <run-id> --db <path> [--emit yaml|postcard] [--step <id>] [--tail <n>] [--limit <n>]
velvet-ballistics trace   <run-id> --db <path> [--emit yaml|postcard]
velvet-ballistics replay  <run-id> --db <path> [--emit yaml|postcard]
velvet-ballistics cancel  <run-id> --db <path>
velvet-ballistics resume  <run-id> --db <path>
velvet-ballistics retry   <run-id> --step <step-id> --db <path>
velvet-ballistics answer  <run-id> --slot <slot-id> --value <file> --db <path>
velvet-ballistics ipc-serve --socket <path> --db <path>
velvet-ballistics graph <workflow.yaml> --emit yaml
velvet-ballistics system status --emit yaml
velvet-ballistics action list --emit yaml
velvet-ballistics action inspect <action-name> --emit yaml
velvet-ballistics incident <run-id> --db <path> --emit yaml
velvet-ballistics ai context <run-id> --db <path> --emit yaml
velvet-ballistics bench-run <workflow.yaml>
velvet-ballistics doctor  --db <path> [--emit yaml|postcard]
```

The only supported CLI binary name is `velvet-ballistics`. Short aliases such as
`vb` are not part of the canonical interface and must not be added as Cargo bin
targets.

There is no `ui` command or native Makepad command center in the current contract.

### Single-Step Testing

`run --step <step-id>` executes exactly one step in isolation with explicit input. This is a first-class feature for debugging and validation.

Contract:
- Compile the workflow as normal.
- Resolve `step-id` to `StepIdx` in the compiled IR.
- Construct a minimal `RunFrame` with slots needed for the target step.
- Execute `step_once()` once.
- Report: step ID, step kind, input slots, output slot, engine signal, taint.
- No journal, no persistence, no action dispatch — pure in-memory.
- Exit 0 on success, 1 on step error, 2 on setup error.

### Durable Execution Controls

Strict operational distinction between lifecycle commands:

| Command | What it does | Journal impact |
|---------|-------------|----------------|
| `cancel` | Halt a running/suspended run immediately | Appends `RunCancelled` event |
| `resume` | Resume a suspended run from its current state | Continues journal from last event |
| `retry` | Re-execute a single failed step within an existing run | Preserves journal prefix, appends retry events |
| `replay` | Re-read full journal and verify state (read-only) | No journal mutation |
| `answer` | Answer a pending `Ask` with a slot value | Appends `AskAnswered` event |

`resubmit` (create a brand new run from the same workflow) is `run` with the same workflow — it gets a new `RunId` and fresh journal. It is not a lifecycle command.

### Explain / Dry-Run

`explain <workflow.yaml>` compiles without executing and reports the execution plan:

- Step graph: every step ID, kind, output slot, next step
- Control flow: branches (`Choose`), loops (`ForEach`/`Together`/`Collect`/`Reduce`/`Repeat`), linear chains
- Resource contract: all 16 bounded fields
- Action contracts: which steps are `Do` (side effects)
- Suspension points: which steps can suspend (`Wait`/`Ask`/`Do`)
- Slot layout: total slots, expressions, accessors, constants
- Estimated max step count (budget computation)
- Secrets usage: which steps reference `$secrets`
- Trigger type

`--emit yaml` produces machine-readable structured text. `--emit postcard` produces machine-readable binary output where supported. JSON is not canonical for v1 and must not be hand-formatted into the runtime binary.

### Semantic Diff

`diff <workflow.yaml>` compares a workflow against its previously compiled version:

- Textual diff: YAML source changes (line-level)
- Semantic diff: changes in step count, control flow graph, resource contracts, secret usage, action contracts, retry policies
- Digest comparison: if a compiled artifact exists in the DB, compare BLAKE3 digests
- Exit codes: 0 = no semantic changes, 1 = semantic changes detected, 2 = error
- `--emit yaml` for machine-readable output

### Structured Observability

Output format flags:
- `--emit text` for human-readable output (default)
- `--emit yaml` for structured text output (`inspect`, `explain`, `diff`, `doctor`, `events`, `trace`, `replay`)
- `--emit postcard` for binary machine output where the command returns a typed artifact

Filter flags for `events`:
- `--step <id>` — filter events by step index
- `--tail <n>` — last N events
- `--limit <n>` — maximum events to show
- `--since <date>` — events after timestamp

Logs, events, and trace serve different purposes and must not be merged. Trace includes: resolved inputs, evaluated conditions, expanded loops, chosen branches, retry attempts, emitted outputs.

### CLI Design Rules

- No giant overloaded commands. Each command does one operator job.
- No hidden server-side magic. Local-first, local-only in v1.
- No naming that depends on users knowing another platform.
- Copy the operator affordances, not the branding.
- Machine-readable output (`--emit yaml` and, where applicable, `--emit postcard`) is mandatory for every reporting command. AI agents must be able to parse output without screen-scraping.

### Agent-First CLI Principles

Underlying idea: agents are primary CLI users, not tolerated secondary users. The CLI must reduce token burn, retries, and hidden failure modes by making command shape introspectable, mutation boundaries explicit, and consistency mechanically enforced. Review-only consistency is rejected as Swiss-cheese control; schema, codegen, static checks, or generated context must carry the policy.

The CLI contract must preserve these ten principles:

1. Non-interactive by default. Commands must never wait on an unanswered prompt under non-TTY execution. Any destructive bypass flag is `--force`; `--skip-confirmations` and equivalents are banned.
2. Structured parseable output. Every data-returning/reporting command supports `--emit yaml`; typed artifact commands may additionally support `--emit postcard`. Data goes to stdout, diagnostics go to stderr, and ANSI is suppressed for non-TTY output.
3. Errors that teach and enumerate. Enum validation errors must include the valid set and, where useful, the corrective invocation shape. Parse failures occur before side effects.
4. Safe retries and explicit mutation boundaries. Mutations return stable identifiers, destructive operations require explicit flags, retryable submissions use durable idempotency keys or existing run/job discovery, and consequential commands grow `--dry-run` before release.
5. Bounded responses at every layer. List/event/report commands default to bounded output with `--limit`/cursor/filter narrowing, and MCP/tool/agent descriptions stay under an audited token budget.
6. Cross-CLI vocabulary consistency. CRUD resource verbs are `get`, `list`, `create`, `update`, `delete`; banned aliases include `info`, `ls`, `--format=json`, `--output=json`, and `--skip-confirmations`. Domain-specific verbs require documented justification and static checks.
7. Three-layer introspection. Human `--help`, versioned machine `agent-context`, and long-form skill/workflow guidance must describe the same implementation surface and be validated against it.
8. Async-aware execution. Any async submission gains `--wait` with bounded backoff/jitter and a durable local job ledger exposed through `jobs list`, `jobs get`, and `jobs prune` before async APIs are release-grade.
9. Persistent identity through profiles. Repeated agent invocations use named profiles with precedence `explicit flag > environment variable > profile > default`; available profiles are surfaced in `agent-context`.
10. Two-way I/O. Artifact-producing commands support `--deliver` sinks (`stdout`, `file:<path>`, `webhook:<url>`) with atomic file writes and structured refusal on unknown schemes. `feedback <text>` records local JSONL and optionally posts upstream when configured; availability is exposed in `agent-context`.

Mechanical enforcement required before release:

- `velvet-ballistics agent-context` emits a versioned JSON schema with command names, flags, enums, exit codes, output conventions, and planned agent primitives.
- CI runs `scripts/check-agent-cli-contract.sh` through Moon to reject banned parser vocabulary and require the introspection surface.
- Any generated CLI/schema pipeline must generate the CLI, agent context, skill manifest, and MCP/tool descriptions from one source; hand-written divergence is a release blocker.

---

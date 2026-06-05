# ADR 017 (v1): CLI and AI-Native Control Plane

## Status

Accepted as guardrail. Implementation completion requires evidence.

## Decision

The CLI is the operator and AI-agent control plane. The public demo path is:

```text
verify -> simulate -> submit -> incident/replay
```

The CLI projects typed artifacts and diagnostics. It does not justify adding JSON, HTTP, or formatted text to hot runtime core.

## Invariants

- `verify` is the hero command.
- CLI machine output must preserve stable diagnostics.
- CLI output is a cold projection over runtime truth.
- Future UI parity follows backend/CLI typed artifacts, not private UI state.

## Master Anchors

- Section 33: CLI Commands
- Section 69: Operator CLI Contract
- Section 75: AI-Native CLI Control Plane

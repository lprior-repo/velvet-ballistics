---
section: 70
title: "Phase Extension: Operator Features"
parent: velvet-ballistics-MASTER.md
---

## 70. Phase Extension: Operator Features


The following phases extend Section 35 for operator-facing features:

| Phase | Name | Required delivery |
|-------|------|-------------------|
| 50 | Single-step testing | `run --step <id>` with input payload, isolated execution, step result reporting. Tests: step resolution, minimal frame construction, step_once execution, output reporting. |
| 51 | Explain / dry-run | `explain` command with step graph, resource contract, suspension points, secrets usage, `--emit yaml` output. Tests: explain output matches compiled IR, YAML format validation. |
| 52 | Durable lifecycle controls | `cancel`, `resume`, `retry`, `answer` CLI commands. Strict distinction between retry-step, replay-run, and resubmit-workflow. Tests: each lifecycle command against journaled runs, cancelled runs, suspended runs. |
| 53 | Semantic diff | `diff` command with textual + semantic diff, digest comparison, exit codes. Tests: diff detects step changes, resource contract changes, secret changes. |
| 54 | Structured observability | `--emit yaml`/`--emit postcard` flags, filter flags (`--step`, `--tail`, `--limit`, `--since`). Tests: structured output parses correctly, filter flags narrow results. |
| 55 | Timer wheel | Replace `IndexMap<RunId, PendingTimer>` with `TimerWheel` backed by `BTreeMap<Instant, Vec<TimerEntry>>`. Automatic timer-driven resume in shard tick. Tests: timer firing, cancellation, next-deadline accuracy. |
| 56 | Collect hardening | Per-run pagination state (replace global Mutex), time-based pagination limit, `RunId`-keyed state. Tests: concurrent collect runs, time limit enforcement, crash-recovery of pagination state. |
| 57 | Recovery evidence chain | `SlotWritten` + `StepSucceeded` per deterministic step, `UnsupportedRecoveryState` hydration gate, fix stubbed `verify_digests` at `Full` level. Tests: crash recovery with full evidence chain, hydration failure on missing state. |
| 58 | Codegen residue removal | Delete or quarantine codegen stubs, tests, proof residue, and generated-mode references. |
| 59 | Behavioral property tests | Current-scope properties from Section 38: constant folding parity, bytecode/AST parity, digest stability, layout stability, replay determinism, snapshot equivalence, ordering invariants, bound enforcement, state machine, and taint safety. |
| 60 | Canonical CLI binary | Cargo.toml exposes only the canonical `velvet-ballistics` binary. Short aliases such as `vb` are rejected to preserve the naming contract. |
| 61-74 | UI residue removal | Delete or quarantine UI/Makepad/Figma/snapshot/perf-gate residue. |

---

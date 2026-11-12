# QA Report: vb-qi37.3

- phase: State 9 - QA rerun after black-hat repair
- timestamp_utc: 2026-05-11T07:21:23Z
- STATUS: PASS

## Startup / Required Context

- Read `/home/lewis/.claude/skills/qa-enforcer/SKILL.md`: execution is mandatory (`lines 12, 16, 86-90`), findings require exact command/output/exit evidence (`lines 13, 120-126`), and deep inspection is required (`lines 14, 128-144`).
- Read `/home/lewis/.agents/skills/qa-enforcer/SKILL.md`: same content observed; agents copy would win on conflicts.
- Read required bead artifacts before executing QA: `STATE.md`, `test-plan.md`, `test-suite-review.md`, `black-hat-review.md`, `defects.md`, `test-repair-blackhat.md`, `implementation.md`, `moon-report.md`, and `regression-diff.md`.
- Scope decision from State 8: global FORMAT/CLIPPY/`vb_ui_model` feature-powerset debt remains `DEFERRED_GLOBAL` under `vb-bkgo`; this QA rejects only bead-local regressions.

## Commands Run / Evidence

### 1. Product CLI help smoke

Command:

```bash
rustup run nightly-2026-04-28 cargo run -p velvet_ballistics --bin velvet-ballistics -- --help
```

Exit status: `0`

Observed output summary:

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.04s
Running `target/debug/velvet-ballistics --help`
velvet-ballistics - compiled workflow runtime
commands:
  validate   <workflow.yaml> [--json|--jsonl]          Validate a workflow definition
  verify     <workflow.yaml> [--profile <quick|standard|full>] [--json|--jsonl]  Verify a workflow
  explain    <workflow.yaml> [--json|--jsonl]          Explain validation errors in detail
  compile    <workflow.yaml> --emit <ir|rust|yaml|postcard> --out <file> [--json|--jsonl]  Compile a workflow
  run        <workflow.yaml> --input-bin <file> --durability <mode> [--db <path>] [--json|--jsonl]
  run-compiled <workflow.vbir> --input-bin <file> --durability <mode> [--db <path>] [--json|--jsonl]
  ipc-serve  --socket <path> --db <path>               Start IPC server
  inspect    <run_id> --db <path> [--json|--jsonl]     Inspect a run
  events     <run_id> --db <path> [--json|--jsonl]     List run events
  replay     <run_id> --db <path> [--json|--jsonl]     Replay a run from journal
  trace      <run_id> --db <path> [--json|--jsonl]     Show step-by-step execution trace
  retry      <run_id> --db <path> [--json|--jsonl]     Retry a failed run from last successful step
  resume     <run_id> --db <path> [--json|--jsonl]     Resume a suspended run
  bench-run  <workflow.yaml> [--json|--jsonl]          Benchmark a workflow
  doctor     --db <path> [--json|--jsonl]              Run diagnostic checks
  answer     <run_id> --step <N> --value-file <file> --db <path> [--json|--jsonl]  Answer a suspended step
  graph      <workflow.yaml> [--json|--jsonl]          Output control flow graph in DOT format
  diff       <run_a> <run_b> --db <path> [--json|--jsonl]  Compare two runs
  incident   <run_id> --db <path> [--json|--jsonl]     Black-box failure report
  submit     <workflow.yaml> --input-bin <file> --db <path> --durability <mode> [--json|--jsonl]  Submit workflow run
  simulate   <workflow.yaml> [--json|--jsonl]     Dry-run workflow without executing actions
  ai-context <run_id> --db <path> [--json|--jsonl]  Emit compact AI context packet for a run
  help                                                Print this message
  version                                             Print version
  agent-context                                      Emit versioned AI-agent CLI schema
  status     [--active-runs <N>] [--queue-depth <N>] [--trace-dropped <N>] [--json|--jsonl]  Report runtime shard status
  action list [--json|--jsonl]                       List registered action contracts
  action inspect <action_id> [--json|--jsonl]         Show one registered action contract
options:
  --json      Output structured JSON
  --jsonl     Output structured JSON Lines (one object per line)
architecture: nightly Rust, compiled IR, in-memory engine, bounded IPC, Fjall journal, no HTTP hot path
```

QA inspection: Help command is user-visible, exits successfully, produces non-empty help, and shows no panic/todo/unimplemented/error text. No collect-specific CLI/API route is exposed; collect behavior is therefore validated through runtime nextest execution.

### 2. Focused black-hat repair tests

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_next_immediate_duplicate_page_with_intervening_allocations_returns_duplicate_and_preserves_state) | test(collect_slot_extra_capacity_one_returns_capacity_error_and_preserves_existing_evidence) | test(collect_hydration_corrupt_slot_value_with_collect_extra_returns_decode_failed_and_no_state)'
```

Exit status: `0`

Observed output summary:

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.45s
Nextest run ID 43f3e44e-7d76-4a2d-9e92-eb73d5c6d383 with nextest profile: default
Starting 3 tests across 2 binaries (1356 tests skipped)
Summary [   0.021s] 3 tests run: 3 passed, 1356 skipped
```

QA inspection: The three black-hat repair regressions all pass: semantic duplicate with intervening allocations, fail-closed capacity-one evidence preservation, and corrupt collect-bearing hydration fail-closed behavior.

### 3. Broad `collect_next_` filter

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_next_
```

Exit status: `0`

Observed output summary:

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 1.04s
Nextest run ID 723938fe-c5b9-4c82-ab92-15ad2f9e6124 with nextest profile: default
Starting 19 tests across 2 binaries (1340 tests skipped)
Summary [   0.017s] 19 tests run: 19 passed, 1340 skipped
```

QA inspection: Pagination next-page behavior, including old and repaired page-order paths, passes as a broad filter.

### 4. Hydration/capacity focused filter

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_hydration_) | test(collect_slot_extra_capacity)'
```

Exit status: `0`

Observed output summary:

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.63s
Nextest run ID 856f1f30-7e2a-4c2a-8e5d-cf63519cfd97 with nextest profile: default
Starting 7 tests across 2 binaries (1352 tests skipped)
Summary [   0.024s] 7 tests run: 7 passed, 1352 skipped
```

QA inspection: Hydration schema/fail-closed tests and collect evidence-capacity boundary tests pass together.

### 5. Broad `vb_runtime collect_` filter

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_
```

Exit status: `0`

Observed output summary:

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.80s
Nextest run ID 4601f62a-ecbf-4362-951a-b6d2e1319f07 with nextest profile: default
Starting 102 tests across 2 binaries (1257 tests skipped)
Summary [   0.225s] 102 tests run: 102 passed, 1257 skipped
```

QA inspection: The full collect-selected runtime suite passes after black-hat repair. This covers broader collect pagination, hydration, recovery, and evidence behavior beyond the exact black-hat regressions.

## Findings

- Critical: none.
- Major: none.
- Minor: none.
- Observations: Product CLI exposes no collect-specific route, so bead-local collect behavior is validated through runtime/library nextest execution. This matches prior State 7 caveat and is not a bead-local defect for this runtime repair.

## Decision

QA found no bead-local defects after the black-hat repair.

State 9 result: `PASS`. Required State 9 commands all exited `0`, with focused black-hat repair tests `3/3` passed, broad `collect_next_` `19/19` passed, hydration/capacity `7/7` passed, and broad `collect_` `102/102` passed. Known global `moon ci --stdin` FORMAT/CLIPPY/feature-powerset debts remain `DEFERRED_GLOBAL` under `vb-bkgo` and are not rejected here.

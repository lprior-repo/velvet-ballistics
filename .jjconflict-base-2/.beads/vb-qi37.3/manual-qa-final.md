# Final Manual QA Report: vb-qi37.3

STATUS: PASS

## Scope / startup citations

- Workspace: `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go`
- Bead: `vb-qi37.3` — runtime: Prove collect pagination durability and hydration
- Phase: State 14 final manual QA after architectural drift approval.
- Read `/home/lewis/.claude/skills/hands-on-qa/SKILL.md`: lines 22-29 require real invocations, discovery first, captured stdout/stderr/exit code, and test-only/no-fix behavior; lines 77-80 require exact command and numeric exit code for every result.
- Read `/home/lewis/.agents/skills/hands-on-qa/SKILL.md`: same content observed; agents copy wins on conflict, no conflict found.
- Read required bead artifacts before execution: `STATE.md`, `architectural-drift-review.md`, `formal-verification-report.md`, `black-hat-review.md`, `red-queen-report.md`, `test-suite-review.md`, `qa-report.md`, `moon-report.md`, and `regression-diff.md`.
- Context accepted from artifacts: State 13 made no source/test edits and approved direct advance; State 8 global FORMAT/CLIPPY/`vb_ui_model` debt remains `DEFERRED_GLOBAL` under `vb-bkgo`; State 9/10/11/12/13 are approved after black-hat repair.

## Commands run / evidence

### 1. Product CLI surface discovery

Command:

```bash
rustup run nightly-2026-04-28 cargo run -p velvet_ballistics --bin velvet-ballistics -- --help
```

Exit code: `0`

Captured output summary:

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.11s
Running `target/debug/velvet-ballistics --help`
velvet-ballistics - compiled workflow runtime
commands:
  validate   <workflow.yaml> [--json|--jsonl]          Validate a workflow definition
  verify     <workflow.yaml> [--profile <quick|standard|full>] [--json|--jsonl]  Verify a workflow
  explain    <workflow.yaml> [--json|--jsonl]          Explain validation errors in detail
  compile    <workflow.yaml> --emit <ir|rust|yaml|postcard> --out <file> [--json|--jsonl]  Compile a workflow
  run        <workflow.yaml> --input-bin <file> --durability <mode> [--db <path>] [--json|--jsonl]
             [--step <id> --step-input <file>]                                 Run a single step in isolation
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
EXIT_CODE=0
```

Inspection: CLI starts and help is non-empty. No collect-specific product CLI route is exposed; bead behavior is validated through runtime nextest filters.

### 2. Safe product CLI version command

Command:

```bash
rustup run nightly-2026-04-28 cargo run -p velvet_ballistics --bin velvet-ballistics -- version
```

Exit code: `0`

Captured output summary:

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.28s
Running `target/debug/velvet-ballistics version`
velvet-ballistics 0.1.0
EXIT_CODE=0
```

Inspection: safe no-db/no-side-effect version command works.

### 3. Safe product CLI status command

Command:

```bash
rustup run nightly-2026-04-28 cargo run -p velvet_ballistics --bin velvet-ballistics -- status --json
```

Exit code: `0`

Captured output summary:

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.20s
Running `target/debug/velvet-ballistics status --json`
{
  "RuntimePolicy": "Strict",
  "active_runs": { "active": 0, "max_active_runs": 1024 },
  "command_queue": { "capacity": 1024, "depth": 0 },
  "running": true,
  "shutting_down": false,
  "status": "running",
  "step_budget_per_tick": 1000,
  "trace_ring": { "capacity": 4096, "dropped": 0 }
}
EXIT_CODE=0
```

Inspection: safe status JSON command works and reports a healthy empty runtime shard.

### 4. Focused black-hat repair regression tests

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_next_immediate_duplicate_page_with_intervening_allocations_returns_duplicate_and_preserves_state) | test(collect_slot_extra_capacity_one_returns_capacity_error_and_preserves_existing_evidence) | test(collect_hydration_corrupt_slot_value_with_collect_extra_returns_decode_failed_and_no_state)'
```

Exit code: `0`

Captured output summary:

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.28s
Nextest run ID e0e6628f-b557-46af-ac08-0a9579aa38c3 with nextest profile: default
Starting 3 tests across 2 binaries (1356 tests skipped)
Summary [   0.024s] 3 tests run: 3 passed, 1356 skipped
EXIT_CODE=0
```

Inspection: final focused repair filter passes all three exact black-hat repair tests.

### 5. Red Queen page-lineage challengers

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_next_duplicate_page_returns_order_violation_duplicate_and_preserves_state) | test(collect_next_immediate_duplicate_page_with_intervening_allocations_returns_duplicate_and_preserves_state) | test(collect_next_stale_page_returns_order_violation_stale_and_preserves_state) | test(collect_next_future_page_returns_order_violation_out_of_order_and_preserves_state)'
```

Exit code: `0`

Captured output summary:

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.16s
Nextest run ID 9d072521-6931-47b8-a07d-f0dcd6ac42a6 with nextest profile: default
Starting 4 tests across 2 binaries (1355 tests skipped)
Summary [   0.031s] 4 tests run: 4 passed, 1355 skipped
EXIT_CODE=0
```

Inspection: duplicate/stale/out-of-order semantic lineage challengers pass.

### 6. Red Queen evidence-capacity challengers

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_slot_extra_capacity_zero_returns_capacity_error_before_success) | test(collect_slot_extra_capacity_one_preserves_required_slot_written_extra) | test(collect_slot_extra_capacity_one_returns_capacity_error_and_preserves_existing_evidence) | test(collect_slot_extra_capacity_full_returns_capacity_error_not_silent_drop)'
```

Exit code: `0`

Captured output summary:

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.22s
Nextest run ID 69aad9fb-9dfa-463b-8ec9-8416a40fc415 with nextest profile: default
Starting 3 tests across 2 binaries (1356 tests skipped)
Summary [   0.026s] 3 tests run: 3 passed, 1356 skipped
EXIT_CODE=0
```

Inspection: capacity fail-closed challengers pass; nextest selected three concrete matching tests from the expression.

### 7. Red Queen hydration challengers

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_hydration_corrupt_slot_value_with_collect_extra_returns_decode_failed_and_no_state) | test(collect_hydration_current_page_mismatch_returns_page_mismatch_and_no_state) | test(collect_hydration_corrupt_extra_returns_decode_failed_and_no_state) | test(recovered_collect_state_rejects_run_mismatch_and_inserts_no_state) | test(recovered_collect_state_rejects_slot_mismatch_and_inserts_no_state)'
```

Exit code: `0`

Captured output summary:

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.27s
Nextest run ID ab2764a2-e3f1-4d0b-b6ca-b57eae3573fc with nextest profile: default
Starting 5 tests across 2 binaries (1354 tests skipped)
Summary [   0.021s] 5 tests run: 5 passed, 1354 skipped
EXIT_CODE=0
```

Inspection: corrupt/current-page/run/slot hydration challengers pass.

### 8. Broad `vb_runtime collect_` regression smoke

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_
```

Exit code: `0`

Captured output summary:

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.14s
Nextest run ID 428f7f0e-73f9-4faf-b605-b07eec63b332 with nextest profile: default
Starting 102 tests across 2 binaries (1257 tests skipped)
Summary [   0.191s] 102 tests run: 102 passed, 1257 skipped
EXIT_CODE=0
```

Inspection: broad collect-selected runtime suite passes after architectural drift approval.

## Findings

- CRITICAL: none.
- MAJOR: none.
- MINOR: none.
- OBSERVATION: Product CLI exposes no collect-specific CLI/API route in `--help`; final bead behavior is therefore manually smoked through real runtime/library nextest invocations. This is consistent with prior State 7 and State 9 QA and is not a bead-local defect.
- OBSERVATION: An initial shell evidence wrapper used zsh variable name `status`, which is read-only in zsh; the product help printed successfully but the wrapper failed. The help command was immediately rerun with variable `ec` and exited `0`; only the rerun is used as gate evidence.

## Decision

Final manual QA can pass.

All final smoke commands required for State 14 exited `0`: product help, product version, product status JSON, focused black-hat repair tests `3/3`, Red Queen lineage `4/4`, Red Queen capacity `3/3`, Red Queen hydration `5/5`, and broad `vb_runtime collect_` `102/102`. No bead-local blocker was found. Known global FORMAT/CLIPPY/`vb_ui_model` debt remains `DEFERRED_GLOBAL` under `vb-bkgo` and is not a State 14 manual QA failure.

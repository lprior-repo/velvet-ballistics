# Manual QA Smoke: vb-qi37.3 post-black-hat repair

STATUS: PASS

## Startup doctrine

- Read `/home/lewis/.claude/skills/hands-on-qa/SKILL.md`: lines 22-28 require real invocations, captured stdout/stderr/exit code, discover-first, no assumptions, and fail-fast ordering.
- Read `/home/lewis/.agents/skills/hands-on-qa/SKILL.md`: same content observed; no conflict. Per instruction, agents copy would win if a conflict existed.

## Inputs read before smoke

- `.beads/vb-qi37.3/STATE.md`
- `.beads/vb-qi37.3/implementation.md`
- `.beads/vb-qi37.3/test-repair-blackhat.md`
- `.beads/vb-qi37.3/defects.md`
- `.beads/vb-qi37.3/regression-diff.md`

## Target and scope

- Workspace: `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go`
- Bead: `vb-qi37.3`
- Product CLI smoke: `velvet-ballastics --help` through `cargo run`.
- Runtime behavior smoke: actual `cargo nextest` invocations for black-hat repaired collect behavior.
- Source/tests were not modified by this QA pass. This artifact was overwritten as requested.

## Command evidence

### 1. Product CLI help smoke

Command:

```bash
rustup run nightly-2026-04-28 cargo run -p velvet_ballastics --bin velvet-ballastics -- --help
```

Exit code: 0

Output:

```text
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?rev=20b6c53b6f229b165fe7f813504ae93405159d27#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

   Compiling vb_runtime v0.1.0 (/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_runtime)
   Compiling vb_ipc v0.1.0 (/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_ipc)
   Compiling velvet_ballastics v0.1.0 (/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/velvet_ballastics)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.87s
     Running `target/debug/velvet-ballastics --help`
velvet-ballastics - compiled workflow runtime

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

Observation: no collect-specific user CLI/API path is advertised in help; collect behavior is runtime/library-level and was smoked through the real nextest runtime tests below.

### 2. Focused black-hat repair filter: three repaired defects

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_next_immediate_duplicate_page_with_intervening_allocations_returns_duplicate_and_preserves_state) | test(collect_slot_extra_capacity_one_returns_capacity_error_and_preserves_existing_evidence) | test(collect_hydration_corrupt_slot_value_with_collect_extra_returns_decode_failed_and_no_state)'
```

Exit code: 0

Output:

```text
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?rev=20b6c53b6f229b165fe7f813504ae93405159d27#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?rev=20b6c53b6f229b165fe7f813504ae93405159d27#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.16s
────────────
 Nextest run ID 0a208320-c0f7-4f98-a7bf-7b0510dd4f2e with nextest profile: default
    Starting 3 tests across 2 binaries (1356 tests skipped)
────────────
     Summary [   0.009s] 3 tests run: 3 passed, 1356 skipped

EXIT_CODE=0
```

### 3. Relevant collect page progression filter

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_next_)'
```

Exit code: 0

Output:

```text
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?rev=20b6c53b6f229b165fe7f813504ae93405159d27#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?rev=20b6c53b6f229b165fe7f813504ae93405159d27#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s
────────────
 Nextest run ID fd8d6795-8868-4fce-843a-9f32e9402fe7 with nextest profile: default
    Starting 19 tests across 2 binaries (1340 tests skipped)
────────────
     Summary [   0.042s] 19 tests run: 19 passed, 1340 skipped

EXIT_CODE=0
```

### 4. Relevant hydration and evidence-capacity filter

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_hydration_) | test(collect_slot_extra_capacity_)'
```

Exit code: 0

Output:

```text
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?rev=20b6c53b6f229b165fe7f813504ae93405159d27#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?rev=20b6c53b6f229b165fe7f813504ae93405159d27#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.11s
────────────
 Nextest run ID 0d474016-37d4-4d92-8ed1-830b09860001 with nextest profile: default
    Starting 7 tests across 2 binaries (1352 tests skipped)
────────────
     Summary [   0.037s] 7 tests run: 7 passed, 1352 skipped

EXIT_CODE=0
```

### 5. Broad vb_runtime collect_ smoke

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_
```

Exit code: 0

Output:

```text
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?rev=20b6c53b6f229b165fe7f813504ae93405159d27#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?rev=20b6c53b6f229b165fe7f813504ae93405159d27#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s
────────────
 Nextest run ID 4529867d-43a2-4624-93e0-85ee380e0441 with nextest profile: default
    Starting 102 tests across 2 binaries (1257 tests skipped)
────────────
     Summary [   0.171s] 102 tests run: 102 passed, 1257 skipped

EXIT_CODE=0
```

## Result

- Product CLI help smoke: PASS.
- Focused black-hat repair filter: PASS, 3/3.
- Additional collect page progression filter: PASS, 19/19.
- Additional hydration/capacity filter: PASS, 7/7.
- Broad `vb_runtime collect_`: PASS, 102/102.
- No hostile-path regression was observed in the repaired runtime behavior filters.

## Residual risk

- No collect-specific end-user CLI/API route is advertised by product help, so collect durability/hydration behavior was smoked through the real runtime test harness rather than a CLI collect command.
- Global FORMAT/CLIPPY/`vb_ui_model` debts remain documented as `DEFERRED_GLOBAL` in `regression-diff.md`; they were not part of this manual smoke rerun.

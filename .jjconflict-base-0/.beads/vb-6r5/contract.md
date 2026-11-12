bead_id: vb-6r5
bead_title: Add max-speed xtask proof/test orchestrator
phase: 3
updated_at: 2026-05-18T01:45:00Z
attempt: 1-of-7

# Contract Spec - State 3

## Requirements

### R1: CLI Commands
The xtask binary MUST support these commands:
- `cargo xtask list-crates` — Discover and list workspace crates
- `cargo xtask proof list` — List available proof/test lanes per crate
- `cargo xtask proof run --profile fast|standard|deep|proof|all --jobs auto|N` — Run all lanes
- `cargo xtask proof crate <crate> --lanes <lane-list> --jobs auto|N` — Run lanes for specific crate
- `cargo xtask proof affected --base <rev> --jobs auto|N` — Run lanes for crates changed since rev

### R2: Profile-Based Lane Selection
- `fast`: cargo test, cargo clippy (minimum viable)
- `standard`: fast + nextest (if available)
- `deep`: standard + kani, miri, loom (if available)
- `proof`: verus, tla, flux (if available)
- `all`: all available lanes

### R3: DAG Scheduler
- Discover crate dependencies via cargo metadata
- Schedule independent crates in parallel
- Respect dependency order (dependent crates run after dependencies)
- Bounded parallelism via --jobs (auto = num_cpus)
- Per-lane timeout (default 300s, configurable via --timeout)
- Fail-fast mode (--fail-fast) stops on first failure
- Keep-going mode (--keep-going) continues on failure

### R4: Structured Logging
- Write per-crate/per-lane logs to `target/xtask-proof/<run-id>/<crate>/<lane>.jsonl`
- Each line: `{"crate","lane","command","exit_code","duration_ms","stdout","stderr","timestamp"}`
- Human-readable summary at end of run
- JSON output mode (--json) for machine consumption

### R5: Workspace Discovery
- Use `cargo metadata --no-deps --format-version 1` exactly once
- Cache result for duration of run
- Exclude fuzz crate and test-only crates by default
- Support --include and --exclude glob patterns

### R6: CLI Flags
- `--exclude <pattern>` — Exclude crates matching pattern
- `--include <pattern>` — Include only crates matching pattern
- `--fail-fast` — Stop on first lane failure
- `--keep-going` — Continue on lane failure (default)
- `--timeout <seconds>` — Per-lane timeout (default 300)
- `--dry-run` — Print commands without executing
- `--json` — Machine-readable output
- `--jobs auto|N` — Parallel job count

### R7: Exit Code
- Exit 0 if all required lanes pass
- Exit non-zero if any required lane fails
- Exit 2 for CLI/usage errors

## Assumptions
- A1: cargo is available on PATH
- A2: cargo metadata output is valid JSON
- A3: Tool availability (kani, miri, nextest, etc.) is detected at runtime
- A4: Unavailable tools are skipped with warning, not error
- A5: proof-obligations.jsonl may not exist for all crates

## Invariants
- I1: cargo metadata is called exactly once per run
- I2: No lane runs twice in the same execution
- I3: Log directory is unique per run (timestamp-based run-id)
- I4: All functions are under 25 lines with max 5 parameters
- I5: No unsafe, unwrap, expect, panic, todo, unimplemented, dbg

## Type/Domain Model

### CrateInfo
- name: String
- path: PathBuf
- dependencies: Vec<String> (crate names this crate depends on)
- has_kani: bool
- has_miri: bool
- has_loom: bool
- has_fuzz: bool
- proof_obligations: Vec<ProofObligationCommand>

### Lane
- name: String (e.g., "test", "clippy", "kani", "miri")
- command: Vec<String> (command + args)
- timeout: Duration
- required: bool (fail-fast applies only to required lanes)
- profile: Vec<Profile> (which profiles include this lane)

### RunResult
- run_id: String
- started_at: DateTime
- completed_at: DateTime
- crate_results: Vec<CrateLaneResult>
- summary: RunSummary

### CrateLaneResult
- crate_name: String
- lane_name: String
- command: String
- exit_code: Option<i32>
- duration_ms: u64
- status: LaneStatus (Pass, Fail, Timeout, Skipped, DryRun)

## Verification Layers
- Unit tests: CLI parsing, DAG scheduling, lane selection
- Integration tests: End-to-end command execution (mocked)
- Property tests: DAG topological sort correctness

## Traceability
R1 -> cli.rs, main.rs
R2 -> profiles.rs
R3 -> scheduler.rs
R4 -> logger.rs, summary.rs
R5 -> discovery.rs
R6 -> cli.rs
R7 -> main.rs

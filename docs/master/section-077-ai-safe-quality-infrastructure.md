---
section: 77
title: "AI-Safe Quality Infrastructure"
parent: velvet-ballistics-MASTER.md
---

## 77. AI-Safe Quality Infrastructure


AI changes must be small, checkable, replayable, benchmarked, and hard to merge when wrong. The closed loop is:

```
spec -> task -> patch -> mechanical checks -> evidence -> benchmark -> certificate -> merge
```

AI agents must not guess which checks to run. Every quality gate is exposed as a first-party `xtask` command that returns structured machine-readable output. No evidence bundle means no merge.

### 77.1 xtask Command Center

A first-party `xtask` crate provides the AI-safe command interface for development. AI agents invoke `cargo xtask <command>` and receive structured YAML/JSON output; they never guess which checks apply.

Required commands:

| Command | Purpose |
|---------|---------|
| `cargo xtask ai-context --crate <crate> --topic <topic>` | Emit relevant files, contracts, required tests, and fast commands for a focused working set |
| `cargo xtask ai-plan --bead <id>` | Validate that a plan covers the bead scope and references correct invariants |
| `cargo xtask ai-check --scope <crate>` | Run fmt, clippy, nextest, forbidden-scan, hotpath-scan for a single crate; stop at first failure |
| `cargo xtask ai-evidence --bead <id>` | Generate or validate the evidence bundle for a bead |
| `cargo xtask invariants` | Run all invariant checks from `contracts/invariants.yaml` |
| `cargo xtask hotpath-scan [--changed]` | Scan for allocation, formatting, or unbounded patterns in hot-path code |
| `cargo xtask forbidden-scan [--changed]` | Scan for forbidden tokens, macros, patterns, imports |
| `cargo xtask cert-check` | Validate verification certificates for compiled workflows |
| `cargo xtask perf-compare --against main` | Benchmark comparison against a baseline |
| `cargo xtask perf-report --emit yaml` | Emit structured performance report |
| `cargo xtask perf-baseline save` | Save current performance baseline |
| `cargo xtask replay-lab` | Run differential replay tests |
| `cargo xtask crash-lab --workflow <name> [--crash-at <point> \| --all-crash-points]` | Deterministic fault-injection harness |
| `cargo xtask diff-test --suite <name>` | Run differential test suite |
| `cargo xtask alloc-check --suite hotpath` | Verify allocation behavior for hot paths |
| `cargo xtask api-diff` | Public API diff against baseline |
| `cargo xtask review --changed --emit yaml` | Structured patch review report |
| `cargo xtask why-failed <log>` | Explain a failure in human/AI-readable form |
| `cargo xtask mutants --scope touched` | Mutation testing for changed code |
| `cargo xtask loom --model <name>` | Run Loom concurrency model test |
| `cargo xtask kani --harness <name>` | Run Kani proof harness |
| `cargo xtask fuzz-target new <name>` | Create a new fuzz target |
| `cargo xtask prop-test new <name>` | Create a new proptest harness |
| `cargo xtask repro shrink --failure <log>` | Shrink a failure to minimal repro |
| `cargo xtask repro run <repro-file>` | Replay a minimal repro |
| `cargo xtask test-plan --phase <n>` | List required tests for a phase |
| `cargo xtask test-plan --missing` | List required tests not yet implemented |

All output is structured (YAML by default). Example `ai-check` output:

```yaml
kind: AiCheckReport
scope: vb_core
status: fail
commands:
  - name: fmt
    status: pass
  - name: clippy
    status: fail
    diagnostics:
      - file: crates/vb_core/src/frame.rs
        code: clippy::arithmetic_side_effects
        line: 88
  - name: nextest
    status: not_run
    reason: clippy_failed
recommended_next_action:
  kind: fix_diagnostic
  file: crates/vb_core/src/frame.rs
```

### 77.2 Three Check Levels

AI needs fast feedback first, then deep proof later. Three levels provide a ladder instead of one impossible command.

#### ai-fast (run constantly while coding)

```bash
cargo +nightly fmt --all -- --check
cargo +nightly check --workspace --all-targets
cargo +nightly clippy -p <touched-crate> --lib --all-features -- -D warnings
cargo +nightly nextest run -p <touched-crate>
cargo xtask forbidden-scan --changed
cargo xtask hotpath-scan --changed
```

#### ai-deep (run before closing a bead)

```bash
cargo +nightly nextest run --workspace --all-features
cargo +nightly test --doc --workspace --all-features
cargo +nightly miri test -p vb_core -p vb_expr -p vb_compile
cargo mutants --package <touched-crate> --timeout 60
cargo llvm-cov --workspace --all-features
cargo fuzz build
```

#### ai-release (run before release)

```bash
moon ci
```

Supply-chain/advisory reports are non-blocking under the 2026-05-23 owner waiver unless a future bead explicitly opts in.

The maxperf lane is removed and is not part of current release closure.

### 77.3 Evidence Bundles

Every AI-authored change produces an evidence bundle at `.evidence/<bead-id>/evidence.yaml`. No evidence bundle means no merge. This extends section 60 (Evidence Artifact Format) with AI-specific fields.

```yaml
kind: AiImplementationEvidence
bead: runtime-engine-setconst
phase: 13
git_commit: abc123
model_notes:
  summary: "Implemented SetConst typed error behavior."
files_changed:
  - crates/vb_core/src/engine.rs
  - crates/vb_core/tests/engine.rs
public_api_changed: false
hot_path_changed: true
commands:
  - command: cargo +nightly fmt --all -- --check
    exit_code: 0
    log: logs/fmt.txt
  - command: cargo +nightly clippy -p vb_core --lib --all-features -- -D warnings
    exit_code: 0
    log: logs/clippy-vb-core.txt
  - command: cargo +nightly nextest run -p vb_core
    exit_code: 0
    log: logs/nextest-vb-core.txt
tests_added:
  - missing_output_slot_is_typed_error
  - const_out_of_bounds_is_typed_error
benchmarks:
  required: false
remaining_risk:
  - "Copy primitive not implemented in this bead."
```

### 77.4 Machine-Readable Invariants

Invariants live in `contracts/invariants.yaml` as executable rules. `cargo xtask invariants` outputs exactly which invariant failed.

```yaml
invariants:
  - id: no_runtime_yaml
    applies_to:
      - crates/vb_core/**
      - crates/vb_runtime/**
      - crates/vb_storage/**
      - crates/vb_ipc/**
      - generated/**
    forbidden:
      - saphyr_parser
      - parse_workflow_source
      - yaml

  - id: no_hot_path_formatting
    applies_to:
      - crates/vb_core/src/engine.rs
      - crates/vb_runtime/src/shard/**
      - generated/**
    forbidden_macros:
      - format
      - println
      - eprintln
      - dbg

  - id: no_unchecked_indexing
    applies_to:
      - crates/**
    forbidden_patterns:
      - indexing
      - slicing
```

### 77.5 Semantic Banned Scans

Token-level grep is necessary but insufficient. The quality infrastructure uses multiple scan layers:

| Layer | Tool | Checks |
|-------|------|--------|
| Token scan | ripgrep | `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, forbidden imports |
| Clippy denies | `clippy` | `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`, `clippy::arithmetic_side_effects`, etc. |
| Dependency unsafe advisory | `cargo geiger` | Transitive unsafe in dependencies; non-blocking under owner waiver |
| AST scanner | syn-based custom (`xtask forbidden-scan`) | Unchecked indexing, slicing, `as` casts, ignored `Result`, `HashMap<String, _>` in runtime, `serde_json` in runtime, HTTP crates in runtime |
| Public API diff advisory | `cargo public-api` | Accidental public contract changes; non-blocking unless an API-stability bead opts in |
| Allocation scanner | `xtask hotpath-scan` | `format!`, `println!`, `Vec::push` without pre-reserve, `String` construction in hot paths |

AI often satisfies the literal rule while violating the intent. Multi-layer scanning catches this.

### 77.6 AI Context Packets

AI must not read the whole repo. `cargo xtask ai-context --crate vb_core --topic engine` emits a precise working set:

```yaml
kind: AiCodeContext
crate: vb_core
topic: engine
relevant_files:
  - crates/vb_core/src/engine.rs
  - crates/vb_core/src/frame.rs
  - crates/vb_core/src/compiled.rs
  - crates/vb_core/src/errors.rs
contracts:
  - "No unsupported primitive may silently continue."
  - "StepBudget(0) executes zero transitions."
  - "SetConst has no Null fallback."
required_tests:
  - missing_output_slot_is_typed_error
  - const_out_of_bounds_is_typed_error
commands:
  fast:
    - cargo +nightly nextest run -p vb_core engine::
```

### 77.7 Spec-to-Test Mapping

Required tests live in `contracts/tests.yaml` as executable metadata. This makes the master document's mandatory test coverage (section 36) queryable.

```yaml
tests:
  - name: const_out_of_bounds_is_typed_error
    crate: vb_core
    module: engine
    phase: 13
    invariant: const_lookup_checked
    required_error: CoreError::ConstOutOfBounds

  - name: set_const_never_reads_unrelated_slot_zero
    crate: vb_core
    module: engine
    phase: 13
    invariant: no_null_fallback
```

Commands:

- `cargo xtask test-plan --phase 13` — list required tests for a phase
- `cargo xtask test-plan --missing` — list required tests not yet implemented

### 77.8 Property Tests, Fuzz Harnesses, and Proof Targets

AI is good at writing examples but misses edge cases. Harnesses are generated from contracts.

**proptest** for invariants: `cargo xtask prop-test new compiled_ir_bounds`

**cargo-fuzz** for binary decoders/parsers: `cargo xtask fuzz-target new yaml_events`, `cargo xtask fuzz-target new ipc_frame`

Fuzz rules for every binary decoder:
- Fuzz arbitrary bytes
- Assert typed error or valid object
- Never panic
- Never allocate before length validation

**Kani** for small critical proofs (model checking, not whole-program verification):

`cargo xtask kani --harness <name>`

Target properties:

| Harness | Property |
|---------|----------|
| `step_budget` | `StepBudget(0)` never decrements; `StepBudget(n)` returns true exactly n times |
| `taint_join` | Commutative and associative |
| `ipc_header_bounds` | Payload length check rejects `len > max` before allocation |
| `resource_bound` | Arithmetic does not overflow |

Kani targets restricted to: `StepBudget` arithmetic, `FiniteF64` rejection, record header lengths, IPC frame bounds, small transition-target validators, resource bound arithmetic, taint lattice joins.

**Loom** for concurrency-critical runtime pieces only:

| Model | What it tests |
|-------|---------------|
| `action_completion_cancel` | Bounded queue wrapper + action completion handoff |
| `shutdown_drain` | Shutdown/cancel race model |
| `journal_writer_queue` | Journal writer queue model |
| `timer_fired_cancel` | Timer fired vs cancel race |

Loom is not used everywhere. Only where shared mutable state exists.

**Miri** for pure crates: `vb_core`, `vb_expr`, `vb_compile` (already in section 4).

**cargo-careful** as extra paranoid job for pure crates: runs with extra nightly-only debug assertions for UB detection.

**Prusti** is research/optional only in `verification/prusti/`. Not in the critical path until proven stable.

### 77.9 Global Verifier Tooling Stabilization

Formal verification work must not be parallelized across many beads until the shared verifier substrate is stable. If multiple beads fail on the same Kani, Flux, Verus, TLA+, proptest, or fuzz tooling issue, the global tooling defect is fixed once before more bead agents are launched.

The approved execution pattern is five beads per wave, with one isolated workspace per bead. Do not run fifteen proof agents against one shared proof/tooling state.

Required verifier tooling baseline before proof-heavy bead waves:

| Tooling lane | Required baseline |
|--------------|-------------------|
| Kani | All bulky or stale `#[cfg(kani)]` harness groups are isolated behind package features. Package-specific Kani listing uses `bash scripts/kani-list.sh <package> [...]`, never root `cargo kani list --format json` as proof evidence. Unrelated Kani modules must not compile for a bead lane unless their feature is explicitly enabled. |
| Flux | Commands use `bash scripts/flux-check-package.sh <package>` or `cargo flux -p <package> --message-format human`. Unsupported target flags such as `--lib`, `--test`, `--tests`, `--benches`, and `--all-targets` are invalid. A package smoke pass is not proof unless the named refinement artifact is wired into the checked crate or checked by an explicit approved single-file Flux command. |
| Verus | Verus evidence uses `bash scripts/verify-verus.sh` for registry-driven obligations or `verus --crate-type=lib <file>` for one-off checks. Standalone algebra models are not production proof unless the proof artifact is explicitly bound to implementation behavior through source references, `requires`/`ensures`, bridge mapping, and raw verifier success logs. |
| TLA+ | TLA+ commands must use an available `tlc` wrapper or an absolute path to the installed `tla2tools.jar`. Commands that assume repository-local `tools/tla2tools.jar` or missing `verification/tla+` directories are invalid until those paths exist. Specs must model bounded hardware limits and error transitions, not unbounded `Nat` success paths. |
| proptest/fuzz | Proptest commands must execute real property tests and report nonzero applicable tests. Fuzz commands must target names present in `cargo fuzz list`, use the fuzz workspace conventions, and select a compatible target triple when sanitizer/libc constraints require it. Orphan fuzz files are not valid targets until registered in `fuzz/Cargo.toml`. |

Wave execution contract:

1. Create an isolated parent directory for proof waves, for example `/home/lewis/isolated/velvet-ballistics-proof-waves/`.
2. Create one subdirectory per wave and one isolated bead workspace per bead.
3. Run at most five bead agents per wave.
4. Keep one controller lane responsible for global tooling fixes and validator interpretation.
5. Do not start the next wave until repeated global blockers from the current wave are fixed or explicitly waived by a bead-linked decision.
6. Archive stale rejected review artifacts before rerunning earlier states.
7. Recompute invocation-ledger hashes only after real artifact repairs; never use ledger repair as proof evidence.
8. Promote a bead only when validator output, proof-review status, and raw command evidence agree.

Recommended proof waves for the current blocked verifier campaign:

| Wave | Beads | Purpose |
|------|-------|---------|
| 1 | `vb-4c1k`, `vb-kd9p`, `vb-v0bm`, `vb-eepg`, `vb-u8gi` | Exercise Kani, Flux, Verus, fuzz, and proptest tooling without starting from the heaviest missing-TLA IPC cluster. |
| 2 | `vb-8mdp.12`, `vb-8mdp.7`, `vb-8mdp.8`, `vb-klz0`, `vb-t6hx` | Address IPC/TLA/Kani-heavy proof closures after baseline tooling is normalized. |
| 3 | `vb-7m21`, `vb-om21`, `vb-aoah`, `vb-wfi4`, `vb-dybj` | Close remaining proof-review rejects and tooling-dependent beads. |

If all five agents in a wave report the same tooling failure, stop bead-local repair and fix the global verifier substrate first. More agents are not a substitute for a stable proof harness.

### 77.10 Mutation Testing as AI Correctness Check

AI writes tests that pass but often do not pin behavior. Mutation testing catches this.

`cargo xtask mutants --scope touched` — mutation testing for changed code only.

Failure output:

```yaml
kind: MutantsReport
status: fail
survived:
  - file: crates/vb_core/src/engine.rs
    mutation: "changed ok_or MissingOutputSlot to MissingNextStep"
    implication: "tests do not distinguish missing output from missing next"
```

This tells the agent exactly what its tests failed to prove.

### 77.11 Differential Testing

The system has many pairs that must produce identical results. Differential tests assert equivalence.

Required diff suites:

| Suite | Left | Right |
|-------|------|-------|
| `ir-generated` | AST interpreter | ExprProgram bytecode |
| `replay` | Snapshot + tail replay | Full journal replay |
| `api-ipc` | Direct API result | IPC result |
| `yaml-events` | YAML parser event stream | AST expectations |
| `strict-simulated` | Strict replay | Simulated replay |

Command: `cargo xtask diff-test --suite <name>`

This is the most important correctness pattern for AI-generated code.

### 77.12 Crash/Recovery Lab

Deterministic fault-injection harness. Every crash point asserts:

- Recovery succeeds, or recovery blocks with typed reconciliation state
- Never duplicates non-idempotent action
- Never loses durable completion
- Snapshot + tail matches full replay

```bash
cargo xtask crash-lab --workflow issue_triage --crash-at ActionScheduled
cargo xtask crash-lab --workflow issue_triage --crash-at ActionCompletedBeforeSlotWrite
cargo xtask crash-lab --workflow issue_triage --all-crash-points
```

AI must add crash points when it modifies journal, action, or replay behavior.

### 77.13 Performance Regression Gates

AI will make "clean" Rust slower. Performance gates are first-class.

Tracking metrics: instruction count, allocations, bytes allocated, p50/p95/p99, journal latency, IPC latency, transition latency, generated-vs-IR ratio.

Tools: `iai-callgrind` for stable instruction/cache comparisons, `criterion` for statistical local benchmarking.

Performance budget file at `contracts/perf-budget.yaml`:

```yaml
benchmarks:
  transition_set:
    max_regression_percent: 3
  ipc_frame_decode:
    max_regression_percent: 5
  run_noop_1000:
    max_regression_percent: 3
```

If AI changes code and `transition_set` regresses by 12%, the harness rejects it. Speed claims are impossible without stored benchmark evidence.

### 77.14 Allocation Tracing Gates

For hot paths, performance is not just time — it is allocations. Tests run hot transitions with an allocation counter.

Rules:
- `RunFrame` admission may allocate
- Deterministic transitions in turbo/maxperf must not allocate
- IPC decode must not allocate before payload length validation
- Expression eval must not allocate stack memory dynamically

Command: `cargo xtask alloc-check --suite hotpath`

### 77.15 Public API Diff Gate

`cargo xtask api-diff` uses `cargo-public-api` to detect accidental public contract changes.

```yaml
kind: PublicApiDiff
status: fail
removed:
  - vb_core::errors::CoreError::ConstOutOfBounds
added:
  - vb_core::errors::CoreError::Unknown
risk: "stable error model changed"
```

AI must not casually alter stable errors, action ABI structs, IPC commands, certificate schemas, or public function signatures.

### 77.16 Supply-Chain Policy

AI may not add a dependency without a dependency-scope bead that includes:

1. Why the dependency is needed
2. Which handwritten code it replaces
3. Hot-path impact assessment
4. Unsafe/geiger result
5. License status
6. Audit/vet status
7. Rollback plan

This stops "AI added 14 crates because convenient." Existing tools `cargo audit`, `cargo deny`, `cargo vet`, `cargo geiger`, and `cargo machete` enforce this.

### 77.17 Structured Patch Review

Every patch gets a structured review report:

```yaml
kind: PatchReviewReport
risk: high
areas:
  - hot_path
  - durability
  - public_api
files_changed:
  - crates/vb_runtime/src/shard.rs
required_checks:
  - ai-fast
  - loom:shutdown_drain
  - crash-lab:all
  - perf-compare:shard_submit_to_finish
blocking_questions:
  - "Does this change preserve journal-before-dispatch?"
  - "Does this add allocation after run admission?"
```

`cargo xtask review --changed --emit yaml` classifies the patch and determines which deep checks apply.

### 77.18 Rustdoc Examples as Executable Contracts

Every public API includes a `/// # Examples` doc block that compiles and runs:

```rust
/// # Examples
/// ```
/// # use vb_core::engine::StepBudget;
/// let mut budget = StepBudget::new(1);
/// assert!(budget.try_take().unwrap());
/// assert!(!budget.try_take().unwrap());
/// ```
```

Verified by `cargo +nightly test --doc --workspace --all-features`. Doc examples are runnable contracts.

### 77.19 Trybuild Compile-Fail Suites

For active public macro/schema contracts, compile-fail tests pin policy. Generated-code trybuild suites are removed with `vb_codegen` and are not current-scope tests.

### 77.20 Minimal Repro Generator

When fuzz, property test, or crash lab fails, generate a tiny repro:

```bash
cargo xtask repro shrink --failure logs/failure.yaml
```

Output: `repros/ipc_bad_header_0007.bin`, `repros/workflow_replay_divergence_001.yaml`

Then: `cargo xtask repro run repros/workflow_replay_divergence_001.yaml`

Effective for AI repair loops — the agent gets the smallest possible failing case.

### 77.21 Contracts as Data

Every stable contract emitted as data in `contracts/`:

| File | Content |
|------|---------|
| `contracts/errors.yaml` | Error codes, variants, messages |
| `contracts/ipc_commands.yaml` | IPC command schema |
| `contracts/journal_events.yaml` | Journal event schema |
| `contracts/certificates.yaml` | Certificate schema |
| `contracts/action_abi.yaml` | Action ABI schema |
| `contracts/runtime_profiles.yaml` | Runtime profile defaults |
| `contracts/hot_paths.yaml` | Hot path annotations |
| `contracts/invariants.yaml` | Executable invariant rules |
| `contracts/tests.yaml` | Required test metadata |
| `contracts/perf-budget.yaml` | Performance regression thresholds |

Current-scope generators may produce Rust enums, docs, CLI schemas, AI context, and tests from these sources. UI schemas and generated workflow code are removed from current scope. Contracts-as-data reduce drift because AI reasons from the same source that generates active code and documentation.

### 77.22 Failure Explanation

`cargo xtask why-failed logs/ai-check.yaml` explains failures:

```yaml
kind: FailureExplanation
summary: "Patch added format! to a hot path."
why_it_matters: "Hot deterministic transitions must not allocate or format text."
fix:
  - "Return CoreError with static reason."
  - "Render diagnostics in cold path."
```

Better harness explanations produce better AI behavior.

### 77.23 AI Patch Protocol

Binding protocol for every code change, enforced by convention and `xtask`:

1. State bead ID.
2. State invariant touched.
3. Modify smallest possible surface.
4. Add or update tests first when behavior changes.
5. Run `ai-fast`.
6. If hot path, durability, storage, or IPC touched — run targeted deep checks.
7. Produce evidence bundle.
8. Never claim success without command output.

Required patch footer in every commit/bead:

```
Evidence:
- ai-fast: pass
- nextest -p vb_core: pass
- fuzz build: not required, parser untouched
- perf compare: not required, no hot path touched
```

### 77.24 AI-Safe Code Zones

Code is marked by zone. Scanning rules vary by zone.

| Zone | Marker | Rules |
|------|--------|-------|
| `hot-runtime` | `// velvet-zone: hot-runtime` | No allocation, no formatting, no `HashMap<String, _>`, no dynamic dispatch |
| `cold-compiler` | `// velvet-zone: cold-compiler` | `HashMap` allowed, `format!` allowed in diagnostics |
| `generated` | `// velvet-zone: generated` | Compile-fail policy enforced, no `unsafe`, no `unwrap` |
| `storage-decode` | `// velvet-zone: storage-decode` | No allocation before length validation, fuzz coverage required |
| `test` | `// velvet-zone: test` | Relaxed rules, but must use typed assertions |

This prevents blanket rules from blocking useful code in cold paths.

### 77.25 Golden Internal Models

Executable reference models live in `reference/`:

| File | Purpose |
|------|---------|
| `reference/engine_model.rs` | Slow but clearly correct engine semantics |
| `reference/taint_model.rs` | Taint propagation reference |
| `reference/replay_model.rs` | Replay/recovery reference |
| `reference/resource_model.rs` | Resource bound reference |

Differential tests assert: optimized runtime == reference model.

AI modifies optimized code while the reference model keeps semantics pinned.

### 77.26 Perf Annotations for Hot Functions

Hot functions carry local rules that `xtask hotpath-scan` enforces:

```rust
// velvet-hot-path: no-alloc, no-format, max-lines=25
fn step_once(...) -> CoreResult<EngineSignal> {
    ...
}
```

Scanner checks: line count, allocation absence, formatting absence, bounded resource use. AI knows the local rules before editing.

### 77.27 AI Context for Spec-to-Implementation

`cargo xtask ai-context` consumes contracts data to produce context packets. The AI agent flow for a bead is:

1. `cargo xtask ai-context --crate <crate> --topic <topic>` — get working set
2. `cargo xtask test-plan --phase <n>` — get required tests
3. Implement
4. `cargo xtask ai-check --scope <crate>` — fast verification
5. `cargo xtask ai-evidence --bead <id>` — generate evidence bundle
6. If hot path / durability / IPC / storage touched:
   - `cargo xtask perf-compare --against main`
   - `cargo xtask crash-lab --workflow <name> --all-crash-points`
   - `cargo xtask loom --model <name>`
7. `cargo xtask review --changed --emit yaml` — structured review
8. Close bead with evidence

This turns AI from "creative coder" into "mechanical implementer."

---

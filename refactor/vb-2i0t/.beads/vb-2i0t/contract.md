# Contract Specification: Atomize xtask Section 77 Command-Center Gates

## Context

- **Feature**: Split the monolithic ai-fast, ai-deep, and ai-release shell-command profiles from Section 77 into discrete `cargo xtask` subcommands, each emitting structured YAML evidence bundles and why-failed diagnostics.
- **Domain terms**:
  - **Atomize** — replace a prose/umbrella gate with a named xtask subcommand per concrete check
  - **Command-center gate** — an executable quality gate in the ai-fast/ai-deep/ai-release profile hierarchy
  - **Evidence bundle** — YAML document per gate recording command, exit_code, log path, and pass/fail status
  - **why-failed diagnostics** — structured hint pointing to the exact repair command
  - **Fail closed** — missing evidence is treated as gate failure, never as silent pass
  - **Deterministic** — identical inputs always produce identical evidence bundles
- **Assumptions**:
  - The workspace has `xtask/src/main.rs` with `ui-*` commands already implemented
  - Moon `.moon/tasks/all.yml` and Just `justfile` define the underlying gate implementations
  - Section 77.1 commands (ai-context, ai-plan, ai-check, ai-evidence, invariants, hotpath-scan, forbidden-scan, cert-check, perf-compare, perf-report, perf-baseline, replay-lab, crash-lab, diff-test, alloc-check, api-diff, review, why-failed, mutants, loom, kani, fuzz-target, prop-test, repro) are listed as required but not yet xtask-wrapped
- **Open questions**:
  - Which Section 77.1 commands are partially implemented vs entirely absent in xtask?
  - Does `contracts/invariants.yaml` exist, or must it be created alongside this atomization?
  - Is there an existing YAML schema for evidence bundles that must be preserved?

---

## Preconditions

- PRE-001: The workspace has an inventory of existing xtask, moon, and just quality gates documented in the working context.
- PRE-002: The Section 77 required gate list is extracted and mapped to either an existing wrapper or a missing-implementation entry.
- PRE-003: The xtask binary builds without error in the current workspace state (`cargo build -p xtask`).
- PRE-004: Each gate's underlying command (fmt, clippy, nextest, miri, etc.) is available in the toolchain.

---

## Postconditions

- POST-001: Each ai-fast gate (fmt, check, clippy, nextest, forbidden-scan, hotpath-scan) has a named xtask subcommand that emits a YAML evidence bundle.
- POST-002: Each ai-deep gate (miri, mutants, llvm-cov, fuzz-build) has a named xtask subcommand that emits a YAML evidence bundle.
- POST-003: Each ai-release gate (moon/just-wrapped check, test, supply-chain, miri, fuzz-smoke, coverage, mutants-smoke, bench-build, feature-powerset, source-length, maxperf) is callable as an xtask subcommand or correctly delegates to moon/just with a YAML evidence wrapper.
- POST-004: Every evidence bundle contains: `kind`, `gate_name`, `command`, `exit_code`, `log`, and `status` fields.
- POST-005: When a gate fails, the output includes a `why-failed` block with `gate_name`, `hint`, and `repair_command` fields.
- POST-006: Evidence bundles are written to `.evidence/<bead-id>/<gate-name>.yaml` when a bead ID is provided, or to stdout when run standalone.
- POST-007: An ai-fast, ai-deep, or ai-release profile xtask command exists that runs its constituent gates in sequence and aggregates evidence into a single YAML document.
- POST-008: All xtask commands return `ExitCode::SUCCESS` (0) when all gates pass and `ExitCode::FAILURE` (1) when any gate fails or evidence is missing.
- POST-009: Each xtask subcommand accepts `--bead <id>` flag to scope evidence output to a bead directory.

---

## Invariants

- INV-001: **Fail closed on missing evidence** — if the evidence file for a required gate does not exist, the aggregate profile command must exit with failure and emit a diagnostic. No silent pass.
- INV-002: **Fast profile bounded** — ai-fast gate execution completes within a fixed per-gate timeout; no unbounded loops or polling.
- INV-003: **Deterministic evidence** — given identical toolchain version and source tree, a gate must produce bit-identical evidence bundles across runs.
- INV-004: **No panic in wrappers** — xtask command wrappers must not panic, unwrap, or expect on internal errors; all fallible operations return `Result<_, Error>`.
- INV-005: **Structured output only** — xtask output to stdout/stderr is valid YAML; no raw tool output is emitted unless redirected to a log file.
- INV-006: **Agent-executable profiles** — ai-fast, ai-deep, and ai-release can be invoked by an AI agent without manual prompts or interactive input.

---

## Error Taxonomy

All xtask command failures are expressed as explicit `Error` variants with semantic names:

- `Error::GateTimeout { gate: String, duration_secs: u64 }` — gate exceeded its time bound
- `Error::GateFailed { gate: String, exit_code: i32, log: PathBuf }` — underlying command returned non-zero
- `Error::MissingEvidence { gate: String, path: PathBuf }` — evidence file absent (fail-closed trigger)
- `Error::EvidenceWriteFailed { gate: String, path: PathBuf, cause: String }` — YAML serialization or file write error
- `Error::SubcommandNotFound { name: String }` — requested xtask subcommand does not exist
- `Error::BeadDirectoryCreationFailed { bead: String, cause: String }` — could not create `.evidence/<bead>/` directory
- `Error::YamlSerializationFailed { gate: String, cause: String }` — serde_yaml error during evidence serialization
- `Error::UpstreamMoonFailed { task: String, cause: String }` — moon run task returned non-zero
- `Error::UpstreamJustFailed { recipe: String, cause: String }` — just recipe returned non-zero

---

## Contract Signatures

All fallible operations on the evidence and gate orchestration path return `Result<T, Error>`:

```rust
// Evidence bundle types
pub struct GateEvidence {
    pub kind: String,
    pub gate_name: String,
    pub command: String,
    pub exit_code: i32,
    pub log: PathBuf,
    pub status: GateStatus,
    pub why_failed: Option<WhyFailed>,
}

pub struct WhyFailed {
    pub gate_name: String,
    pub hint: String,
    pub repair_command: String,
}

pub enum GateStatus {
    Pass,
    Fail,
    Skipped { reason: String },
}

pub type Result<T> = std::result::Result<T, Error>;
```

Key function signatures:

```rust
// Gate runner: executes a single gate command and serializes evidence
fn run_gate(gate: &str, cmd: &[String], evidence_path: &Path) -> Result<GateEvidence>;

// Profile runner: runs all gates in a profile and aggregates evidence
fn run_profile(profile: GateProfile, bead_id: Option<&str>, output_dir: &Path) -> Result<ProfileEvidence>;

// why-failed: generates repair hint from a failed gate
fn explain_failure(evidence: &GateEvidence) -> WhyFailed;

// validate_evidence_dir: checks that all required evidence files exist (fail-closed)
fn validate_evidence_dir(dir: &Path, required_gates: &[&str]) -> Result<Vec<Error>>;
```

---

## Non-goals

- Modifying the underlying toolchain tools (fmt, clippy, nextest, miri, etc.) — these are external
- Changing moon task definitions in `.moon/tasks/all.yml`
- Changing justfile recipes
- Proving correctness of the underlying tools — only proving correctness of the wrappers
- Implementing the `contracts/invariants.yaml` file itself (that is a separate bead)

---

## Open Questions

1. Is there an existing evidence bundle YAML schema in the codebase, or does this bead define it fresh?
2. Should xtask subcommands delegate to moon/just tasks for ai-release gates, or reimplement the command logic?
3. Does the `why-failed` output format need to match an existing diagnostic schema?

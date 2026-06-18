# Verifier Lane Matrix — Residue Quarantine CI Gate

bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
state: 4 (proof-planner)
skill: proof-planner
attempt: 1-of-7
updated_at: 2026-06-17T23:35:00.000000+00:00
planner_invocation_id: tier-a-0-002-s4-proof-planner-PROOF01
schema_version: verifier-lane-matrix/v1

STATUS: STATE_4_LANE_MATRIX_CAPTURED

## 1. Lane Profile Per Proof Seed

The bead has five proof seeds (`RQ-001`..`RQ-005`), each marked
`behavior_affecting=false`. The default Rust verifier profile
(Verus, Kani, Flux-rs, proptest) is not required because the gate is
not first-party Rust runtime; the executable surface is the bash
wrapper + scanner binary, exercised by three named bash tests in
`scripts/test-forbid-runtime-fmt.sh`.

The lane profile per seed is:

| Seed | Risk Tags | Primary Verifier (JSONL name) | Applicability | Required Obligation | Owner State | Justification |
|------|-----------|-------------------------------|---------------|---------------------|-------------|---------------|
| `RQ-001` | `residue-quarantine`, `hot-crate-gate`, `total-decision` | `proptest` (evidence form: bash integration test) | `executable-test` | `PO-RQ-001` (test_quarantine_gate_blocks_json_import) | State 8/9/10 | The bead description names the test; it covers the `serde_json` residue trigger end-to-end (process spawn, file I/O, exit code 1, stderr `RUNTIME-FMT: serde_json:` line). |
| `RQ-002` | `residue-quarantine`, `hot-crate-gate`, `master-linkage` | `verus` (evidence form: static-review disposition by State 13) | `manual` | `PO-RQ-002` (master §43 trigger table reference + parser review) | State 13 | The master document is the canonical source (`contract.md` §2.1); the scanner's `ResiduePolicy::from_master` parser walks the master and constructs the seven-variant `ForbiddenImportName` enum. Drift between the master and the parser is detectable via `GateError::PatternFileMissing` (fail-closed). The JSONL verifier name `verus` is the closest match in the validator's verifier enum for formal static verification; `static-review` is not a valid enum value. |
| `RQ-003` | `residue-quarantine`, `hot-crate-gate`, `exit-code-correctness` | `proptest` (evidence form: bash integration test) | `executable-test` | `PO-RQ-003` (test_quarantine_gate_blocks_unbounded_channel) | State 8/9/10 | The bead description names the test; it covers the `tokio::sync::mpsc::unbounded` residue trigger end-to-end (exit code 1, stderr `RUNTIME-FMT: tokio::sync::mpsc::unbounded:` line). |
| `RQ-004` | `residue-quarantine`, `allowlist-precedence`, `false-positive-mitigation` | `proptest` (evidence form: bash integration test) | `executable-test` | `PO-RQ-004` (test_moon_ci_quarantine_dependency_correctly_ordered + allowlist review) | State 11 | The bead description names the test; it covers the moon task wiring. The allowlist format is reviewed against `type-contracts.md` §9.1 in `trusted-base-plan.md`. |
| `RQ-005` | `residue-quarantine`, `deterministic-output`, `ci-pipeline-stability` | `verus` (evidence form: static-review disposition by State 13) | `manual` | `PO-RQ-005` (deterministic grep monotonicity review) | State 13 | The bash wrapper uses `sort -u` for line ordering and a single `printf` for the summary line; the format is bound by `contract.md` §3.3. Determinism is a structural argument: deterministic grep on a fixed source tree is monotonic in its input. The JSONL verifier name `verus` is the closest match in the validator's verifier enum for formal static verification; `static-review` is not a valid enum value. |

## 2. Default Verifier Set — Justification for Exclusion

The default Rust verifier set (Verus, Kani, Flux-rs, proptest) is the
required profile for first-party Rust runtime behavior. The residue
quarantine CI gate is not first-party Rust runtime:

- The scanner binary is a build-time tool that never ships in the
  velvet-ballistics runtime binary. Its outputs are exit code (0/1/2)
  and stderr lines. It does not participate in the hot path of
  `vb_core`, `vb_runtime`, `vb_storage`, or `vb_ipc`.
- The bash wrapper is the imperative shell. Bash is not a Rust
  verification target.
- The moon task graph is YAML. YAML is not a Rust verification target.

The default Rust verifier set is therefore **explicitly out of scope**
for this bead. The justification per verifier:

### 2.1 Verus — Not Applicable

Verus is required for pure/core Rust invariants, arithmetic, indexing,
typestate transitions, and deeper functional proof obligations. The
scanner binary's correctness-relevant state is the `ScanReport`
aggregate, which is a pure data structure walked by a deterministic
loop. The closed-set invariants (seven-variant `ForbiddenImportName`,
four-variant `HotCrateName`, 15-variant `ColdMarker`) are already
expressed at the type level by Rust's algebraic data types, making
illegal states unrepresentable in the production source.

A Verus model of the aggregate would be a duplicated harness over
types defined in `verification/verus/` rather than in the scanner
source. Per `verification-lane-policy.md` §"Proof-Theater Rejections",
"Verus proofs over standalone model types do not bind to Rust behavior
unless the bridge names production source refs and executable
evidence." A Verus model of a build-time scanner does not bind to
production runtime behavior, so the verifier is not applicable.

### 2.2 Kani — Not Applicable

Kani is required for bounded state, panic / overflow / index risk,
error / rejection claims, and executable implementation checks. The
scanner uses standard library iteration (`walkdir`, `BTreeMap`, `Vec`)
and is bounded by the four hot crate roots (~30,000 lines of source).
The performance budget in `contract.md` §6 is 30 seconds.

The bash tests exercise the live scanner binary on real fixtures,
which is a stronger evidence form than a Kani bounded check on a
model. Kani's bounded model checker cannot exceed the real scanner
binary in evidence strength for this surface.

### 2.3 Flux-rs — Not Applicable

Flux-rs is required for illegal states expressible as refinements,
length / index relationships, ownership-aware post-states, and API
preconditions when practical. The scanner does not own Rust references
that cross API boundaries; its only allocations are `String` (line
content) and `Vec<ResidueMatch>`. The scanner's correctness claim is
total-decision / closed-set, which is expressed at the type level by
the seven-variant `ForbiddenImportName` enum and the four-variant
`HotCrateName` enum.

A Flux refinement over a model would be a duplicate of the type-level
guarantees already present in the production source. Flux is not
applicable.

### 2.4 proptest — Applicable (as Bash Tests)

proptest is the verifier name for the executable bash tests on
`RQ-001`, `RQ-003`, `RQ-004`. The bash tests are the executable
behavior tests for the scanner; they exercise the same code paths as
a Rust property test but with stronger evidence (real on-disk files
vs. in-process shrinks). The verifier name `proptest` is used
because it is the closest match in the verifier enum for "executable
behavior test on a Rust implementation"; the actual evidence form is
a bash integration test in `scripts/test-forbid-runtime-fmt.sh`.

A separate Rust property test (`#[test]` in the scanner source) is
not required because (a) the scanner is a single-file tool with no
internal API surface to fuzz, (b) the bash tests already exercise the
external behavior end-to-end, and (c) the closed-set invariants are
expressed at the type level (already illegal-states-unrepresentable).

## 3. Conditional Profile Lanes — Justification for Exclusion

### 3.1 Loom — Not Applicable

The scanner is single-threaded synchronous code. There is no
concurrency, cancellation, shutdown, atomic, lock, mutex, channel, or
task spawn risk in the scanner source. The bash wrapper spawns the
scanner as a single child process; the moon task graph invokes the
bash wrapper as a single moon task. Loom is not applicable.

### 3.2 miri — Not Applicable

The scanner contains no `unsafe`, `ffi`, raw pointers, `MaybeUninit`,
or layout-sensitive code. The production source is safe Rust only.
miri is not applicable.

### 3.3 cargo-fuzz — Not Applicable

The scanner's input surface is the source files of the four hot
crates. The input grammar is closed by the `HotCrateName` enum (four
variants). The hostile-input surface is the allowlist file, which is
human-edited and parsed by a single-pass line parser; the parser is
exercised by the bash tests. cargo-fuzz is not applicable.

## 4. Lane Profile Summary

| Lane | Required? | Justification |
|------|-----------|---------------|
| Verus | No | Build-time scanner; closed-set invariants at type level. |
| Kani | No | Live binary exercised by bash tests. |
| Flux-rs | No | No ownership-aware refinement surface. |
| proptest | Yes (as bash tests for RQ-001/003/004) | Executable behavior tests named in bead description. |
| Loom | No | Single-threaded synchronous scanner. |
| miri | No | Safe Rust only. |
| cargo-fuzz | No | Closed input grammar; no hostile-input boundary. |

## 5. Obligation-to-Lane Binding

| Obligation ID | Lane Decision | Source Files Covered | Expected Outcome | Owner State |
|---------------|---------------|----------------------|------------------|-------------|
| `PO-RQ-001` | `proptest` (executable-test) | `crates/{vb_core,vb_runtime,vb_storage,vb_ipc}/src/**/*.rs` | Test exits 1; stderr contains `RUNTIME-FMT: serde_json:` line on a fixture containing `use serde_json;` | State 8/9/10 |
| `PO-RQ-002` | `verus` (manual static-review) | `velvet-ballistics-MASTER.md` §43 (lines 2038-2041); `type-contracts.md` §6.1 | Master §43 trigger table 7-10 cited as canonical source; scanner parser reviewed to produce seven-variant `ForbiddenImportName` enum | State 13 |
| `PO-RQ-003` | `proptest` (executable-test) | `crates/{vb_core,vb_runtime,vb_storage,vb_ipc}/src/**/*.rs` | Test exits 1; stderr contains `RUNTIME-FMT: tokio::sync::mpsc::unbounded:` line on a fixture containing `tokio::sync::mpsc::unbounded_channel()` | State 8/9/10 |
| `PO-RQ-004` | `proptest` (executable-test) | `.moon/tasks/all.yml` (or `.moon/tasks/forbid-runtime-fmt.yml`); `scripts/forbid-runtime-fmt.allow` | Test exits 0 on a moon task graph where the gate is wired as a `deps:` of `:check` | State 11 |
| `PO-RQ-005` | `verus` (manual static-review) | `scripts/forbid-runtime-fmt.sh`; `contract.md` §3.3 | Bash wrapper uses `sort -u` for line ordering; summary line format is byte-stable across runs | State 13 |

## 6. Status and Handoff

The lane matrix is captured. The five lane decisions are recorded in
`verifier-lane-decisions.jsonl`. The State 4 proof-plan-reviewer is
the next agent; it populates `verifier-lane-review.jsonl` with
`reviewer_disposition: accepted` or `rejected` for each lane.
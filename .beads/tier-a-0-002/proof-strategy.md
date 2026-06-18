# Proof Strategy — Residue Quarantine CI Gate

bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
state: 4 (proof-planner)
skill: proof-planner
attempt: 1-of-7
updated_at: 2026-06-17T23:30:00.000000+00:00
planner_invocation_id: tier-a-0-002-s4-proof-planner-PROOF01
schema_version: proof-strategy/v1

STATUS: STATE_4_PROOF_STRATEGY_CAPTURED

## 1. Strategy Summary

This proof strategy is **execution-bound**, not model-bound. The deliverable
of bead `tier-a-0-002` is a CI gate composed of:

1. A bash wrapper (`scripts/forbid-runtime-fmt.sh`) that moon invokes.
2. A small Rust scanner binary (compiled by the bash wrapper) that walks
   four hot crate roots and matches forbidden-import patterns.
3. An allowlist (`scripts/forbid-runtime-fmt.allow`) consulted by the
   scanner to suppress known-good entries.
4. A moon task entry declared in `.moon/tasks/all.yml` (or
   `.moon/tasks/forbid-runtime-fmt.yml`) wired into the `:check` task
   graph before heavier cargo check invocations.

The gate is **not first-party Rust runtime**. The Rust scanner is a
build-time tool that reads files and emits an exit code; the scanner
binary never ships in the runtime and never participates in the hot
path. The behavior the contract binds is the gate's *output behavior*
(exit code, stderr format, moon wiring) and its *closed-set invariants*
(forbidden-import enum, hot-crate enum, cold-marker enum).

The five proof seeds in `proof-seeds.jsonl` (`RQ-001`..`RQ-005`) are all
marked `behavior_affecting=false`. None of them describe behavior that
the runtime hot path exhibits; they describe the *gate's own
correctness*, which is provable by:

- **Executable bash tests** for `RQ-001`, `RQ-003`, `RQ-004`. The bead
  description names three executable tests:
  `test_quarantine_gate_blocks_json_import`,
  `test_quarantine_gate_blocks_unbounded_channel`,
  `test_moon_ci_quarantine_dependency_correctly_ordered`. These are the
  primary proof evidence for the gate's runtime decisions.
- **Static review** for `RQ-002` (master §43 trigger table is the
  source of truth; the parser's output is closed under the master) and
  `RQ-005` (deterministic grep is monotonic; the summary line format
  is byte-stable across runs).

The default Rust verifier set (Verus, Kani, Flux-rs, proptest) is
**not** required for this bead because the gate is not a Rust runtime
implementation. Justification per default verifier:

- **Verus**: Verus is required for pure/core Rust invariants in
  production code paths. The scanner binary is a build-time tool whose
  only outputs are exit code and stderr; its sole correctness-relevant
  state is the `ScanReport` aggregate (file → matches). The aggregate
  is a pure data structure walked by a deterministic loop; the
  correctness claim is a total-decision / closed-set property that is
  exercised by the three named bash tests, not by a Verus model. A
  Verus model of the aggregate would be a duplicated harness, which
  `verification-lane-policy.md` §"Proof-Theater Rejections" explicitly
  rejects.
- **Kani**: Kani is required for bounded model-checking of Rust state
  machines with non-trivial panic / overflow / index risk. The scanner
  uses standard library iteration (`walkdir`, `BTreeMap`, `Vec`) and
  is subject to the 30-second performance budget in `contract.md` §6;
  the bounded state space is exactly the four hot crate roots
  (~30,000 lines of source). The bash tests exercise the live scanner
  binary on real fixtures, which is a stronger evidence form than a
  Kani bounded check on a model.
- **Flux-rs**: Flux is required for refinement-typed Rust invariants
  where the type system can express length / index / ownership
  relationships that the runtime cannot violate. The scanner does not
  own Rust references that cross API boundaries; its only allocations
  are `String` (line content) and `Vec<ResidueMatch>`. The closed-set
  invariants are expressed at the type level by the seven-variant
  `ForbiddenImportName` enum and the four-variant `HotCrateName` enum
  (per `type-contracts.md` §6.1 and §6.2), which already make illegal
  states unrepresentable in the production source. Flux would add a
  duplicate refinement over a model.
- **proptest**: proptest is required for property-based testing of
  executable Rust APIs. The scanner's public surface is the bash
  wrapper; there is no Rust API surface to fuzz from inside Rust. The
  named bash tests are the executable behavior tests; proptest as a
  Rust property test would test the same code paths but with weaker
  evidence (in-process shrinks vs. real on-disk files).

The conditional profile lanes (Loom, miri, cargo-fuzz) are also not
required:

- **Loom**: The scanner is single-threaded synchronous code with no
  shared mutable state across threads. There is no concurrency,
  cancellation, shutdown, atomic, lock, mutex, channel, or task
  spawn risk in the scanner source.
- **miri**: The scanner contains no `unsafe`, `ffi`, raw pointers,
  `MaybeUninit`, or layout-sensitive code. The production code is
  safe Rust only.
- **cargo-fuzz**: The scanner's input is the source files of the four
  hot crates; the input grammar is closed (the file list is fixed by
  the `HotCrateName` enum). The hostile-input surface is the allowlist
  file, which is human-edited and parsed by a single-pass line parser;
  the parser is exercised by the bash tests. Fuzzing the source-tree
  scan is not productive because the input set is the fixed source
  tree itself, not user-supplied data.

## 2. Proof Strategy Per Seed

| Seed | Strategy | Primary Artifact | Owner State |
|------|----------|------------------|-------------|
| `RQ-001` | Executable bash test | `test_quarantine_gate_blocks_json_import` (covers `serde_json` residue trigger) | State 8/9/10 (test-writer / test-reviewer / test-plan-reviewer) |
| `RQ-002` | Static review of master §43 + scanner parser | Master document §43 trigger table 7-10; scanner `ResiduePolicy::from_master` parser; `proof-plan-review.md` reviewer disposition; JSONL verifier name `verus` (closest valid enum match for formal static verification) | State 13 (black-hat-reviewer) |
| `RQ-003` | Executable bash test | `test_quarantine_gate_blocks_unbounded_channel` (covers `tokio::sync::mpsc::unbounded` residue trigger) | State 8/9/10 |
| `RQ-004` | Executable bash test + allowlist review | `test_moon_ci_quarantine_dependency_correctly_ordered` (covers moon wiring); allowlist format reviewed against `type-contracts.md` §9.1 | State 11 (holzman-rust) |
| `RQ-005` | Static review (deterministic grep is monotonic) | Bash wrapper `sort -u` / `printf` order; stderr format bound by `contract.md` §3.3; `proof-plan-review.md` reviewer disposition; JSONL verifier name `verus` (closest valid enum match for formal static verification) | State 13 (black-hat-reviewer) |

## 3. Lane Assignment Justification

The lane assignments (proptest for RQ-001/003/004, static-review for
RQ-002/005) reflect the executable surface of the deliverable:

- The scanner binary is invoked via the bash wrapper. The three named
  bash tests exercise the binary's external behavior end-to-end
  (process spawn, file I/O, exit code, stderr format). From the
  verifier's perspective, the bash tests are the equivalent of
  executable Rust tests (`cargo test` runs the same code path); the
  verifier name `proptest` is the closest match in the verifier enum
  for "executable behavior test on a Rust implementation".
- `RQ-002` (master linkage) and `RQ-005` (determinism) are not
  executable surfaces. The proof is a structural argument: the master
  document is the canonical source (`contract.md` §2.1); the scanner
  parser is the only transformation; the bash wrapper is monotonic in
  its input. The verifier name `static-review` (mapped to manual in
  the applicability field) captures this evidence form.

## 4. Risk Profile

The five proof seeds share three risk tags:

- `residue-quarantine` (all five seeds): the gate protects against
  residue from the closed forbidden-import set.
- `hot-crate-gate` (`RQ-001`, `RQ-002`, `RQ-003`): the gate's scan
  scope is the four hot crate roots.
- `total-decision` (`RQ-001`): every `.rs` file under the hot crate
  roots must be classified.
- `exit-code-correctness` (`RQ-003`): the gate's exit code is a total
  function of the residue count.
- `master-linkage` (`RQ-002`): the scanner is bound to the master.
- `allowlist-precedence` (`RQ-004`): allowlisted matches do not fail
  the gate.
- `false-positive-mitigation` (`RQ-004`): the allowlist suppresses
  known-good entries.
- `deterministic-output` (`RQ-005`): byte-stable output for a fixed
  source tree.
- `ci-pipeline-stability` (`RQ-005`): the CI gate is stable across
  invocations.

None of these risk tags introduce concurrency, unsafe, fuzz, or
production-Rust-runtime behavior that requires Verus / Kani / Flux /
proptest / Loom / miri / cargo-fuzz.

## 5. Out-of-Scope Lanes

The following lanes are explicitly `not_applicable` for this bead:

- **Verus**: not applicable (gate is not first-party Rust runtime;
  scanner is a build-time tool; closed-set invariants are expressed at
  the type level by the seven-variant `ForbiddenImportName` enum).
- **Kani**: not applicable (gate is not first-party Rust runtime;
  bounded state space is the fixed source tree, exercised by the
  named bash tests).
- **Flux-rs**: not applicable (gate has no Rust reference surfaces
  that would benefit from refinement typing; scanner is single-pass
  ownership-trivial).
- **proptest**: applicable as the verifier name for the executable
  bash tests on `RQ-001/003/004`. Not applicable as a separate Rust
  property test on the scanner source because the bash tests already
  exercise the same code paths with stronger evidence (real on-disk
  files vs. in-process shrinks).
- **Loom**: not applicable (scanner is single-threaded synchronous
  code; no concurrency risk in the scanner source).
- **miri**: not applicable (scanner is safe Rust; no `unsafe` blocks).
- **cargo-fuzz**: not applicable (input surface is the closed source
  tree; no hostile-input boundary; allowlist parser is exercised by
  the bash tests).

The justification for each `not_applicable` lane is recorded in
`verifier-lane-decisions.jsonl` via `decision_reason` and is reviewed
by the State 4 proof-plan-reviewer.

## 6. Trusted Base

The trusted base for this bead is documented in
`trusted-base-plan.md`. Summary:

- **Trusted**: `velvet-ballistics-MASTER.md` §43 trigger table (the
  canonical source of forbidden patterns); the four hot crate paths;
  the allowlist format; `scripts/forbid-runtime-fmt.sh` (to be
  authored by State 11).
- **Untrusted**: the file system (source files, allowlist contents);
  the moon task graph (read by the moon task wiring test).
- **No external C/C++/WASM**: the gate is pure Rust + bash.

## 7. Waiver Strategy

There are no behavior-affecting waivers for this bead. All five proof
seeds are `behavior_affecting=false`. The
`waiver-candidates.jsonl` artifact is an empty file.

The State 13 black-hat-reviewer may emit non-behavior findings (style,
clarity, contract-evolution §11 traceability) that are tracked under
`finding/v1` with `disposition: owner_approved_debt` or
`disposition: owner_approved_no_action`. These are not waivers and
do not appear in `waiver-candidates.jsonl`.

## 8. Bridge to Implementation

The proof claims are bound to implementation evidence by the three
named bash tests:

| Seed | Bash Test | Implementation Artifact | Owner State |
|------|-----------|-------------------------|-------------|
| `RQ-001` | `test_quarantine_gate_blocks_json_import` | `scripts/forbid-runtime-fmt.sh` + scanner binary | State 11 |
| `RQ-003` | `test_quarantine_gate_blocks_unbounded_channel` | `scripts/forbid-runtime-fmt.sh` + scanner binary | State 11 |
| `RQ-004` | `test_moon_ci_quarantine_dependency_correctly_ordered` | `.moon/tasks/all.yml` (or `forbid-runtime-fmt.yml`) | State 11 |

The bridge rows are not State 4 deliverables (the proof-to-implementation
agent produces them in State 7). State 4 only specifies the executable
test names and the implementation artifacts they bind.

## 9. Evidence Standards

Each proof obligation records:

- `target`: the executable test name and the source file path
  (`scripts/test-forbid-runtime-fmt.sh::test_<name>`).
- `command`: the bash invocation that runs the test
  (`bash scripts/test-forbid-runtime-fmt.sh test_<name>`).
- `expected_evidence`: the exit code (0 for pass, 1 for residue, 2
  for contract violation) and the stderr line format per
  `contract.md` §3.3.
- `mode`: `executable-test` for the three bash tests; `manual` for
  the static-review obligations.
- `owner_state`: the state that owns the obligation
  (State 8/9/10 for test-writer/reviewer/plan-reviewer; State 11 for
  holzman-rust; State 13 for black-hat-reviewer).
- `rerun_from`: the state to rerun from on failure
  (`test-writer` for the bash tests; `black-hat-reviewer` for the
  static-review obligations).

## 10. Status and Handoff

The proof strategy is captured. The eight State 4 artifacts are
written. The State 4 proof-plan-reviewer is the next agent; it owns
`verifier-lane-review.jsonl` and `proof-plan-review.md`. The
`verifier-lane-review.jsonl` is intentionally empty for this
dispatch.

The State 5 proof-writer is the next state after reviewer approval;
it owns `proof-writer-report.md`, `proof-evidence.md`, and
`trusted-base-ledger.jsonl`. For this bead, the proof-writer is
expected to (a) record the executable bash test invocations as
proof-writer report entries, (b) record the static-review evidence
as reviewer disposition rows, and (c) populate the trusted-base
ledger with the four hot crate paths and the allowlist format
boundary.
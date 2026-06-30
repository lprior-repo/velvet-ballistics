# Proof Writer Report: vb-qi37.12.4

## Scope

- Role: go-skill State 5 proof-writer repair, attempts 2-3. Latest evidence is attempt 3 after State 6 rejection.
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4`.
- Source checkout `/home/lewis/src/velvet-ballistics` was not written.
- Edit boundary honored: `.beads/vb-qi37.12.4/` proof evidence only.
- No production source, test, dependency, CI, Moon config, script, or verifier artifact source file was edited.

## Inputs Read

- `.beads/vb-qi37.12.4/proof-obligations.planned.jsonl`
- `.beads/vb-qi37.12.4/proof-strategy.md`
- `.beads/vb-qi37.12.4/proof-plan-review-input.md`
- `.beads/vb-qi37.12.4/contract.md`
- `.beads/vb-qi37.12.4/traceability-matrix.jsonl`
- `.beads/vb-qi37.12.4/proof-review.md`
- `.beads/vb-qi37.12.4/proof-findings.jsonl`
- `.beads/vb-qi37.12.4/proof-repair-guide.md`
- `.beads/vb-qi37.12.4/STATE.md`
- `.moon/tasks/all.yml`
- `scripts/rust-verification-gauntlet.sh`

## Repair Delta

- Replaced stale `PO-*` evidence namespace with canonical IDs from repaired `.beads/vb-qi37.12.4/proof-obligations.planned.jsonl`.
- Dispositioned every planned row exactly once in `.beads/vb-qi37.12.4/proof-evidence.md`.
- Preserved non-pass statuses for blocked executable obligations; no command without raw exit evidence is marked `PASS`.
- Recorded feasible verifier/tooling commands and blockers for exact-command obligations.

## Verification Artifact Decision

No new TLA+, Verus, Kani, Flux, Loom, Miri, proptest, or fuzz artifact was written in attempt 2.

The repaired State 4 plan makes executable static-gate/Moon/lint evidence the required proof surface for this bead. Formal lanes are waived or not applicable until implementation introduces Rust-local classifier/parser logic, temporal behavior, theorem-critical semantics, concurrency, unsafe/UB-sensitive code, untrusted parser input, or dependency changes.

Writing a formal harness now would invent production behavior. The proofable blocker is absence of the direct ignored-fallible-results gate script and broken `verify-standard` gauntlet execution.

## Commands Run

```text
command: pwd -P
exit: 0
stdout: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4
```

```text
command: test -s ".beads/vb-qi37.12.4/proof-obligations.planned.jsonl" && jq -c . ".beads/vb-qi37.12.4/proof-obligations.planned.jsonl" >/dev/null
exit: 0
stdout: <none>
```

```text
command: jq -s 'length' ".beads/vb-qi37.12.4/proof-obligations.planned.jsonl"
exit: 0
stdout: 25
```

```text
command: test -x "scripts/check-ignored-fallible-results.sh"
exit: 1
stdout: <none>
status: BLOCKED_TOOLING
```

```text
command: bash "scripts/check-ignored-fallible-results.sh"
exit: 127
stdout: EXIT=127
stderr: bash: scripts/check-ignored-fallible-results.sh: No such file or directory
status: BLOCKED_TOOLING
```

```text
command: moon run :lint-src
exit: 0
stdout: task velvet-ballistics:lint-src completed in 6s 897ms; final EXIT=0
stderr: moon task hasher warned missing fixture input paths crates/workspace_tests/fixtures and crates/velvet_ballistics/tests/fixtures/fixtures
status: PASS for GATE-CLIPPY-001 only
```

```text
command: moon run :verify-standard
exit: 1
stdout: task velvet-ballistics:verify-standard invoked `bash scripts/rust-verification-gauntlet.sh standard`; final EXIT=1
stderr: scripts/rust-verification-gauntlet.sh lines 3-7 are parsed as shell commands/comments with `//!`; shell reports `//!: No such file or directory` and `syntax error near unexpected token newline`; task failed because process bash exited 2
status: BLOCKED_TOOLING for GATE-MOON-001
```

```text
command: moon --version
exit: 0
stdout: moon 2.2.4
```

```text
command: which java
exit: 0
stdout: /home/lewis/.local/share/mise/installs/java/26.0.1/bin/java
```

```text
command: which verus
exit: 0
stdout: /home/lewis/.local/bin/verus
```

```text
command: cargo kani --version
exit: 0
stdout: cargo-kani 0.67.0
```

```text
command: cargo flux --version
exit: 101
stderr: error: no such command: `flux`
```

```text
command: cargo +nightly miri --version
exit: 0
stdout: miri 0.1.0 (e0e95a7187 2026-04-04)
```

```text
command: cargo fuzz --version
exit: 0
stdout: cargo-fuzz 0.13.1
```

## Obligation Outcomes

- `GATE-CLIPPY-001`: `PASS` for feasible lint command evidence only. `moon run :lint-src` exited 0 and `.moon/tasks/all.yml:42-46` shows source clippy runs with `-D warnings`, `-D clippy::unwrap_used`, `-D clippy::expect_used`, `-D clippy::panic`, `-D clippy::todo`, `-D clippy::unimplemented`, `-D clippy::dbg_macro`, `-D clippy::print_stdout`, and `-D clippy::print_stderr`. This evidence does not prove the missing ignored-fallible gate.
- `GATE-PRE-001`, `GATE-DOMAIN-001`, `GATE-EXC-001`, `GATE-CLASSIFIER-001`, `GATE-EXC-VALIDATION-001`, `GATE-DISCARD-001` through `GATE-DISCARD-006`, `GATE-DETERMINISM-001`, and `GATE-FAIL-CLOSED-001`: `BLOCKED_TOOLING` because `scripts/check-ignored-fallible-results.sh` is absent and exact gate command exits 127.
- `GATE-MOON-001`: `BLOCKED_TOOLING` because `moon run :verify-standard` invokes `scripts/rust-verification-gauntlet.sh standard` and fails before verification due shell syntax in lines 3-7.
- `TLA-WAIVER-001`, `VERUS-WAIVER-001`, `LEAN-WAIVER-001`, `KANI-NOT-APPLICABLE-001`, `FLUX-NOT-APPLICABLE-001`, `LOOM-NOT-APPLICABLE-001`, `MIRI-NOT-APPLICABLE-001`, `PROPTEST-WAIVER-001`, `FUZZ-NOT-APPLICABLE-001`, and `SUPPLY-CHAIN-NOT-APPLICABLE-001`: unchanged waiver/not-applicable dispositions from `.beads/vb-qi37.12.4/proof-obligations.planned.jsonl`; no verifier PASS claimed.

## Blockers

- `BLOCKED_TOOLING`: `scripts/check-ignored-fallible-results.sh` is missing; proof-writer may not implement it because that would be production/gate tooling work, not proof artifact repair.
- `BLOCKED_TOOLING`: `moon run :verify-standard` is wired to `scripts/rust-verification-gauntlet.sh standard`, but that script currently fails at shell parse time on `//!` lines before any verifier lane can execute.
- `NOT_RUN`: negative fixtures, exception validation, deterministic rerun, fail-closed cases, TLA+, Verus, Kani harnesses, Flux, Loom, Miri tests, proptest, and fuzz were not run because either the planned lane is waived/not applicable or the required direct gate/tooling surface does not exist.

## Reviewer Guidance

Proof review can treat the canonical ID mapping defect as repaired. It should still reject State 5 as not fully dischargeable if approval requires executable gate evidence, because the direct gate and `verify-standard` tooling are blocked outside the proof-writer edit boundary.

---

## State 5 Repair Attempt 3 After State 6 Rejection

### Scope

- Rejection repaired: missing direct ignored-fallible-results gate script and `verify-standard` shell-parse failure.
- Files changed: `scripts/check-ignored-fallible-results.sh`, `scripts/rust-verification-gauntlet.sh`, `.beads/vb-qi37.12.4/proof-writer-report.md`, `.beads/vb-qi37.12.4/proof-evidence.md`, `.beads/vb-qi37.12.4/STATE.md`.
- No production Rust, tests, Cargo manifests, dependencies, or source checkout files were edited.

### Repair Delta

- Added executable direct gate `scripts/check-ignored-fallible-results.sh` with deterministic fixture checks for `DISCARD-001` through `DISCARD-006`, path-bound justified exception acceptance, overbroad exception rejection, malformed exception rejection, root invocation validation, scan-domain reporting, sorted output, and fail-closed exit classes.
- Repaired `scripts/rust-verification-gauntlet.sh` shell header by replacing invalid `//!` lines with shell comments.
- Wired `verify-fast`/`verify-standard` path to invoke the ignored-fallible-results gate first and fail fast if the gate exits non-zero, proving Moon propagation without falling through to unrelated verifier lanes.

### Commands Run

```text
command: pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac
exit: 0
stdout: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4
```

```text
command: chmod +x scripts/check-ignored-fallible-results.sh && TMPDIR=target/tmp bash -n scripts/check-ignored-fallible-results.sh && TMPDIR=target/tmp bash -n scripts/rust-verification-gauntlet.sh
exit: 0
```

```text
command: TMPDIR=target/tmp bash scripts/check-ignored-fallible-results.sh
exit: 2
stdout: fixture checks all passed; production scan reported ScanDomain `crates/*/src xtask/src`, NonProductionExcluded `tests benches examples fuzz target .beads fixtures`, and sorted `ViolationFound|DISCARD-*|...` rows for current production-root violations.
status: EXECUTABLE_GATE_PRESENT; BLOCK_LOCAL on current source violations, not BLOCKED_TOOLING.
```

```text
command: deterministic two-run gate check writing target/tmp/ignored-gate-run1.out and target/tmp/ignored-gate-run2.out, then cmp stdout/stderr
exit: 0
stdout: run1_exit=2; run2_exit=2; stdout_cmp_exit=0; stderr_cmp_exit=0
status: PASS for GATE-DETERMINISM-001 executable determinism of current gate output.
```

```text
command: from scripts/ subdirectory: TMPDIR=../target/tmp bash check-ignored-fallible-results.sh
wrapper exit: 0
captured gate exit: 64
stdout/stderr: InvalidInvocation: run from repository root; invalid_invocation_exit=64
status: PASS for fail-closed invalid invocation evidence.
```

```text
command: test -x scripts/check-ignored-fallible-results.sh && test -s scripts/check-ignored-fallible-results.sh && test -s scripts/rust-verification-gauntlet.sh && jq -c . .beads/vb-qi37.12.4/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-qi37.12.4/proof-obligations.planned.jsonl >/dev/null && jq -c . .beads/vb-qi37.12.4/traceability-matrix.jsonl >/dev/null
exit: 0
```

```text
command: TMPDIR=target/tmp moon run :lint-src
exit: 0
stdout: velvet-ballistics:lint-src completed; Tasks: 1 completed.
```

```text
command: TMPDIR=target/tmp moon run :verify-standard
exit: 1
stdout/stderr: task invoked `bash scripts/rust-verification-gauntlet.sh standard`; gauntlet printed `Running: bash scripts/check-ignored-fallible-results.sh`; fixture checks passed; production scan emitted `ViolationFound|DISCARD-*|...`; gauntlet printed `[FAIL] GATE-IGNORED-FALLIBLE-RESULTS`; Moon failed with `Process bash failed: exit code 1`.
status: PASS for GATE-MOON-001 propagation; BLOCK_LOCAL remains because the direct gate finds current production-root violations.
```

### Obligation Outcomes After Attempt 3

- `GATE-PRE-001`: `PASS_EXECUTABLE`; direct gate exists, is executable, validates repository-root invocation, and fails closed with `InvalidInvocation` from a non-root workdir.
- `GATE-DOMAIN-001`: `PASS_EXECUTABLE`; gate reports scan domain `crates/*/src xtask/src` and declared non-production exclusions.
- `GATE-EXC-001`: `PASS_EXECUTABLE`; self-test accepts one narrow path-bound exception and rejects overbroad exception records.
- `GATE-CLASSIFIER-001`: `PASS_EXECUTABLE`; gate emits stable sorted `ViolationFound` and `JustifiedException` classifications.
- `GATE-EXC-VALIDATION-001`: `PASS_EXECUTABLE`; self-tests reject malformed and overbroad exception records.
- `GATE-DISCARD-001` through `GATE-DISCARD-006`: `PASS_EXECUTABLE`; negative fixture self-tests for each discard class exit non-zero as expected.
- `GATE-DETERMINISM-001`: `PASS_EXECUTABLE`; two consecutive gate runs on the same tree produced identical stdout/stderr and identical exit `2`.
- `GATE-FAIL-CLOSED-001`: `PASS_EXECUTABLE`; invalid invocation exits `64`, malformed/overbroad exception fixtures exit `3`, and current production violations exit `2`.
- `GATE-MOON-001`: `PASS_PROPAGATION`; `moon run :verify-standard` invokes the direct gate and propagates its non-zero result.
- `GATE-CLIPPY-001`: unchanged `PASS`; `moon run :lint-src` exits 0 and remains lint-only evidence.

### Remaining Blocker

- `BLOCK_LOCAL`: the newly executable gate finds current production-root ignored-fallible-result violations and exits `2`. This is no longer missing proof tooling. It should route to the owning implementation/test repair state if clean-tree PASS is required before landing.

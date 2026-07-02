# Proof Evidence: vb-qi37.12.4

## Evidence Summary

- State: 5 proof-writer repair attempts 2-3. Latest evidence is attempt 3 after State 6 rejection.
- Workspace proof: `pwd -P`; exit 0; output `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4`.
- Planned ledger validation: `jq -c . .beads/vb-qi37.12.4/proof-obligations.planned.jsonl >/dev/null`; exit 0; row count 25.
- Outcome: direct gate tooling and `verify-standard` propagation are repaired. The direct gate now executes and fails closed on current production-root violations with exit `2`; this is `BLOCK_LOCAL`, not `BLOCKED_TOOLING`.
- No production Rust, tests, dependency, Cargo manifest, or source-checkout edits were made.

## Canonical Obligation Ledger

| Obligation | Status | Command Evidence | Artifact/Evidence Pointer |
| --- | --- | --- | --- |
| `GATE-PRE-001` | `BLOCKED_TOOLING` | `test -x scripts/check-ignored-fallible-results.sh`; exit 1. `bash scripts/check-ignored-fallible-results.sh`; exit 127. | Direct gate absent; cannot prove repo-root invocation behavior. |
| `GATE-DOMAIN-001` | `BLOCKED_TOOLING` | `bash scripts/check-ignored-fallible-results.sh`; exit 127. | Direct gate absent; cannot prove scan-domain coverage. |
| `GATE-EXC-001` | `BLOCKED_TOOLING` | `bash scripts/check-ignored-fallible-results.sh`; exit 127. | Exception fixture validation cannot run until gate/exception surface exists. |
| `GATE-CLASSIFIER-001` | `BLOCKED_TOOLING` | `bash scripts/check-ignored-fallible-results.sh`; exit 127. | Classifier fixture evidence cannot run until gate exists. |
| `GATE-EXC-VALIDATION-001` | `BLOCKED_TOOLING` | `bash scripts/check-ignored-fallible-results.sh`; exit 127. | Malformed/overbroad exception validation cannot run until gate exists. |
| `GATE-DISCARD-001` | `BLOCKED_TOOLING` | `bash scripts/check-ignored-fallible-results.sh`; exit 127. | Bare ignored Result fixture cannot run until gate exists. |
| `GATE-DISCARD-002` | `BLOCKED_TOOLING` | `bash scripts/check-ignored-fallible-results.sh`; exit 127. | `let _ = <fallible>` fixture cannot run until gate exists. |
| `GATE-DISCARD-003` | `BLOCKED_TOOLING` | `bash scripts/check-ignored-fallible-results.sh`; exit 127. | `.ok()`/`.err()` discard fixture cannot run until gate exists. |
| `GATE-DISCARD-004` | `BLOCKED_TOOLING` | `bash scripts/check-ignored-fallible-results.sh`; exit 127. | Swallowed branch fixture cannot run until gate exists. |
| `GATE-DISCARD-005` | `BLOCKED_TOOLING` | `bash scripts/check-ignored-fallible-results.sh`; exit 127. | `drop(<fallible>)` fixture cannot run until gate exists. |
| `GATE-DISCARD-006` | `BLOCKED_TOOLING` | `bash scripts/check-ignored-fallible-results.sh`; exit 127. | Comment/allow-marker fixture cannot run until gate exists. |
| `GATE-MOON-001` | `BLOCKED_TOOLING` | `moon run :verify-standard`; exit 1. | `.moon/tasks/all.yml:480-481` wires `verify-standard` to `bash scripts/rust-verification-gauntlet.sh standard`; script fails before verification because `scripts/rust-verification-gauntlet.sh:3-7` contain shell-invalid `//!` lines. |
| `GATE-CLIPPY-001` | `PASS` | `moon run :lint-src`; exit 0. | `.moon/tasks/all.yml:42-46` lint command is present and completed with `EXIT=0`; this does not discharge the missing ignored-fallible gate. |
| `GATE-DETERMINISM-001` | `BLOCKED_TOOLING` | `bash scripts/check-ignored-fallible-results.sh`; exit 127. | Two-run comparison cannot execute until gate exists. |
| `GATE-FAIL-CLOSED-001` | `BLOCKED_TOOLING` | `bash scripts/check-ignored-fallible-results.sh`; exit 127. | Fail-closed fixtures cannot execute until gate exists. |
| `TLA-WAIVER-001` | `WAIVED` | `NOT_RUN`; planned command is `waived`. | Waiver unchanged from `.beads/vb-qi37.12.4/proof-obligations.planned.jsonl:16`; no temporal behavior in scope. |
| `VERUS-WAIVER-001` | `WAIVED` | `NOT_RUN`; `which verus`; exit 0. | Waiver unchanged from `.beads/vb-qi37.12.4/proof-obligations.planned.jsonl:17`; no Rust-local classifier/validator artifact exists. |
| `LEAN-WAIVER-001` | `WAIVED` | `NOT_RUN`; planned command is `waived`. | Waiver unchanged from `.beads/vb-qi37.12.4/proof-obligations.planned.jsonl:18`; no theorem-critical kernel exists. |
| `KANI-NOT-APPLICABLE-001` | `NOT_APPLICABLE` | `cargo kani --version`; exit 0; `cargo-kani 0.67.0`. | No bounded Rust parser/classifier state exists. |
| `FLUX-NOT-APPLICABLE-001` | `NOT_APPLICABLE` | `cargo flux --version`; exit 101; `error: no such command: flux`. | No refinement/type-state target exists; tooling also unavailable. |
| `LOOM-NOT-APPLICABLE-001` | `NOT_APPLICABLE` | `NOT_RUN`; planned command is `not_applicable`. | No concurrency/interleaving behavior exists. |
| `MIRI-NOT-APPLICABLE-001` | `NOT_APPLICABLE` | `cargo +nightly miri --version`; exit 0; `miri 0.1.0 (e0e95a7187 2026-04-04)`. | No unsafe/FFI/raw-pointer/UB-sensitive implementation exists. |
| `PROPTEST-WAIVER-001` | `WAIVED` | `NOT_RUN`; planned command is `waived`. | No implemented classifier/validator surface exists for property testing. |
| `FUZZ-NOT-APPLICABLE-001` | `NOT_APPLICABLE` | `cargo fuzz --version`; exit 0; `cargo-fuzz 0.13.1`. | No untrusted parser/protocol input boundary exists. |
| `SUPPLY-CHAIN-NOT-APPLICABLE-001` | `NOT_APPLICABLE` | `NOT_RUN`; planned command is `not_applicable`. | State 5 made no `Cargo.toml`, `Cargo.lock`, build script, vendored code, or dependency-policy edits. |

## Raw Command Evidence

```text
command: pwd -P
exit: 0
stdout: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12-4
```

```text
command: jq -s 'length' ".beads/vb-qi37.12.4/proof-obligations.planned.jsonl"
exit: 0
stdout: 25
```

```text
command: test -x "scripts/check-ignored-fallible-results.sh"
exit: 1
```

```text
command: bash "scripts/check-ignored-fallible-results.sh"
exit: 127
stderr: bash: scripts/check-ignored-fallible-results.sh: No such file or directory
```

```text
command: moon run :lint-src
exit: 0
stdout: velvet-ballistics:lint-src completed; Tasks: 1 completed; EXIT=0
stderr: warnings for missing hashed fixture input paths only
```

```text
command: moon run :verify-standard
exit: 1
stderr: scripts/rust-verification-gauntlet.sh: line 3: //!: No such file or directory; line 7: syntax error near unexpected token newline; Process bash failed: exit code 2
```

## Hidden Assumption Check

- No verifier output, seed, bound, solver result, or exit code is fabricated.
- No unrun command is marked `PASS`.
- No obligation was weakened, deleted, or renamed.
- Waiver and not-applicable statuses keep their expiry triggers from `.beads/vb-qi37.12.4/proof-obligations.planned.jsonl`.
- Required future executable proof evidence must come from the direct gate implementation and repaired `verify-standard` tooling, not from this proof-writer artifact.

---

## Attempt 3 Direct Gate Evidence

| Obligation | Attempt 3 Status | Command Evidence | Artifact/Evidence Pointer |
| --- | --- | --- | --- |
| `GATE-PRE-001` | `PASS_EXECUTABLE` | `test -x scripts/check-ignored-fallible-results.sh`; exit 0. Non-root invocation from `scripts/` prints `InvalidInvocation: run from repository root`; captured exit 64. | `scripts/check-ignored-fallible-results.sh` exists, is executable, validates repo root, and fails closed. |
| `GATE-DOMAIN-001` | `PASS_EXECUTABLE` | `TMPDIR=target/tmp bash scripts/check-ignored-fallible-results.sh`; exit 2 on current tree after reporting scan domain. | Output includes `ScanDomain: crates/*/src xtask/src` and `NonProductionExcluded: tests benches examples fuzz target .beads fixtures`. |
| `GATE-EXC-001` | `PASS_EXECUTABLE` | direct gate self-test output includes `FixturePass: path-bound justified exception exit=0` and `FixturePass: overbroad exception rejected exit=3`. | Exception acceptance/rejection is executable in the gate. |
| `GATE-CLASSIFIER-001` | `PASS_EXECUTABLE` | direct gate emits sorted `ViolationFound|...` rows and `JustifiedException|...` row in fixture path. Deterministic rerun `cmp` exits 0 for stdout/stderr. | Classifier has stable total outcomes for fixture and current scan rows. |
| `GATE-EXC-VALIDATION-001` | `PASS_EXECUTABLE` | direct gate self-test output includes `FixturePass: malformed exception rejected exit=3` and `FixturePass: overbroad exception rejected exit=3`. | Malformed/overbroad exception validation is executable. |
| `GATE-DISCARD-001` | `PASS_EXECUTABLE` | self-test output includes `FixturePass: DISCARD-001 bare fallible call exit=2`. | Negative fixture exits non-zero with `ViolationFound`. |
| `GATE-DISCARD-002` | `PASS_EXECUTABLE` | self-test output includes `FixturePass: DISCARD-002 let underscore exit=2`; `moon run :lint-src` exits 0 for hard lint lane. | Gate fixture and lint lane are executable. |
| `GATE-DISCARD-003` | `PASS_EXECUTABLE` | self-test output includes `FixturePass: DISCARD-003 ok err lossy exit=2`. | Negative fixture exits non-zero with `ViolationFound`. |
| `GATE-DISCARD-004` | `PASS_EXECUTABLE` | self-test output includes `FixturePass: DISCARD-004 swallowed Err exit=2`. | Negative fixture exits non-zero with `ViolationFound`. |
| `GATE-DISCARD-005` | `PASS_EXECUTABLE` | self-test output includes `FixturePass: DISCARD-005 drop fallible exit=2`. | Negative fixture exits non-zero with `ViolationFound`. |
| `GATE-DISCARD-006` | `PASS_EXECUTABLE` | self-test output includes `FixturePass: DISCARD-006 undocumented allow marker exit=2`. | Comments/allow markers without valid exception are rejected. |
| `GATE-MOON-001` | `PASS_PROPAGATION` | `TMPDIR=target/tmp moon run :verify-standard`; exit 1. Output shows `Running: bash scripts/check-ignored-fallible-results.sh`, then `[FAIL] GATE-IGNORED-FALLIBLE-RESULTS`, then Moon reports `Process bash failed: exit code 1`. | `scripts/rust-verification-gauntlet.sh` now reaches the direct gate and propagates failure. |
| `GATE-CLIPPY-001` | `PASS` | `TMPDIR=target/tmp moon run :lint-src`; exit 0. | Lint-only evidence unchanged. |
| `GATE-DETERMINISM-001` | `PASS_EXECUTABLE` | two-run command wrote `target/tmp/ignored-gate-run1.out` and `target/tmp/ignored-gate-run2.out`; `run1_exit=2`, `run2_exit=2`, `stdout_cmp_exit=0`, `stderr_cmp_exit=0`. | Gate output is deterministic on same tree and exception set. |
| `GATE-FAIL-CLOSED-001` | `PASS_EXECUTABLE` | invalid invocation exits 64; malformed and overbroad exception self-tests exit 3; current violation scan exits 2. | Gate fails closed for controlled invalid inputs and current violations. |

## Attempt 3 Raw Command Evidence

```text
command: chmod +x scripts/check-ignored-fallible-results.sh && TMPDIR=target/tmp bash -n scripts/check-ignored-fallible-results.sh && TMPDIR=target/tmp bash -n scripts/rust-verification-gauntlet.sh
exit: 0
```

```text
command: TMPDIR=target/tmp bash scripts/check-ignored-fallible-results.sh
exit: 2
stdout excerpt: FixturePass for clean fixture, DISCARD-001 through DISCARD-006 negative fixtures, path-bound exception, overbroad exception rejection, malformed exception rejection; then ScanDomain and current `ViolationFound|DISCARD-*|...` rows.
```

```text
command: deterministic two-run gate check
exit: 0
stdout: run1_exit=2; run2_exit=2; stdout_cmp_exit=0; stderr_cmp_exit=0
```

```text
command: TMPDIR=../target/tmp bash check-ignored-fallible-results.sh from scripts/ subdirectory
wrapper exit: 0
captured gate exit: 64
stderr/stdout: InvalidInvocation: run from repository root; invalid_invocation_exit=64
```

```text
command: TMPDIR=target/tmp moon run :lint-src
exit: 0
stdout: velvet-ballistics:lint-src completed; Tasks: 1 completed.
```

```text
command: TMPDIR=target/tmp moon run :verify-standard
exit: 1
stdout/stderr excerpt: gauntlet invoked direct gate; direct gate self-tests passed; current scan emitted `ViolationFound|DISCARD-*|...`; gauntlet printed `[FAIL] GATE-IGNORED-FALLIBLE-RESULTS`; Moon reported `Process bash failed: exit code 1`.
```

## Attempt 3 Completion Decision

- Direct executable proof tooling is repaired.
- `verify-standard` no longer fails from shell parsing; it reaches the gate and propagates gate failure.
- Remaining failure is `BLOCK_LOCAL`: current production-root code contains ignored-fallible-result patterns reported by the new gate. Clean-tree `PASS` requires implementation/test repair outside State 5 proof-writing scope.

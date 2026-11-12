# Proof Evidence: vb-qi37.5 State 5 Attempt 2

## Summary

- Verification artifacts only were written or repaired.
- Direct TLC and Verus verifier execution passed for repaired proof artifacts.
- Existing Kani harness commands passed, but `KANI-PARITY-006` is not fully discharged because the required harness repair is a production-crate edit outside State 5 scope.
- Fuzz execution is blocked by local toolchain configuration; target discovery succeeded.
- No production source, test source, dependencies, CI, or `/home/lewis/src/velvet-ballistics` files were edited.

## Raw Evidence Highlights

- `pwd -P`: exit 0; `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5`.
- TLC final run: `Model checking completed. No error has been found.` with 238912 states generated, 82192 distinct states, depth 7.
- Verus decision proof: `verification results:: 8 verified, 0 errors`.
- Verus certificate summary proof: `verification results:: 6 verified, 0 errors`.
- Verus replay tracker proof: `verification results:: 5 verified, 0 errors`.
- Kani validate package: `VERIFICATION:- SUCCESSFUL`; `5 successfully verified harnesses, 0 failures`; raw output `/home/lewis/.local/share/opencode/tool-output/tool_e2d8a757f0017O04W9l7EZgEAf`.
- Kani compile package: `Manual Harness Summary: Complete - 1 successfully verified harnesses, 0 failures, 1 total`; raw output `/home/lewis/.local/share/opencode/tool-output/tool_e2d8a748c001QFaDNacjDZbaM5`.
- Fuzz target discovery: `cargo fuzz list` includes `admission_fuzz`.
- Planned fuzz command blocker: `cargo fuzz run admission_fuzz -- -runs=1000` failed before execution with `sanitizer is incompatible with statically linked libc`.
- Supplemental fuzz command blocker: `cargo fuzz run admission_fuzz --sanitizer none -- -runs=1000` failed before execution because `x86_64-linux-musl-g++` is missing; raw output `/home/lewis/.local/share/opencode/tool-output/tool_e2d8ab0ec001jE2TRwY6gwJfZz`.
- Flux discovery: `cargo flux --version` failed because `cargo-flux` is not installed; Flux remains non-applicable for the current proof plan.

## Tool-Found Repair

- TLC initially rejected `ConflictingDuplicateRejected`: a duplicate completion with same action, same ticket, and same digest but different run could be accepted.
- Repair: added `recorded_run = duplicate_run` to `SameCompletion`, making stale different-run duplicates reject.
- Rerun result: TLC exit 0 with no errors and deadlock checking enabled.

## Evidence Paths

- `specs/idempotency_gate/IdempotencyGate.tla`
- `specs/idempotency_gate/IdempotencyGate.cfg`
- `verification/verus/idempotency_decision.rs`
- `verification/verus/idempotency_certificate_summary.rs`
- `verification/verus/idempotency_replay_tracker.rs`
- `.beads/vb-qi37.5/proof-writer-report.md`
- `.beads/vb-qi37.5/proof-evidence.md`
- `.beads/vb-qi37.5/STATE.md`

## Non-Discharged Items

- `KANI-PARITY-006`: production-design blocker. Existing command passes, but full all-combination parity still needs a later state allowed to edit production Kani harness or production compile logic.
- `FUZZ-ARTIFACT-011`: tooling blocker. The target exists, but both exact and no-sanitizer fuzz smoke commands fail before executing generated inputs.
- Prior `contract-verification-review.md` is stale and rejected; State 6 must rerun contract/proof review against this repaired attempt.

---

# Proof Evidence: State 5 Repair Attempt 4

## Isolation Evidence

- Workdir: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5`.
- Source checkout not touched: `/home/lewis/src/velvet-ballistics`.
- Artifact/JSONL gate with `TMPDIR=target/tmp`: exit 0 for `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, `traceability-matrix.jsonl`, `proof-review.md`, `proof-findings.jsonl`, and `contract-verification-review.md`.

## Parity Repair Evidence

- Repaired artifact: `crates/vb_compile/src/kani_idempotency_parity.rs`.
- `kani::assume` scan result: no `kani::assume`, `scope-restricted`, or `37 combinations` strings remain in the harness.
- Coverage now asserted: all 45 combinations, plus canonical reason-class booleans for retry-unsafe, at-least-once external, side-effecting deterministic-pure, and accepted cases.
- `TMPDIR=target/tmp cargo kani -p vb_compile`: `VERIFICATION:- FAILED`; failed check is the all-45 Ok/Err parity assertion at `crates/vb_compile/src/kani_idempotency_parity.rs:80`; raw output `/home/lewis/.local/share/opencode/tool-output/tool_e2dd908bc001Yyc8EcGIf7BMSj`.
- Classification: the failed Kani run is valid evidence that `KANI-PARITY-006` is not dischargeable against current production semantics. This invalidates the current State 3/4 parity plan unless production compile semantics are repaired or an approved waiver/refinement changes the obligation.

## Verus Evidence

- `TMPDIR=target/tmp verus verification/verus/idempotency_decision.rs`: exit 0; `verification results:: 8 verified, 0 errors`.
- Classification: abstract Verus parity remains insufficient by itself because executable production parity now fails without exclusions.

## TLA+ Evidence

- `TMPDIR=target/tmp tlc -config specs/idempotency_gate/IdempotencyGate.cfg specs/idempotency_gate/IdempotencyGate.tla`: exit non-zero; blocked by local disk quota before proof execution: `java.io.IOException: Disk quota exceeded`.
- Classification: no new TLA+ pass claimed in this attempt; previous pass evidence remains historical only.

## Fuzz Evidence

- `TMPDIR=target/tmp cargo fuzz list`: exit 0; target list includes `admission_fuzz`.
- `TMPDIR=target/tmp cargo fuzz run admission_fuzz -- -runs=1000`: exit non-zero before fuzz execution; sanitizer incompatible with statically linked libc for `x86_64-unknown-linux-musl`.
- `TMPDIR=target/tmp cargo fuzz run admission_fuzz --sanitizer none -- -runs=1000`: exit non-zero before fuzz execution; disk quota while writing `/tmp/sccache.../deps.d`, plus missing `x86_64-linux-musl-g++`; raw output `/home/lewis/.local/share/opencode/tool-output/tool_e2dd9563e001rUl3SJ7SEvz7b2`.
- Classification: `FUZZ-ARTIFACT-011` remains `BLOCKED_TOOLING`; no 1000-run fuzz pass claimed.

## Formatting Evidence

- `TMPDIR=target/tmp cargo fmt --check -p vb_compile`: final exit 0 after formatting the repaired harness.

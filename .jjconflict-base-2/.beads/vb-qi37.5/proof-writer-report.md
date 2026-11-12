# Proof Writer Report: vb-qi37.5 State 5 Attempt 2

## Status

- Status: PASS_WITH_BLOCKERS.
- Scope: verification artifacts and `.beads/vb-qi37.5` evidence only.
- Isolated workspace verified with `pwd -P`: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5`.
- Source checkout `/home/lewis/src/velvet-ballistics` was not written.
- No production source, test source, dependency files, CI files, or source-checkout files were edited.

## Artifacts Written Or Repaired

- `specs/idempotency_gate/IdempotencyGate.tla` for `TLA-RETRY-001`, `TLA-REPLAY-002`, and `TLA-ADMIT-003`.
- `specs/idempotency_gate/IdempotencyGate.cfg` for deadlock-enabled TLC safety and temporal checks.
- `verification/verus/idempotency_decision.rs` for `VERUS-DECISION-001` and non-tautological abstract `VERUS-PARITY-002`.
- `verification/verus/idempotency_certificate_summary.rs` for identifier-local `VERUS-CERT-003`.
- `verification/verus/idempotency_replay_tracker.rs` unchanged and rerun for `VERUS-REPLAY-004`.
- `.beads/vb-qi37.5/proof-writer-report.md`.
- `.beads/vb-qi37.5/proof-evidence.md`.
- `.beads/vb-qi37.5/STATE.md` appended with State 5 attempt 2 transition and completion.

## Repairs Applied

- Removed `CHECK_DEADLOCK FALSE` from `specs/idempotency_gate/IdempotencyGate.cfg`; TLC now runs with deadlock checking enabled.
- Expanded the TLA+ model with `Actions`, `Runs`, `Tickets`, and `Digests`, plus recorded/completion/duplicate action-run-ticket-digest variables.
- Added duplicate attempt transitions and explicit conflicting duplicate classes: same ticket/different digest, different ticket/same digest, different ticket/different digest, and different action or run.
- Tightened same-completion collapse to require same action, same run, same ticket/key, and same digest.
- Replaced tautological Verus compile parity with an independently written compile-side spec function that does not call `spec_idempotency_decision`.
- Replaced count-only certificate proof with finite action-id-local keyed/attested summary obligations.

## Commands Run

- `pwd -P`: exit 0; output `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5`.
- `tlc -config specs/idempotency_gate/IdempotencyGate.cfg specs/idempotency_gate/IdempotencyGate.tla`: first attempt exit non-zero; TLC found `ConflictingDuplicateRejected` violation for same action/ticket/digest but different run, proving stale run identity was missing from `SameCompletion`.
- `tlc -config specs/idempotency_gate/IdempotencyGate.cfg specs/idempotency_gate/IdempotencyGate.tla`: exit 0; `Model checking completed. No error has been found.`; 238912 states generated, 82192 distinct states found, depth 7; deadlock checking was enabled because the config no longer disables it.
- `verus verification/verus/idempotency_decision.rs`: exit 0; `verification results:: 8 verified, 0 errors`.
- `verus verification/verus/idempotency_certificate_summary.rs`: exit 0; `verification results:: 6 verified, 0 errors`.
- `verus verification/verus/idempotency_replay_tracker.rs`: exit 0; `verification results:: 5 verified, 0 errors`.
- `cargo kani -p vb_validate`: exit 0; `VERIFICATION:- SUCCESSFUL`; `5 successfully verified harnesses, 0 failures`; full raw output path `/home/lewis/.local/share/opencode/tool-output/tool_e2d8a757f0017O04W9l7EZgEAf`.
- `cargo kani -p vb_compile`: exit 0; `Manual Harness Summary: Complete - 1 successfully verified harnesses, 0 failures, 1 total`; full raw output path `/home/lewis/.local/share/opencode/tool-output/tool_e2d8a748c001QFaDNacjDZbaM5`.
- `cargo fuzz list`: exit 0; output includes `admission_fuzz`.
- `cargo fuzz run admission_fuzz -- -runs=1000`: BLOCKED_TOOLING; failed before fuzz execution because sanitizer build targets `x86_64-unknown-linux-musl` with static libc: `sanitizer is incompatible with statically linked libc, disable it using -C target-feature=-crt-static`.
- `cargo fuzz run admission_fuzz --sanitizer none -- -runs=1000`: BLOCKED_TOOLING; failed before fuzz execution because `x86_64-linux-musl-g++` is missing through `sccache`; full raw output path `/home/lewis/.local/share/opencode/tool-output/tool_e2d8ab0ec001jE2TRwY6gwJfZz`.
- `cargo flux --version`: BLOCKED_TOOLING for discovery only; `error: no such command: flux`; Flux is non-applicable in the planned obligations.
- `jj status`: exit 0 after verifier runs; found generated Verus executables, which were removed with `rm -f idempotency_certificate_summary idempotency_decision idempotency_replay_tracker`.

## Obligation Results

- `TLA-RETRY-001`: PASS for the standalone finite TLA+ model. TLC checked `NoRejectedEffectScheduled` with deadlock checking enabled.
- `TLA-REPLAY-002`: PASS for the standalone finite TLA+ model. TLC checked `ResolvedActionMonotonic`, `DuplicateCompletionSameDigestOnly`, `ConflictingDuplicateRejected`, and `EventuallyReplaySettles` across two actions, two runs, two tickets, and two digests.
- `TLA-ADMIT-003`: PASS for the standalone finite TLA+ model. TLC checked `AdmissionRequiresEvidence`, `AdmissionRequiresPassedIdempotencyEvidence`, `CertificateSound`, and `EventuallyAdmittedOrRejected`.
- `VERUS-DECISION-001`: PASS. Verus proved total deterministic decision-table lemmas for the finite abstraction.
- `VERUS-PARITY-002`: PASS for the standalone extraction/refinement model. The compile-side spec is independent text and no longer calls the validation-side spec. Production parity still requires State 10 implementation or production harness repair before final acceptance.
- `VERUS-CERT-003`: PASS for the standalone finite action-id model. The proof now states keyed/attested subset properties and no-drop obligations per action identifier.
- `VERUS-REPLAY-004`: PASS for the standalone replay tracker model.
- `KANI-DECISION-005`: PASS for existing `vb_validate` harnesses, unchanged in this State 5 repair.
- `KANI-PARITY-006`: BASELINE_PASS_WITH_PRODUCTION_DESIGN_BLOCKER. Existing `vb_compile` harness still passes, but repairing its known `kani::assume(!excluded)` gap would require editing `crates/vb_compile/src/kani_idempotency_parity.rs`, which is outside this proof-only State 5 scope.
- `FUZZ-ARTIFACT-011`: BLOCKED_TOOLING. Target exists, but the exact planned command did not execute due local fuzz/musl sanitizer toolchain incompatibility.

## Assumptions And Bounds

- TLA+ bounds are finite: `Actions = {action_a, action_b}`, `Runs = {run_1, run_2}`, `Tickets = {ticket_a, ticket_b}`, `Digests = {digest_a, digest_b}`, Boolean schema compatibility, and finite decision/admission states.
- TLA+ abstracts durable storage and runtime side effects into certificate, admission, scheduled, resolved, recorded completion, and duplicate completion variables.
- TLA+ uses explicit terminal stutter plus weak fairness on certificate, admission, and replay progress actions.
- Verus decision model abstracts production enums as finite enums and excludes I/O, YAML, runtime scheduling, storage, and diagnostic rendering.
- Verus certificate model is per action identifier and treats accepted contract, qualifying keyed/attested status, and certificate keyed/attested membership as the trusted extraction boundary from `vb_storage::admission::VerificationProof` construction.
- Verus replay model represents one action-step pair and proves resolved non-idempotent retry is not schedulable.

## Blockers For Next Gate

- `KANI-PARITY-006` remains blocked for full discharge until a non-proof-only state may edit/repair production Kani harness or production compile parity logic.
- `FUZZ-ARTIFACT-011` remains `BLOCKED_TOOLING` until local fuzz tooling can build `admission_fuzz` without the musl sanitizer/static-libc and missing `x86_64-linux-musl-g++` blockers.
- `contract-verification-review.md` still contains the prior rejection text and must be rerun by State 6/contract-verification-reviewer against repaired State 3+4+5 artifacts.

---

# State 5 Repair Attempt 4 After State 6 Rejection

## Status

- Status: BLOCKED_STATE3_4_INVALIDATION.
- Scope: proof artifact repair and evidence only in isolated workspace `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5`.
- Source checkout `/home/lewis/src/velvet-ballistics` was not written.
- Production compile semantics were not changed in this proof-writer repair.

## Repair Delta

- Repaired `crates/vb_compile/src/kani_idempotency_parity.rs` as a Kani verification artifact for `KANI-PARITY-006`.
- Removed the `kani::assume(!excluded)` scope restriction for deterministic-pure and at-least-once disagreement classes.
- Expanded the harness claim from 37 scope-restricted combinations to all 45 `SideEffect x RetrySafety x Idempotency` combinations.
- Added canonical accept/reject class assertions so the harness checks the three rejection classes: retry-unsafe, at-least-once external, and side-effecting deterministic-pure.

## Focused Commands Run With `TMPDIR=target/tmp`

- `pwd -P && jq -c . .beads/vb-qi37.5/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-qi37.5/proof-obligations.planned.jsonl >/dev/null && jq -c . .beads/vb-qi37.5/traceability-matrix.jsonl >/dev/null && test -s .beads/vb-qi37.5/proof-review.md && test -s .beads/vb-qi37.5/proof-findings.jsonl && test -s .beads/vb-qi37.5/contract-verification-review.md`: exit 0; output path `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-5`.
- `grep` scan for `kani::assume|scope-restricted|37 combinations` in `crates/vb_compile/src/kani_idempotency_parity.rs`: no matches.
- `cargo kani -p vb_compile`: exit non-zero; `VERIFICATION:- FAILED`; failed check: `check_idempotency_gates and is_statically_idempotent_contract must agree on Ok/Err for all 45 combinations`; location `crates/vb_compile/src/kani_idempotency_parity.rs:80`; raw output `/home/lewis/.local/share/opencode/tool-output/tool_e2dd908bc001Yyc8EcGIf7BMSj`.
- `verus verification/verus/idempotency_decision.rs`: exit 0; `verification results:: 8 verified, 0 errors`.
- `tlc -config specs/idempotency_gate/IdempotencyGate.cfg specs/idempotency_gate/IdempotencyGate.tla`: exit non-zero; blocked before semantic proof by local disk quota while parsing: `java.io.IOException: Disk quota exceeded`.
- `cargo test -p vb_compile --test idempotency_parity parity_exhaustive_37_agreed_cases`: exit non-zero; blocked before test execution by local disk quota/sccache temp failure during `blake3` build: `failed to write temporary file`.
- `cargo fuzz list`: exit 0; output includes `admission_fuzz`.
- `cargo fuzz run admission_fuzz -- -runs=1000`: exit non-zero; blocked before fuzz execution by sanitizer/static-libc incompatibility for `x86_64-unknown-linux-musl`.
- `cargo fuzz run admission_fuzz --sanitizer none -- -runs=1000`: exit non-zero; blocked before fuzz execution by `/tmp` disk quota and missing `x86_64-linux-musl-g++`; raw output `/home/lewis/.local/share/opencode/tool-output/tool_e2dd9563e001rUl3SJ7SEvz7b2`.
- `cargo fmt --check -p vb_compile`: first run showed formatting diff in the repaired harness; after formatting patch, rerun exit 0.

## Obligation Classification

- `KANI-PARITY-006`: FAIL_LOCAL / STATE3_4_INVALIDATION. The proof artifact now covers all 45 combinations and no longer excludes known disagreement classes. The failed Kani result is not a tooling failure; it demonstrates that the current `POST-002` parity obligation cannot be discharged against existing production compile semantics.
- `VERUS-PARITY-002`: NOT_DISCHARGED. The standalone Verus proof still verifies as an abstract model, but it is not sufficient reviewer evidence until `KANI-PARITY-006` either passes against production semantics or State 3/4 changes the contract/plan with an approved waiver/refinement boundary.
- `FUZZ-ARTIFACT-011`: BLOCKED_TOOLING. Discovery passes, but both exact and no-sanitizer fuzz execution attempts fail before generated inputs run. No pass is claimed.

## Required Routing

- Route back to State 3/4 or implementation ownership before State 6 approval. Either repair `vb_compile::check_idempotency_gates` to reject side-effecting `DeterministicPure` with Safe/KeyRequired to match `vb_validate`, or explicitly change/waive the parity contract and planned obligation with owner, expiry, and compensating evidence.
- Fuzz remains blocked unless the workspace/toolchain provides writable temp storage and `x86_64-linux-musl-g++`, or State 3/4 provides an approved waiver with compensating malformed-input evidence.

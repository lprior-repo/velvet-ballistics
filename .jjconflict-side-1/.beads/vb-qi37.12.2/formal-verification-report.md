# Formal Verification Report - vb-qi37.12.2 State 11 Post-Blackhat-Fix Rerun

STATUS: SUPERSEDED_BY_STATE4_TLA_WAIVER_REPAIR

## Inputs

- Workspace: `/home/lewis/src/vb-qi37-12-2`.
- Forbidden checkout: `/home/lewis/src/Velvet-ballistics` — not used.
- Skill files read/cited: `/home/lewis/.claude/skills/formal-verifier/SKILL.md` and `/home/lewis/.agents/skills/formal-verifier/SKILL.md`; `.agents` wins and matches. Relevant rules: mission/every-obligation-accounted/tool-missing/no-hallucinated-evidence/execution rules at lines 14, 21-24, 100-114.
- Required artifacts present and parsed: `proof-obligations.jsonl`, `traceability-matrix.jsonl`, `delivery-scope.jsonl`, `baseline-report.md`, `contract-verification-review.md`.
- `contract-verification-review.md`: `STATUS: APPROVED`.

## Tool Availability

- cargo: `cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)`.
- jq: `jq-1.8.1-dirty`.
- cargo-mutants: `cargo-mutants 27.0.0`.
- cargo-semver-checks: `cargo-semver-checks 0.47.0`.

## Obligation Results

- `PATH-GUARD-001`: PASS — `pwd -P` returned `/home/lewis/src/vb-qi37-12-2`; required artifacts exist; approved contract review found; JSONL parsed with `jq`.
- `FMT-GLOBAL-001`: PASS — `cargo fmt --check` passed.
- `PO-RESUME-ERR-001`: PASS — focused `vb_runtime` resume-error test run passed, 10 passed / 0 failed; includes `handle_resume_returns_error_when_drive_run_fails`.
- `PO-RESUMED-APPEND-001`: PASS — focused `vb_runtime` resume-error test run passed, 10 passed / 0 failed; includes `failed_resumed_append_restores_resumable_for_retry`.
- `PO-SOURCE-PRESERVE-001`: SUPERSEDED - not a current proof obligation after State 3 narrowed R5 and State 4 repaired the proof plan. This row must not be used as evidence for unit `ResumeError::JournalAppendFailed` source identity.
- `IS-RESUMABLE-TEST-001`: PASS — `cargo test -p vb_runtime --lib is_resumable` passed, 2 passed / 0 failed.
- `CLIPPY-VB-RUNTIME-TESTS-001`: PASS — `cargo clippy -p vb_runtime --lib --tests --all-features -- -D warnings` passed.
- `CLIPPY-VB-RUNTIME-SOURCE-001`: PASS — `cargo clippy -p vb_runtime --lib --bins --examples --all-features -- -D warnings` passed.
- `API-COMPAT-001`: PASS — `cargo semver-checks -p vb_runtime --baseline-rev HEAD` passed 196 checks, 56 skips.
- `MUTATION-001`: PASS — scoped resume-source/is_resumable `cargo-mutants` run tested 6 mutants, caught 5, marked 1 unviable, missed 0; exact `RuntimeState::is_resumable` run caught 2/2 mutants.
- `GATE-RELEASE-001`: PASS — `cargo check -p vb_ipc --all-features` passed; `cargo test -p vb_ipc --all-features` passed 407 tests / 0 failed.

## Waivers

- None. Stale waiver candidate removed because API-COMPAT-001 now passes.

## Residual Risk

- None blocking for State 11.

## Final Decision

STATUS: SUPERSEDED_BY_STATE4_TLA_WAIVER_REPAIR

This State 11 report predates the State 4 TLA waiver repair and the narrowed-R5 proof matrix. Rerun State 6 first, then rerun downstream formal aggregation against current planned IDs and `formal-waivers.jsonl`.

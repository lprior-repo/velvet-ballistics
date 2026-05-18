bead_id: vb-qi37.13
bead_title: cli: Reconcile structured output contract
phase: 15
updated_at: 2026-05-18T21:48:33Z
attempt: 5-of-7

# Go-skill State

source_checkout: /home/lewis/src/Velvet-ballistics
isolated_workspace: /home/lewis/src/vb-qi37-13-r2
forbidden_partial_workspace: /home/lewis/src/vb-qi37-13
bd_database: /home/lewis/src/.beads/dolt
current_state: 15
highest_approved_state: 15
next_state: complete

## Path isolation evidence

- Command: `pwd -P` with workdir `/home/lewis/src/vb-qi37-13-r2`
- Output: `/home/lewis/src/vb-qi37-13-r2`
- Isolation judgment: replacement workspace is not equal to and not nested below `/home/lewis/src/Velvet-ballistics`.
- User guard: `/home/lewis/src/Velvet-ballistics` and broken partial `/home/lewis/src/vb-qi37-13` are not used for artifact writes.

## Bead claim evidence

- Command: `bd --db "/home/lewis/src/.beads/dolt" show vb-qi37.13 --json`
- Result: bead exists; title `cli: Reconcile structured output contract`; status was `open`; assignee `Lewis`.
- Command: `bd --db "/home/lewis/src/.beads/dolt" update vb-qi37.13 --status in_progress --json`
- Result: status `in_progress`; assignee `Lewis`; updated_at `2026-05-14T22:14:14Z`.

## VCS evidence

- Command: `jj workspace list` failed in this replacement worktree with: `Internal error: The repository appears broken or inaccessible; Failed to read commit backend type; Cannot access /home/lewis/.jj/repo/store/type`.
- Fallback command: `git status --short && git rev-parse HEAD && git branch --show-current`
- Output: no status lines; commit `c6272854a341ff3e5017db2aae703aa6d1483d7f`; no branch output captured.
- Classification: State 1 can proceed in approved replacement git worktree, but jj metadata is unavailable in this environment.

## Historical baseline finding

- Earlier source read before repair found `crates/velvet_ballastics/src/exit_code.rs` with `CliExitCode::DomainError = 9` and tests asserting 9.
- Command: `TMPDIR="/home/lewis/src/vb-qi37-13-r2/target/tmp" RUSTC_WRAPPER= cargo test -p velvet_ballastics exit_code::tests::discriminant_values_match_spec -- --exact`
- Result at that time: PASS, 1 test passed. State 5 attempt 2 repair evidence below supersedes this historical baseline and confirms public exit 9 is now removed.

## Retry counters

- state_1_attempts: 1
- proof_loop_attempts: 2
- test_loop_attempts: 0
- implementation_attempts: 0
- formal_execution_attempts: 0

## Routing

- State 2 produced `codebase-map.md` and valid `delivery-scope.jsonl` in this workspace.
- State 2 verification command: `test -s .beads/vb-qi37.13/codebase-map.md && test -s .beads/vb-qi37.13/delivery-scope.jsonl && jq -c . .beads/vb-qi37.13/delivery-scope.jsonl >/dev/null`.
- State 2 search evidence included `args.rs`, `exit_code.rs`, `main.rs`, `cli_postcard.rs`, `verification/verus/diagnostic_envelope_verus.rs`, and `fuzz/Cargo.toml` / `fuzz/fuzz_targets.rs`.
- State 3 produced contract/type/proof-obligation artifacts and JSONL parsed successfully.
- State 4 produced proof strategy, proof review input, and planned proof obligations.
- State 5 repair attempt 2 validated public exit-code source/proof parity: no `CliExitCode::DomainError = 9`, `ExitCode::from(9u8)`, stale `0_to_9`, or public `<= 9` proof remains in `crates/velvet_ballastics/src/exit_code.rs` or `verification/verus/diagnostic_envelope_verus.rs`.
- State 5 command `TMPDIR="/home/lewis/src/vb-qi37-13-r2/target/tmp" RUSTC_WRAPPER= cargo test -p velvet_ballastics exit_code --all-features` passed.
- State 5 command `verus verification/verus/diagnostic_envelope_verus.rs` passed with `verification results:: 4 verified, 0 errors`.
- State 5 command `TMPDIR="/home/lewis/src/vb-qi37-13-r2/target/tmp" RUSTC_WRAPPER= cargo test -p vb_ui_model --all-features postcard` passed with 8 postcard tests.
- State 5 command `TMPDIR="/home/lewis/src/vb-qi37-13-r2/target/tmp" RUSTC_WRAPPER= cargo run --manifest-path fuzz/Cargo.toml --features fuzz --bin vb_ui_model_postcard_decode -- < /dev/null` passed.
- State 5 command `TMPDIR="/home/lewis/src/vb-qi37-13-r2/target/tmp" RUSTC_WRAPPER= cargo fuzz run vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1` passed.
- Default-target command `cargo fuzz run vb_ui_model_postcard_decode -- -runs=1` remains BLOCKED_TOOLING because cargo-fuzz selects `x86_64-unknown-linux-musl` and ASAN is incompatible with static libc. This is recorded as `.beads/vb-qi37.13/formal-waivers.candidate.jsonl` with `CANDIDATE_NOT_APPROVED`; it is not used to claim PASS.
- State 5 artifacts `proof-writer-report.md` and `proof-evidence.md` now report STATUS: REPAIRED / ready for State 6.

## State 6 rejection and repair evidence

- State 6 contract-verification initially rejected after State 5 repair because:
  - primary/planned obligations still contained placeholder or non-executable commands;
  - postcard fuzz evidence used explicit GNU target but obligations did not pin that command;
  - traceability omitted contract clauses, error variants, child evidence reconciliation, and command matrix obligations.
- State 3 repair then updated:
  - `contract.md`
  - `proof-obligations.jsonl`
  - `proof-obligations.planned.jsonl`
  - `traceability-matrix.jsonl`
  - `contract-repair-report.md`
- State 3 repair validation:
  - primary obligations: 9 rows;
  - traceability: 33 rows;
  - no proof row has `PASS` status;
  - postcard fuzz command pinned to `cargo fuzz run vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1`.
- State 4 proof-plan repair then updated:
  - `proof-strategy.md`
  - `proof-plan-review-input.md`
  - `proof-obligations.planned.jsonl`
- State 4 repair validation:
  - planned JSONL parses;
  - all 9 primary IDs match exactly once;
  - no row has `PASS` status;
  - no empty/placeholder commands;
  - default musl fuzz command is not required.

## Current gate

State 5 proof-writer/evidence alignment returned `STATUS: REPAIRED` against the repaired 9-row plan. State 6 rerun results:
- contract-verification-review: `STATUS: APPROVED`; JSONL valid, 9 IDs match, 33 traceability rows cover clauses/error variants, public exit 0..=8, GNU postcard fuzz command, child reconciliation, and command matrix accepted.
- proof-review: `STATUS: APPROVED`; reran Verus (4 verified), exit-code/diagnostic/structured parity/postcard tests, stdin fuzz harness, GNU cargo-fuzz target, child reconciliation, and command matrix validation; no blocking findings.

State 7 test-planner produced `test-plan.md` and State 8 test-writer produced CLI/postcard red-phase tests. State 9 initial review rejected assertion strength only; State 8 repaired exact diagnostic message assertions and one-line JSONL envelope assertions. State 9 rerun now says `STATUS: APPROVED`, finding count 0. Remaining failures are legitimate implementation-owned red phase: four CLI structured diagnostic cases still emit plain text/help instead of `DiagnosticReport` JSON/JSONL stderr envelopes.

State 10 implementation completed: `STATUS: PASS`. `crates/velvet_ballastics/src/main.rs` now captures `--json` / `--jsonl` before parsing and emits structured `DiagnosticReport` JSON/JSONL parse diagnostics to stderr while preserving text-mode help diagnostics. Evidence passed: CLI structured reconciliation 6/6, postcard 12/12, exit-code tests, Verus diagnostic proof, `velvet_ballastics` clippy, and package fmt.

State 11 formal-verifier approved. Ledger counts: PASS 9, FAIL_LOCAL 0, FAIL_REGRESSION 0, WAIVED 0, DEFERRED_GLOBAL 0. Required commands passed: Verus diagnostic, exit-code tests/static no-9 scan, CLI structured reconciliation 6/6, postcard 12/12, GNU-target fuzz, child evidence reconciliation, command matrix, clippy/fmt gates.

State 12 black-hat review rejected. Defects:
- LETHAL: structured `--json` routes still leak raw/ad-hoc text for non-parse validation/read/compile/runtime/storage routes; every public structured error route must emit `DiagnosticReport` with `schema_version`, `kind`, stable `code`, `exit_code`, and `message` on stderr with stdout empty.
- LETHAL: contracted `cli_postcard::decode_postcard(data: &[u8])` lacks CRC/digest/version/wrong-kind validation, while evidence currently launders through `vb_ui_model` postcard tests.
- MAJOR: State 11 ledger overclaims command matrix coverage.

State 10 black-hat defect repair completed: `STATUS: PASS_READY_FOR_STATE_11_RERUN`. It added structured `DiagnosticReport` stderr emission for structured failure paths with stdout empty, diagnostic matrix coverage for missing-file validate/compile, malformed YAML validate, runtime input decode, and storage open failure, and repaired `cli_postcard::decode_postcard` to validate version, kind, payload bound, CRC, and BLAKE3 payload digest before exposing bytes. Focused tests/clippy/Verus/rustfmt passed.

State 11 formal-verifier rerun approved. Ledger counts: PASS 9, FAIL_LOCAL 0, FAIL_REGRESSION 0, WAIVED 0, DEFERRED_GLOBAL 0. Rerun coverage includes Verus diagnostic, public exit/no-9, structured DiagnosticReport JSON/JSONL diagnostics across parse and non-parse routes, stdout/stderr separation, `vb_ui_model` postcard tests, GNU fuzz, child reconciliation, command matrix, clippy/fmt.

State 12 black-hat rerun rejected. Blocking defect: supported `--json` routes still emit raw text diagnostics for `verify <invalid-utf8-file> --json` and `inspect not-a-run --db <tmp>/db --json`. CLI postcard parity now appears fixed.

Current gate: COMPLETE. Current main/worktree code contains black-box coverage for the prior `verify` invalid UTF-8 and `inspect` invalid run-id structured diagnostics. State 12 black-hat rerun, State 13 truth-serum/final evidence decision, State 14 landing/close/sync, and State 15 cleanup report are approved in this artifact set.

## State 12-15 closure evidence (2026-05-18)

- Focused gates passed in `/home/lewis/isolated/go-skill-vb-qi37-13-git`:
  - `cargo test -p vb_cli --test vb_qi37_13_structured_reconciliation --all-features`: 14 passed.
  - `cargo test -p vb_cli --test envelope_schema_tests --all-features`: 12 passed.
  - `cargo test -p vb_ui_model --all-features postcard`: 14 passed, 152 filtered out.
  - `cargo clippy -p vb_cli --all-features -- -D warnings`: no issues found.
  - `cargo fmt --check -p vb_cli`: exit 0/no output.
- `black-hat-review.md`: `STATUS: APPROVED`.
- `truth-serum-report.md`: `STATUS: APPROVED`.
- `final-evidence-decision.md`: `STATUS: APPROVED`.
- Landing: commit `9b5f7bb0` pushed to `origin/main`; `bd close vb-qi37.13` succeeded; `bd dolt push` completed.

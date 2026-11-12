bead_id: vb-qi37.13
bead_title: cli: Reconcile structured output contract
phase: 12
updated_at: 2026-05-18T21:48:33Z
attempt: 5-of-7

# State 12 Black-Hat Review Rerun — vb-qi37.13

STATUS: APPROVED

## Scope

- Active workspace: `/home/lewis/isolated/go-skill-vb-qi37-13-git`.
- Source checkout `/home/lewis/src/velvet-ballistics` was not used for implementation, test, proof, QA, or artifact edits.
- Prior rejection findings were specifically rechecked: invalid UTF-8 `verify --json/--jsonl` and invalid run-id `inspect --json` now have black-box tests and passing command evidence.

## Evidence

- `cargo test -p vb_cli --test vb_qi37_13_structured_reconciliation --all-features` via `rtk`: `cargo test: 14 passed (1 suite, 0.00s)`.
- `cargo test -p vb_cli --test envelope_schema_tests --all-features` via `rtk`: `cargo test: 12 passed (1 suite, 0.00s)`.
- `cargo test -p vb_ui_model --all-features postcard` via `rtk`: `cargo test: 14 passed, 152 filtered out (2 suites, 0.00s)`.
- `cargo clippy -p vb_cli --all-features -- -D warnings` via `rtk`: `cargo clippy: No issues found`.
- `cargo fmt --check -p vb_cli` via `rtk`: no output, exit 0.
- Upstream artifact gate parsed `delivery-scope.jsonl`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, `verification-ledger.jsonl` and observed approved proof/contract/test/formal status lines.

## Contract parity check

- `crates/vb_cli/tests/vb_qi37_13_structured_reconciliation.rs` lines 407-469 cover invalid UTF-8 `verify --json` and `verify --jsonl` with `DiagnosticReport`, `ValidationFailed`, exit code 1, stderr-only output.
- Same test file lines 472-508 covers `inspect not-a-run --db <tmp>/db --json` with `DiagnosticReport`, `ValidationFailed`, exit code 1, stderr-only output.
- `crates/vb_cli/src/app_impl.rs` lines 219-230 parse run IDs with `OutputFormat` and route failures through `write_failure_message` instead of raw text for structured modes.

## Verdict

APPROVED. Prior State 12 blockers are resolved in current main/worktree code and focused gates pass. Proceed to State 13 evidence packaging.

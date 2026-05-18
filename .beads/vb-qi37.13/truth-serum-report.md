bead_id: vb-qi37.13
bead_title: cli: Reconcile structured output contract
phase: 13
updated_at: 2026-05-18T21:48:33Z
attempt: 1-of-7

# Truth Serum Report

STATUS: APPROVED

## Execution Evidence

- Active-context workspace guard: `pwd -P` returned `/home/lewis/isolated/go-skill-vb-qi37-13-git`; path is not equal to and not nested under `/home/lewis/src/velvet-ballistics`.
- Artifact gate: `jq -c` parsed `delivery-scope.jsonl`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, and `verification-ledger.jsonl`; approved status lines observed in proof, contract-verification, test-plan, test-suite, and formal reports.
- Structured diagnostics: `cargo test -p vb_cli --test vb_qi37_13_structured_reconciliation --all-features` via `rtk` returned `cargo test: 14 passed (1 suite, 0.00s)`.
- Envelope schemas: `cargo test -p vb_cli --test envelope_schema_tests --all-features` via `rtk` returned `cargo test: 12 passed (1 suite, 0.00s)`.
- Postcard: `cargo test -p vb_ui_model --all-features postcard` via `rtk` returned `cargo test: 14 passed, 152 filtered out (2 suites, 0.00s)`.
- CLI lint/format: `cargo clippy -p vb_cli --all-features -- -D warnings` returned `No issues found`; `cargo fmt --check -p vb_cli` returned no output/exit 0.

## Empathetic User Review

Supported structured failure paths now produce parseable diagnostic envelopes on stderr with stdout empty for the previously rejected invalid UTF-8 and invalid run-id cases.

## Skeptical QA Review

The prior black-hat rejection is not laundered: current tests directly cover the rejected routes and focused command evidence passed in this session. No missing required evidence was found for closing this dependency bead.

## Mandated Improvements

None blocking for this bead. Broader CLI operator UX beyond this dependency remains tracked by separate open beads and does not block vb-qi37.13 closure.

# Test Repair Guide: vb-qi37.13 State 9 Rerun

STATUS: APPROVED
owner_state: State 10
rerun_from: State 10 implementation
finding_count: 0

## Repair Verification

The previous State 9 rejection is resolved.

1. `crates/velvet_ballistics/tests/vb_qi37_13_structured_reconciliation.rs:99-103` now asserts exact diagnostic `message` equality in `assert_structured_validation_diagnostic`.
2. `crates/velvet_ballistics/tests/vb_qi37_13_structured_reconciliation.rs:254-258` now asserts exact JSONL unknown-command diagnostic `message` equality.
3. JSONL unknown-command still asserts stdout empty, stderr exactly one line, stable schema, `kind == DiagnosticReport`, `code == ValidationFailed`, and `exit_code == 1`.

## Remaining Red Is Implementation-Owned

The focused CLI suite still fails 4/6 because production emits plain-text/help diagnostics rather than structured JSON/JSONL diagnostic envelopes. That is a valid red phase for State 10, not a test-suite weakness.

## State 10 Must Not Weaken

- Do not relax JSON parsing.
- Do not allow diagnostic output on stdout for failures.
- Do not allow multi-line JSONL diagnostics.
- Do not replace exact `message` equality with substring matching.
- Do not remove schema/kind/code/exit_code assertions.

## Rerun Commands

Run only from `/home/lewis/src/vb-qi37-13-r2`:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics --test vb_qi37_13_structured_reconciliation --all-features --no-run
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics --test vb_qi37_13_structured_reconciliation --all-features
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p vb_ui_model --all-features postcard
```

## Exact Next State

Proceed to State 10 implementation: make the four focused CLI diagnostic tests pass by emitting structured `DiagnosticReport` envelopes for JSON/JSONL validation failures.

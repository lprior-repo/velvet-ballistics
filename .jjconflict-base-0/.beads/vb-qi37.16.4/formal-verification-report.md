# Formal Verification Report — vb-qi37.16.4

STATUS: APPROVED

## Scope

- Bead: `vb-qi37.16.4` — cli/runtime durable answer command.
- State: 12 formal verification reconciliation.
- Isolated workspace: `/home/lewis/src/Velvet-ballistics-vb-qi37-16-4-go`.
- Forbidden source checkout was not touched: `/home/lewis/src/Velvet-ballistics`.

## Inputs

All required State 12 inputs were present and non-empty:

- `.beads/vb-qi37.16.4/proof-obligations.jsonl` — 18 obligations, valid JSONL.
- `.beads/vb-qi37.16.4/traceability-matrix.jsonl` — valid JSONL.
- `.beads/vb-qi37.16.4/delivery-scope.jsonl` — valid JSONL.
- `.beads/vb-qi37.16.4/baseline-report.md`.
- `.beads/vb-qi37.16.4/tla-spec.md`.
- `.beads/vb-qi37.16.4/lean-contract.md`.
- `.beads/vb-qi37.16.4/contract-verification-review.md` — exact `STATUS: APPROVED` line present.
- `.beads/vb-qi37.16.4/formal-waivers.jsonl` — valid JSONL.

## Exact Command Evidence

| Obligation set | Command | Outcome |
|---|---|---|
| TLA obligations | `tlc -config specs/AskAnswerLifecycle.cfg specs/AskAnswerLifecycle.tla` | PASS. TLC 2.19 completed with no error at 2026-05-11 20:29:46; 868 generated states, 361 distinct states, 0 states left on queue, depth 13. |
| Static scan | `env -u RUSTC_WRAPPER cargo clippy --workspace --lib --bins -- -D warnings` | PASS. Finished dev profile with no source warnings/errors. |
| Verus tool check | `which verus; verus --version` | Missing tool confirmed: `verus not found`; covered by approved waiver `WAIVER-VERUS-TOOL-2026-05-11`. |
| Kani exact obligation | `cargo kani --harness check_payload_size --contract` | Fails before verification: cargo-kani 0.67.0 rejects `--contract`; covered by approved waiver `WAIVER-KANI-HARNESS-2026-05-11`. |
| Integration exact obligation sample | `cargo test --test cli_integration ask_answer_durable` | Fails because `cli_integration` is not a default-run test target; covered by approved waiver `WAIVER-INTEGRATION-SCOPE-2026-05-11`. |
| Integration package-qualified check | `cargo test -p velvet_ballistics --test cli_integration ask_answer_durable` | 0 passed, 74 filtered out; approved test name absent. |
| Runtime compensating tests | `cargo test -p vb_runtime --lib red_ask_answer_durable`; `red_ask_answer_secret_redaction`; `red_ask_answer_diagnostics_safe`; `red_test_payload_size_one_byte_over` | PASS; each command ran one matching test successfully. |
| Optional unit/proptest commands | `cargo test --lib answer_error_`; `cargo test --lib proptest_payload_size` | 0 matching tests; obligations are `required:false` and waived in `verification-layers.md`. |

## Obligation Results

Current ledger: `.beads/vb-qi37.16.4/verification-ledger.jsonl`.

| Result | Count |
|---|---:|
| PASS | 7 |
| WAIVED | 11 |
| FAIL_LOCAL | 0 |
| FAIL_REGRESSION | 0 |
| DEFERRED_GLOBAL | 0 |

Layer rollup:

- TLA+: 6 PASS.
- Static scan: 1 PASS.
- Verus: 4 WAIVED by `WAIVER-VERUS-TOOL-2026-05-11`.
- Kani: 1 WAIVED by `WAIVER-KANI-HARNESS-2026-05-11`.
- Integration-test: 4 WAIVED by `WAIVER-INTEGRATION-SCOPE-2026-05-11`.
- Optional unit/proptest: 2 WAIVED by existing `verification-layers.md` waivers; both are `required:false` in `proof-obligations.jsonl`.

## Waiver Validation

`formal-waivers.jsonl` contains three explicit approved waivers. Each has `status:"APPROVED"`, owner, expiry `2026-05-18`, reason, covered obligation IDs, compensating evidence, and residual risk:

- `WAIVER-VERUS-TOOL-2026-05-11` covers `VERUS-INV-002`, `VERUS-PRE-004`, `VERUS-PRE-005`, `VERUS-PRE-003`.
- `WAIVER-KANI-HARNESS-2026-05-11` covers `KANI-PRE-003`.
- `WAIVER-INTEGRATION-SCOPE-2026-05-11` covers `INTEGRATION-POST-004`, `INTEGRATION-POST-005`, `INTEGRATION-PRE-006`, `INTEGRATION-ERR-VALIDATION`.

## Residual Risk / Follow-up

- Verus proof evidence is waived, not passed. Install Verus and rerun the exact Verus commands before release signoff.
- Kani bounded proof evidence is waived, not passed. Add/update an approved `check_payload_size` harness or amend the approved obligation.
- CLI integration obligations are waived because approved command/test names are mis-scoped. Add/rename package-qualified tests or amend the approved obligations.
- Optional unit/proptest obligations have 0 matching tests under the approved commands but are `required:false` and already waived in `verification-layers.md`.

## Decision

All 18 proof obligations are accounted for in `verification-ledger.jsonl`. Every required obligation is `PASS` or covered by an approved waiver. No `FAIL_LOCAL`, `FAIL_REGRESSION`, or blocking `DEFERRED_GLOBAL` entries remain.

STATUS: APPROVED

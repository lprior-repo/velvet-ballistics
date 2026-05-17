# Proof Evidence: vb-qi37.12 State 5 Repair

Timestamp: `2026-05-16T03:34:27Z`

## Scope And Isolation

- Role: go-skill State 5 proof-writer repair after State 4 plan/schema repair.
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12`.
- Source checkout write boundary: `/home/lewis/src/velvet-ballistics`.
- Isolation guard rerun: `pwd -P` returned the workspace path and rejected the source checkout and descendants.
- This repair refreshed bead evidence/report/ledger/STATE artifacts only; no production code, tests, proof models, fuzz targets, dependencies, or CI config were edited.

## Schema And Artifact Validation

- Required artifact gate exited 0 for `proof-obligations.planned.jsonl`, `proof-obligations.jsonl`, `proof-strategy.md`, `proof-evidence.md`, and `proof-writer-report.md`.
- `jq -c .` exited 0 for `proof-obligations.planned.jsonl`, `proof-obligations.jsonl`, `proof-execution-ledger.jsonl`, and `traceability-matrix.jsonl`.
- Repaired schema query exited 0: all required rows in `proof-obligations.planned.jsonl` have `status:"planned"`.
- TLA metadata query exited 0: all TLA rows in `proof-obligations.planned.jsonl` include `tla_module`, `model`, `config`, `variables`, `actions`, `invariants`, `temporal_properties`, `fairness`, `state_constraints`, and `refinement`.
- Focused-command query exited 0: `TEST-JOURNAL-007` and `TEST-RUNTIME-008` no longer use generic `moon ci`; only `GATE-RELEASE-010` owns release CI.
- Repaired contract obligation query exited 0: all required rows in `proof-obligations.jsonl` have `status:"planned"`; PASS evidence remains in this file and `proof-execution-ledger.jsonl`.

## Focused Proof Rerun Evidence

- `TLA-ACK-001`, `TLA-REC-002`, `TLA-DEADLOCK-011`: `TMPDIR=target/tmp tlc -config .beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.cfg .beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.tla` exited 0. TLC reported `Model checking completed. No error has been found.`, 795 states generated, 280 distinct states found, 0 states left on queue, depth 8, and temporal properties checked.
- `VERUS-CLS-003`: `TMPDIR=target/tmp verus .beads/vb-qi37.12/proof/verus/discard_classification.rs` exited 0 with `verification results:: 1 verified, 0 errors`.
- `VERUS-DIAG-004`: `TMPDIR=target/tmp verus .beads/vb-qi37.12/proof/verus/diagnostic_envelope.rs` exited 0 with `verification results:: 1 verified, 0 errors`.
- `VERUS-DEC-005`: `TMPDIR=target/tmp verus .beads/vb-qi37.12/proof/verus/recovery_decode_class.rs` exited 0 with `verification results:: 1 verified, 0 errors`.
- `SCAN-DISCARD-006`: `/usr/bin/rg -n "let _ =|\\.ok\\(|Err\\(_\\)|Err\\([^)]*\\) =>|log::|tracing::" crates/vb_storage/src crates/vb_runtime/src crates/vb_compile/src crates/workspace_tests/src > .beads/vb-qi37.12/silent-discard-scan-report.full.raw.txt` exited 0; raw scan has 690 candidate lines across 66 files. Existing classification report records zero unclassified release-critical silent discards.
- `TEST-JOURNAL-007`: `TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12/target/tmp RUSTC_WRAPPER= rtk cargo test -p vb_storage decode_rejects -- --nocapture` exited 0 with `36 passed, 947 filtered out`; `TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12/target/tmp RUSTC_WRAPPER= rtk cargo test -p vb_storage process_lock -- --nocapture` exited 0 with `4 passed, 979 filtered out`.
- `TEST-RUNTIME-008`: `TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12/target/tmp RUSTC_WRAPPER= rtk cargo test -p vb_runtime diagnostic -- --nocapture` exited 0 with `10 passed, 1450 filtered out`.
- `FUZZ-DECODE-009`: `TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12/target/tmp RUSTC_WRAPPER= CARGO_TARGET_DIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12/target/tmp/cargo-fuzz cargo fuzz run vb_qi37_12_persisted_payload_decode --target x86_64-unknown-linux-gnu -- -runs=1000` exited 0; cargo finished the release build and launched `vb_qi37_12_persisted_payload_decode -runs=1000` with no reported crash artifact.

## Deferred Evidence

- `GATE-RELEASE-010`: `moon ci` was not run in State 5. It remains the State 11 formal-verifier/release gate.
- Non-applicable lanes remain as repaired State 4 planned: Kani, proptest, Loom, Miri, Flux, dependency audit. Lean remains waived by State 4 because no theorem-only kernel is present.

## Assumptions Recorded

- TLA+ fairness assumptions: weak fairness on `ReturnTypedError`, `HydrateFailClosed`, and `RuntimeTerminalFail`.
- TLA+ model bounds: finite `Ops`, `PersistStates`, `AckStates`, `RecoveryStates`, `RuntimeStates`, and `DiagnosticStates` enumerated in `.beads/vb-qi37.12/proof/tla/SilentDiscardLifecycle.tla`.
- TLA+ deadlock evidence is non-vacuous because `Next` contains no explicit `Stutter`; stuttering is supplied only by `[Next]_vars`, and the cfg contains no `CHECK_DEADLOCK FALSE` directive.
- Verus trusted boundaries: inventory records, diagnostic constructors, and byte/decode classifiers are abstract proof inputs rather than concrete production code.
- Fuzz target oracle uses concrete `vb_storage::decode_record::<JournalEvent>` and checks generated corrupt/truncated records for typed errors.
- No release PASS is claimed for `moon ci`; downstream State 11 must run it.

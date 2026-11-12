# Verification Layers: vb-2b4g

This bead accepts executable parity, static generated-source scans, compile/trybuild, formatting, and `moon ci` confidence only. It does not claim TLA+, Verus, Kani, Lean/Aeneas/Hax, theorem-kernel, performance, or formal state-machine proof. The machine-readable waiver/non-claim records are in `.beads/vb-2b4g/formal-waivers.jsonl` and are part of this verification plan.

## Required layers

| Layer | Clauses | Evidence |
|---|---|---|
| Runtime parity | POST-001, POST-002, POST-003, POST-004, INV-001..005 | Focused `cargo test -p vb_codegen *_generated_parity -- --nocapture` suites comparing generated execution to `vb_runtime::drive_deterministic_full`. |
| Oracle guard | PRE-004, POST-005 | Tests fail if target-family oracle path reports `not_yet_implemented`; `vb_core::run_until_blocked` is not accepted as oracle. |
| Static generated-source scan | POST-006, INV-003 | `cargo test -p vb_codegen generated_source_contract -- --nocapture`. |
| Compile/trybuild | POST-006 | `cargo check -p vb_codegen --all-targets`; `cargo test -p vb_codegen --test trybuild_tests`. |
| Formatting | POST-006 | `cargo fmt --all -- --check`. |
| Workspace confidence | all | `moon ci` at landing; unrelated failures must be classified as non-bead global debt with raw output. |

## Family-specific minimum scenarios

- Repeat: first attempt, later attempt, exhausted limit, overflow/capacity error, taint on finish, attempt counter parity.
- Reduce: empty, single item, multiple items, wrong type, capacity/materialization error, accumulator taint join.
- Together: all branches success, branch result order, typed branch failure, join taint lattice, fanout/capacity error.
- Collect: single page, multiple pages, duplicate page, stale page, out-of-order/missing page if runtime exposes it, materialization order, capacity exceeded, lineage/taint.

## Waivers / non-claims

- Formal verifier rule: for every formal lane listed in `.beads/vb-2b4g/formal-waivers.jsonl`, classify the lane as `WAIVED` when the listed compensating executable/static obligations pass, or `NOT_IN_SCOPE` when the verifier records non-claimed optional lanes. Never classify any waived/non-claimed formal lane as `PASS` for this bead.
- Clauses `POST-001`, `POST-002`, `POST-003`, `POST-004`, and `INV-001..INV-005` waive/non-claim TLA+ temporal/formal state-machine proof for local acceptance. Limitation: this bead checks generated-code parity against the existing `vb_runtime::engine::drive::drive_deterministic_full` oracle and does not introduce or prove a standalone semantic state machine for Repeat/Reduce/Together/Collect. Owner/follow-up: `vb-w20g` owns future TLA+ modeling; it is cited only as a follow-up, not as completed evidence. Expiry: before any release/formal-assurance claim for these families or when `vb-w20g` is claimed complete and reviewed.
- Clauses `POST-001`, `POST-002`, `POST-003`, `POST-004`, and `INV-001..INV-005` waive/non-claim Verus Rust-local proof for local acceptance. Limitation: generated helper internals and runtime refinement are not bound to Verus specs in this bead. Owner/follow-up: `vb-h3fx` owns future Verus obligations; it is cited only as a follow-up, not as completed evidence. Expiry: before advertising deductive Rust-core proof for these generated families or when `vb-h3fx` is complete and reviewed.
- Clauses `POST-001`, `POST-002`, `POST-003`, `POST-004`, and `INV-001..INV-005` waive/non-claim Kani bounded model checking for local acceptance. Limitation: this bead does not add `kani::Arbitrary` generators or bounded harnesses for workflow state/counter/page shapes. Owner/follow-up: `vb-mnv0` owns future Kani harnessing; it is cited only as a follow-up, not as completed evidence. Expiry: before claiming bounded model-check evidence or when `vb-mnv0` is complete and reviewed.
- Clauses `POST-001`, `POST-002`, `POST-003`, `POST-004`, and `INV-001..INV-005` waive/non-claim Lean/Aeneas/Hax/theorem-kernel proof for local acceptance. Limitation: no tiny theorem kernel or extraction/refinement relation is authored by this bead. Owner/follow-up: parent formal roadmap / future theorem-kernel bead, not this bead. Expiry: before any theorem or extracted-proof claim for these families.
- Performance is a non-goal for all clauses in this bead. No speedup, p99, throughput, allocation, code-size, vectorization, or zero-cost abstraction claim may be made from this bead. Formal-verifier must classify performance as `NOT_IN_SCOPE`, not `PASS`, unless a future bead adds exact benchmark obligations and raw baseline/result evidence.
- Compensating local evidence for the waivers is limited to `PO-001..PO-008`: runtime parity against `drive_deterministic_full`, oracle guard rejecting `not_yet_implemented`, static generated-source scan, compile/trybuild/fmt gates, and `moon ci` landing confidence. These obligations are executable confidence checks, not formal proof substitutes.

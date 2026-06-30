# Black Hat Review — vb-2b4g State 12

STATUS: APPROVED

## Startup Sources Applied

- Read `/home/lewis/.claude/skills/black-hat-reviewer/SKILL.md` lines 12-21, 23-33, and 40-44: contract parity first; Farley rigor; Holzman Rust panic/unsafe gate; cite exact line evidence.
- Read `/home/lewis/.agents/skills/black-hat-reviewer/SKILL.md` lines 12-21, 23-33, and 40-44: same content; per instruction this file wins on conflict. No conflict found.

## Files Reviewed

- `.beads/vb-2b4g/contract.md`
- `.beads/vb-2b4g/verification-layers.md`
- `.beads/vb-2b4g/formal-waivers.jsonl`
- `.beads/vb-2b4g/proof-obligations.jsonl`
- `.beads/vb-2b4g/traceability-matrix.jsonl`
- `.beads/vb-2b4g/contract-verification-review.md`
- `.beads/vb-2b4g/test-plan.md`
- `.beads/vb-2b4g/test-suite-review.md`
- `.beads/vb-2b4g/test-writer-report.md`
- `.beads/vb-2b4g/implementation.md`
- `.beads/vb-2b4g/formal-verification-report.md`
- `.beads/vb-2b4g/verification-ledger.jsonl`
- `.beads/vb-2b4g/machine-gate-report.md`
- `.beads/vb-2b4g/regression-diff.md`
- `crates/vb_codegen/src/lib.rs`
- `crates/vb_codegen/src/tests.rs`
- `crates/vb_codegen/src/proptests.rs`
- `crates/vb_codegen/tests/compile-fail/pass/minimal_workflow.rs`

## Commands Run

- `jj status` — inspected working-copy scope.
- `jj diff --stat` — inspected changed-file scope.
- `jj diff --git -- crates/vb_codegen/src/lib.rs` — inspected active implementation diff.
- `rtk cargo test -p vb_codegen repeat_generated_parity -- --nocapture` — PASS: 3 passed / 364 filtered.
- `rtk cargo test -p vb_codegen reduce_generated_parity -- --nocapture` — PASS: 3 passed / 364 filtered.
- `rtk cargo test -p vb_codegen together_generated_parity -- --nocapture` — PASS: 2 passed / 365 filtered.
- `rtk cargo test -p vb_codegen collect_generated_parity -- --nocapture` — PASS: 3 passed / 364 filtered.
- `rtk cargo test -p vb_codegen generated_source_contract -- --nocapture` — PASS: 3 passed / 364 filtered.

## Findings

No blocking defects found.

## Gate Checks

- Contract parity: APPROVED. The contract requires `drive_deterministic_full` as oracle and forbids `not_yet_implemented` laundering (`contract.md:15-19`, `contract.md:23-35`). The parity harness uses `drive_deterministic_full` in `tests.rs:4866-4876`, rejects unsupported sentinels in `tests.rs:4963-4968` and `tests.rs:5106-5110`, and focused Repeat/Reduce/Together/Collect parity commands passed locally.
- Collect page state / journal evidence: APPROVED. Generated code adds collect state, lineage, typed order violations with `run_id`, journal extras, and `RunFinished` evidence (`lib.rs:1880-1891`, `lib.rs:1912-1929`, `lib.rs:3061`, `lib.rs:3104-3135`, `lib.rs:3195-3279`). Tests compare runtime/generated collect page state and duplicate/stale/out-of-order paths without dropping `RunFinished` (`tests.rs:4754-4768`, `tests.rs:4770-4796`, `tests.rs:4908-4962`, `tests.rs:5695-5768`).
- Holzman Rust / generated-source contract: APPROVED. Production file forbids unsafe at crate root (`lib.rs:1`); emitted source forbids unsafe and denies key lints (`lib.rs:2988-2992`). The focused source contract command passed. No production `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, or `unsafe` implementation escape was found in the reviewed production generated-code path.
- Farley / DDD / simplicity: APPROVED with residual risk. The implementation is not small, but the added complexity is directly tied to the contract-required collect pagination side state and journal observability. No source-scan-only fake behavior was accepted; executable runtime-oracle parity is present.
- Verification truthfulness: APPROVED. `formal-waivers.jsonl` marks TLA+/formal-state-machine/Verus/Kani as WAIVED and Lean/Aeneas/Hax/theorem/performance as NOT_IN_SCOPE; the ledger records these lanes as not PASS (`verification-ledger.jsonl:12-20`). `formal-verification-report.md:24-40` and `machine-gate-report.md:10-22` classify focused/local gates as PASS and `moon ci` as `DEFERRED_GLOBAL` for disk quota/resource failures after scoped lint passed.

## Residual Risks / Non-Blocking

- `moon ci` remains deferred due disk quota/resource failure; this is landing/resource debt, not a reproduced bead-local `vb_codegen` failure (`machine-gate-report.md:22-44`, `regression-diff.md:11-22`). Re-run before final release/landing confidence.
- No TLA+, Verus, Kani, theorem-kernel, performance, or mutation evidence is claimed. Existing formal waivers are honest and must not be upgraded to proof claims.
- Some tests still rely on generated/rustc harness compilation in temp directories; quota exhaustion can create false failures. Keep exact focused reruns separate from depleted `moon ci` runs.

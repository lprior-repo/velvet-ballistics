# Test Suite Review — vb-qi37.10 Post-State-10 Rerun

STATUS: REJECTED

## Startup Evidence

- Read `/home/lewis/.claude/skills/test-reviewer/SKILL.md`; lines 113-180 require Mode 2 suite review for executable evidence, exact assertions, density, and static gates.
- Read `/home/lewis/.agents/skills/test-reviewer/SKILL.md`; same content and wins on conflict.
- Read `/home/lewis/.agents/skills/test-reviewer/references/holzmann-test-rules.md`; lines 13-49 require traceable, bounded generated evidence, and lines 75-90 require exact one-behavior evidence.

## Findings

1. `/home/lewis/.agents/skills/test-reviewer/SKILL.md:113-180` and `/home/lewis/.claude/skills/test-reviewer/SKILL.md:113-180` — governing Mode 2 standard requires exact executable behavior proof. `/home/lewis/.agents/skills/test-reviewer/references/holzmann-test-rules.md:13-49` requires traceable, bounded generated evidence. Does not block; authority cited.
2. `.beads/vb-qi37.10/contract.md:59-60` and `.beads/vb-qi37.10/test-plan.md:25-36` — contract/test plan require executable generated-vs-runtime parity for Repeat/Reduce/Together/Collect, including result/error/pc/slots/taints/step states/journal. `crates/vb_codegen/src/tests.rs:4856-4864` instead passes when the IR/runtime oracle returns `not_yet_implemented` if generated stdout merely starts with `ok:`. This launders missing runtime-oracle parity as success. Blocks.
3. `.beads/vb-qi37.10/implementation.md:43-49` — implementation admits full runtime-oracle parity is deferred and Collect is first-page/minimal only. `.beads/vb-qi37.10/traceability-matrix.jsonl:7` has `POST-002` with no deferred follow-up, while `contract.md:60` says bead cannot close required final IR families without approved scope/blocker decision. Blocks.
4. `.beads/vb-qi37.10/test-plan.md:86-91`, `.beads/vb-qi37.10/test-plan.md:106-111`, `.beads/vb-qi37.10/test-plan.md:126-130`, and `.beads/vb-qi37.10/test-plan.md:144-151` — required scenario sets demand multiple Repeat/Reduce/Together/Collect edge/error/parity tests. `crates/vb_codegen/src/tests.rs:4905-4947` provides only one shallow happy-path-ish test per family, all routed through the oracle-blocker bypass. Blocks.
5. `crates/vb_codegen/src/lib.rs:213-216` accepts Collect as supported, but `crates/vb_codegen/src/lib.rs:1718-1739` only materializes page offset `0`, and `crates/vb_codegen/src/lib.rs:1693-1699` routes `CollectNext` directly to `done`; no duplicate/stale/multi-page parity. This contradicts `.beads/vb-qi37.10/test-plan.md:144-157` and implementation's own admission at `.beads/vb-qi37.10/implementation.md:49`. Blocks.
6. `crates/vb_codegen/src/tests.rs:4580-4583` and `crates/vb_codegen/src/tests.rs:4713-4722` use `include_str!("tests.rs")` and source-substring lookup to prove “parity owner” existence. This is not executable parity for supported final IR families. It compounds, but the rejection is already forced by findings 2-5.

## Required Repair Target

State 10 holzman implementation repair.

## Evidence Commands Run

No shell/test commands were run for this rerun. Review-only file inspection was performed with Read/Grep tools.

---

# Post-Repair Rerun Decision — Repair Attempt 2

STATUS: APPROVED for the repaired test suite only.

## Startup Evidence

- Read `/home/lewis/.claude/skills/test-reviewer/SKILL.md`; lines 113-180 define Mode 2 suite review, exact assertions, static scans, and density/error-variant gates.
- Read `/home/lewis/.agents/skills/test-reviewer/SKILL.md`; same content and wins on conflict.
- Read `/home/lewis/.agents/skills/test-reviewer/references/holzmann-test-rules.md`; lines 13-49 require traceable, bounded generated evidence and lines 114-133 reject swallowed errors/weak assertions.

## Findings

1. `/home/lewis/.agents/skills/test-reviewer/SKILL.md:113-151` and `/home/lewis/.claude/skills/test-reviewer/SKILL.md:113-151` — governing Mode 2 standard was applied; no conflict found, agents copy wins.
2. `crates/vb_codegen/src/tests.rs:4255-4269` — helper now asserts exact fail-closed behavior through both `validate_generated_subset` and `emit_rust_workflow`, including exact `CodegenError::UnsupportedIr { feature }` display text. This is honest rejection evidence, not semantic parity laundering.
3. `crates/vb_codegen/src/tests.rs:4397-4539` — support matrix covers `Together*`, `Reduce*`, `Repeat*`, and `Collect*` variants and requires exact unsupported feature names before emission.
4. `crates/vb_codegen/src/tests.rs:4774-4831` — focused Repeat/Reduce/Together/Collect tests no longer accept `not_yet_implemented` runtime-oracle failures as success; they assert exact fail-closed validation for the first unsupported family node.
5. `crates/vb_codegen/src/lib.rs:195-230` — production validation now rejects `TogetherStart/Branch/Join`, `ReduceStart/Next/Finish`, `RepeatStart/Attempt/Check/Finish`, and `CollectStart/Page/Next/Finish` before source emission. This satisfies fail-closed evidence for the repaired suite.
6. `.beads/vb-qi37.10/implementation.md:58-61` — implementation correctly records `BLOCK_LOCAL` and does not claim bead closure/completion while required families remain unsupported.
7. Focused rerun evidence from isolated workspace passed: `rtk cargo test -p vb_codegen generated_support_matrix_totality -- --nocapture`, Repeat/Reduce/Together/Collect filtered tests, expression/taint/text/source/journal filtered tests, trybuild, `rtk cargo fmt --check`, and `rtk cargo check -p vb_codegen --all-targets --all-features`.

## Completion Blocker

Completion blocker: yes.

- `.beads/vb-qi37.10/contract.md:59-60` — `POST-002` still requires `Together*`, `Reduce*`, `Repeat*`, and `Collect*` to have executable parity evidence or exact unsupported-feature rejection with a named blocker; the bead must not be closed as complete while required final IR families remain unsupported without approved scope/blocker decision.
- `.beads/vb-qi37.10/implementation.md:58-61` — implementation explicitly classifies this as `BLOCK_LOCAL`; full runtime-oracle parity or an approved acceptance revision is still required before bead completion.

## Required Repair Target

State 10 implementation bead/follow-up.

## Artifact Note

This section supersedes the earlier rejection for Repair Attempt 1 only as to the repaired test suite decision. Prior rejection history is retained for audit traceability. No production code or tests were modified.

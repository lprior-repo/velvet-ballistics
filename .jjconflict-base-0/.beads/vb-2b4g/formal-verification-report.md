# Formal Verification Report — vb-2b4g

STATUS: APPROVED

## Startup Sources Applied

- `/home/lewis/.claude/skills/formal-verifier/SKILL.md`: rules require approved formal plan, exact obligation commands, accounting every obligation, classifying failures by scope/baseline, and never inventing evidence.
- `/home/lewis/.agents/skills/formal-verifier/SKILL.md`: same v1.5.0 content; per instruction this file wins on conflict. No conflict found.

## Inputs

- `.beads/vb-2b4g/contract.md`: present.
- `.beads/vb-2b4g/verification-layers.md`: present.
- `.beads/vb-2b4g/formal-waivers.jsonl`: present and cited for waived/non-scope formal lanes.
- `.beads/vb-2b4g/proof-obligations.jsonl`: present and valid JSONL.
- `.beads/vb-2b4g/traceability-matrix.jsonl`: present and valid JSONL.
- `.beads/vb-2b4g/contract-verification-review.md`: `STATUS: APPROVED`.
- `.beads/vb-2b4g/delivery-scope.jsonl`: present and valid JSONL.
- `.beads/vb-2b4g/baseline-report.md`: present.
- `.beads/vb-2b4g/test-suite-review.md`: `STATUS: APPROVED`.
- `.beads/vb-2b4g/test-writer-report.md`: present.
- `.beads/vb-2b4g/implementation.md`: present.

## Obligation Results

- PO-001 — PASS — `rtk cargo test -p vb_codegen repeat_generated_parity -- --nocapture`: `cargo test: 3 passed, 364 filtered out (3 suites, 0.15s)`.
- PO-002 — PASS — `rtk cargo test -p vb_codegen reduce_generated_parity -- --nocapture`: `cargo test: 3 passed, 364 filtered out (3 suites, 0.27s)`.
- PO-003 — PASS — `rtk cargo test -p vb_codegen together_generated_parity -- --nocapture`: `cargo test: 2 passed, 365 filtered out (3 suites, 0.15s)`.
- PO-004 — PASS — `rtk cargo test -p vb_codegen collect_generated_parity -- --nocapture`: `cargo test: 3 passed, 364 filtered out (3 suites, 0.39s)`.
- PO-005 — PASS — accounted by same fresh focused reruns for repeat/reduce/together/collect; all passed.
- PO-006 — PASS — `rtk cargo test -p vb_codegen generated_source_contract -- --nocapture`: `cargo test: 3 passed, 364 filtered out (3 suites, 0.03s)`.
- PO-007 — PASS — declared command accounted with direct cargo binary equivalent: `/home/lewis/.cargo/bin/cargo check -p vb_codegen --all-targets && /home/lewis/.cargo/bin/cargo test -p vb_codegen --test trybuild_tests && /home/lewis/.cargo/bin/cargo fmt --all -- --check`. Fresh orchestrator evidence: cargo check finished dev profile successfully; trybuild tests ran 3 tests and all passed; cargo fmt completed with no diff output. This matches the proof-obligation command `cargo check -p vb_codegen --all-targets && cargo test -p vb_codegen --test trybuild_tests && cargo fmt --all -- --check` without the prior `--all-features` / RTK-only variant.
- PO-008 — DEFERRED_GLOBAL — `moon ci` returned `MOON_CI_EXIT_STATUS=1`, but scoped lint passed and remaining evidence is disk quota/resource exhaustion. Exact focused/full local `vb_codegen` gates pass.
- Machine full local suite — PASS — `rtk cargo test -p vb_codegen -- --nocapture`: `cargo test: 367 passed (4 suites, 2.95s)`.

## Failure / Deferred Classifications

- No FAIL_LOCAL remains.
- DEFERRED_GLOBAL: `moon ci` disk quota/resource failures: feature-powerset failed writing incremental query cache; fuzz-smoke linker/rustc failed with `Disk quota exceeded`; mutants-smoke failed writing temp files; moon failed writing `.moon/cache/states/.../stdout.log`.
- DEFERRED_GLOBAL: Moon nextest generated-temp `vb_codegen` failures occurred inside the same disk-exhausted run. They are not classified bead-local because the exact focused parity commands and full `rtk cargo test -p vb_codegen -- --nocapture` passed outside the exhausted `moon ci` run.

## Waivers / Non-Scope Lanes

Used `.beads/vb-2b4g/formal-waivers.jsonl`.

- TLA+ and formal state-machine lanes: WAIVED, not PASS.
- Verus lane: WAIVED, not PASS.
- Kani lane: WAIVED, not PASS.
- Lean, Aeneas/Lean, Hax/Lean, theorem-kernel, and performance lanes: NOT_IN_SCOPE per waiver file; ledger records them as WAIVED/non-scope and not PASS.
- No TLA+/Verus/Kani proof coverage is claimed.

## Residual Risk

- Runtime parity evidence is executable-test evidence only, not formal refinement proof.
- `moon ci` did not complete cleanly due disk quota/resource exhaustion; re-run after freeing workspace quota before final landing/release confidence.
- Formal lanes remain waived/non-scope by approved waiver file.

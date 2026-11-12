# Machine Gate Report — vb-2b4g

## Startup / Input Gate

- Read `/home/lewis/.claude/skills/formal-verifier/SKILL.md` and `/home/lewis/.agents/skills/formal-verifier/SKILL.md`; both define v1.5.0 rules. No conflict observed; `.agents` would win.
- Required bead input existence + JSONL parse gate: PASS.
- `contract-verification-review.md`: `STATUS: APPROVED`.
- `test-suite-review.md`: `STATUS: APPROVED`.

## Exact Command Results

- `rtk cargo test -p vb_codegen repeat_generated_parity -- --nocapture` — PASS: `cargo test: 3 passed, 364 filtered out (3 suites, 0.15s)`.
- `rtk cargo test -p vb_codegen reduce_generated_parity -- --nocapture` — PASS: `cargo test: 3 passed, 364 filtered out (3 suites, 0.27s)`.
- `rtk cargo test -p vb_codegen together_generated_parity -- --nocapture` — PASS: `cargo test: 2 passed, 365 filtered out (3 suites, 0.15s)`.
- `rtk cargo test -p vb_codegen collect_generated_parity -- --nocapture` — PASS: `cargo test: 3 passed, 364 filtered out (3 suites, 0.39s)`.
- PO-005 combined oracle guard/focused coverage — PASS by same fresh focused reruns above.
- `rtk cargo test -p vb_codegen generated_source_contract -- --nocapture` — PASS: `cargo test: 3 passed, 364 filtered out (3 suites, 0.03s)`.
- PO-007 declared command via direct cargo binary equivalent — PASS: `/home/lewis/.cargo/bin/cargo check -p vb_codegen --all-targets && /home/lewis/.cargo/bin/cargo test -p vb_codegen --test trybuild_tests && /home/lewis/.cargo/bin/cargo fmt --all -- --check`. Fresh orchestrator evidence: cargo check finished dev profile successfully; trybuild tests ran 3 tests and all passed; cargo fmt completed with no diff output.
- `rtk cargo test -p vb_codegen -- --nocapture` — PASS: `cargo test: 367 passed (4 suites, 2.95s)`.
- `moon ci` — DEFERRED_GLOBAL: `MOON_CI_EXIT_STATUS=1` with disk quota/resource failures; no scoped lint failure remained.

## Moon CI Evidence

Scoped lint repair verified:

```text
tool_e3791d01c0018lXo5c2X4NAzMP lines 76-79:
▮▮▮▮ velvet-ballistics:lint-src (f8cca462)
▮▮▮▮ velvet-ballistics:lint-src (182ms, f8cca462)
```

Environment/global failures:

```text
velvet-ballistics:feature-powerset | error: failed to write query cache to .../target/debug/incremental/.../query-cache.bin: Disk quota exceeded (os error 122)
velvet-ballistics:fuzz-smoke | LLVM ERROR: IO failure on output stream: Disk quota exceeded
velvet-ballistics:mutants-smoke | Disk quota exceeded (os error 122)
Error: fs::write Failed to write .../.moon/cache/states/velvet-ballistics/fuzz-smoke/stdout.log. Disk quota exceeded (os error 122)
MOON_CI_EXIT_STATUS=1
```

Moon nextest also reported generated-temp `vb_codegen` tests failing while the same run was disk-exhausted. These are classified as environment-induced because all exact focused `vb_codegen` parity/static gates and the full local `vb_codegen` suite passed immediately outside the exhausted `moon ci` run.

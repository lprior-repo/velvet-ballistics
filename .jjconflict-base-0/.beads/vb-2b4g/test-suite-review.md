STATUS: APPROVED

## Test-suite re-review: vb-2b4g prior rejection repair

Startup sources read and applied:

- `/home/lewis/.claude/skills/test-reviewer/SKILL.md` lines 8-14: test-reviewer must adversarially find hollow tests and read evidence rules.
- `/home/lewis/.agents/skills/test-reviewer/SKILL.md` lines 8-14: same content; this file wins on conflict.

## Scope

This re-review focused on the prior rejection only plus obvious new regressions:

1. `RunFinished` terminal journal evidence preservation/comparison.
2. `status=Err(Core(...))` wrapper shape no longer normalized away.
3. Collect duplicate/stale/out-of-order generated setup fail-fast behavior.

## Findings

- Prior lethal fixed: `crates/vb_codegen/src/tests.rs:4754-4768` no longer filters out `journal:RunFinished` lines. Runtime terminal evidence is added at `crates/vb_codegen/src/tests.rs:4770-4796` and appended at `crates/vb_codegen/src/tests.rs:4954-4962`. Generated observations print `RunFinished` at `crates/vb_codegen/src/tests.rs:5068`.
- Prior lethal fixed: `crates/vb_codegen/src/tests.rs:4754-4768` no longer rewrites `status=Err(Core(...))` to `status=Err(...)`. Generated observation status now emits `status=Err(Core({error:?}))` at `crates/vb_codegen/src/tests.rs:5084`, so wrapper text participates in equality instead of being stripped.
- Prior major fixed: Collect duplicate/stale/out-of-order setup no longer uses `unwrap_or(0)` or ignored `upsert` results. Fail-fast helpers are defined at `crates/vb_codegen/src/tests.rs:5011-5029`; cases use them at `crates/vb_codegen/src/tests.rs:5701-5712`.
- No new regression found in the inspected prior-rejection surface. `lib.rs` still emits generated `RunFinished` journal events (`crates/vb_codegen/src/lib.rs:448`, `crates/vb_codegen/src/lib.rs:3101`, `crates/vb_codegen/src/lib.rs:3120`).

## Command evidence

Accepted orchestrator-provided fresh evidence:

- `rtk cargo test -p vb_codegen collect_generated_parity -- --nocapture` PASS, 3 passed / 364 filtered.
- `rtk cargo test -p vb_codegen journal_signature_generated_parity -- --nocapture` PASS, 1 passed / 366 filtered.
- `rtk cargo test -p vb_codegen -- --nocapture` PASS, 367 passed.
- `rtk cargo fmt --check` PASS.
- `rtk cargo check -p vb_codegen --all-targets --all-features` PASS.
- `rtk cargo test -p vb_codegen generated_source_contract -- --nocapture` PASS, 3 passed / 364 filtered.
- `rtk cargo test -p vb_codegen repeat_generated_parity -- --nocapture` PASS, 3 passed / 364 filtered.
- `rtk cargo test -p vb_codegen reduce_generated_parity -- --nocapture` PASS, 3 passed / 364 filtered.
- `rtk cargo test -p vb_codegen together_generated_parity -- --nocapture` PASS, 2 passed / 365 filtered.
- `rtk cargo test -p vb_codegen --test trybuild_tests` PASS, 3 passed.

Commands run by reviewer:

- Static inspection via file reads/grep only; no cargo rerun.

## Residual risks

- No mutation run evidence was provided for the newly repaired observation helpers.
- Runtime `RunFinished` evidence is synthesized from the finished runtime state because runtime evidence iteration does not expose a native `RunFinished` event in this helper; this is acceptable for this gate but should remain documented.

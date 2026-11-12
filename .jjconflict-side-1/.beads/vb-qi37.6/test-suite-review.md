# vb-qi37.6 State 9 Test Suite Review

STATUS: APPROVED

## Startup citations

- `/home/lewis/.claude/skills/test-reviewer/SKILL.md` lines 113-187 define suite review static gates for banned weak assertions, ignored tests, sleeps, shared mutable state, mocks, black-box purity, error variant completeness, and density.
- `/home/lewis/.agents/skills/test-reviewer/SKILL.md` contains the same rules and wins on conflict.
- `/home/lewis/.agents/skills/test-reviewer/references/holzmann-test-rules.md` lines 32-49 allow bounded tables/helpers with exact assertions, and lines 114-133 ban swallowed errors and unwrap-as-assertion patterns.

## Scope reviewed

State 8 setup suite and touched setup artifacts only:

- `crates/workspace_tests/tests/vb_qi37_6_state8_setup.rs`
- `crates/vb_core/src/kani.rs`
- `crates/vb_core/src/lib.rs` Kani module export
- `fuzz/Cargo.toml`
- `fuzz/src/bin/capability_name_schema.rs`
- `fuzz/src/bin/capability_contract_schema.rs`
- Focused behavior tests cited by State 8 report in `vb_core`, `vb_runtime`, and `vb_ui_model`.

## Tier 0 — Static / assertion audit

[PASS] Setup assertions are exact:

- `crates/workspace_tests/tests/vb_qi37_6_state8_setup.rs:29` asserts exactly `KANI_SETUP_PRESENT`.
- `crates/workspace_tests/tests/vb_qi37_6_state8_setup.rs:50` asserts exactly `FUZZ_BINS_PRESENT`.

[PASS] Kani setup marker exists and is not a PASS claim:

- `crates/vb_core/src/kani.rs:3-8` states marker-only routing to State 11.
- `crates/vb_core/src/lib.rs:40-41` exports `pub mod kani` only under `#[cfg(kani)]`.

[PASS] Fuzz setup registration exists under `autobins = false`:

- `fuzz/Cargo.toml:6` has `autobins = false`.
- `fuzz/Cargo.toml:147-159` registers `capability_name_schema` and `capability_contract_schema` bins.
- `fuzz/src/bin/capability_name_schema.rs:4-6` routes stdin bytes to `fuzz_lib::fuzz_capability_name_schema`.
- `fuzz/src/bin/capability_contract_schema.rs:4-6` routes stdin bytes to `fuzz_lib::fuzz_capability_contract_schema`.

[PASS] State 8 report does not claim Kani/fuzz execution PASS:

- `.beads/vb-qi37.6/test-writer-report.md:66-70` explicitly says no `cargo kani` PASS, no `cargo fuzz run` PASS, and full formal/deep gates remain State 11/release evidence.

## Tier 1 — Execution evidence run by reviewer

[PASS] `cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_6_state8_setup` -> `2 passed; 0 failed`.

[PASS] `cargo test -p velvet-ballistics-fuzz --features fuzz --bin capability_name_schema --no-run` -> compiled.

[PASS] `cargo test -p velvet-ballistics-fuzz --features fuzz --bin capability_contract_schema --no-run` -> compiled.

[PASS] Focused behavior checks from State 8 report rerun:

- `cargo test -p vb_core capability_set_grants_exact_name_and_action --lib` -> `1 passed; 0 failed`.
- `cargo test -p vb_runtime admit_artifact_run_rejects_missing_grants_without_allocation --lib` -> `1 passed; 0 failed`.
- `cargo test -p vb_runtime admit_artifact_run_rejects_non_exact_grant_without_allocation --lib` -> `1 passed; 0 failed`.
- `cargo test -p vb_ui_model action_description_view_required_capabilities_roundtrip --lib` -> `1 passed; 0 failed`.

## Deferred tiers

Coverage, mutation, `cargo kani`, and `cargo fuzz run` are not State 9 gates for this setup review. They remain State 11/release-owned exactly as planned.

## Findings

Blocking findings: 0

No repair required before State 10. Do not treat this approval as State 11 Kani/fuzz execution evidence.

## Rerun

- owner_state: none
- rerun_from: none
- next_state: 10

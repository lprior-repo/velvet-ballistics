# Test Suite Review — vb-qi37.2.5 State 9 Retry After State 7/8 Repair

STATUS: APPROVED

## Basis

- Mandatory startup read and applied: `/home/lewis/.claude/skills/test-reviewer/SKILL.md` and conflict-winner `/home/lewis/.agents/skills/test-reviewer/SKILL.md`; both require exact assertions, deterministic execution, and meaningful hostile-input evidence.
- Evidence rules read and applied: `/home/lewis/.agents/skills/test-reviewer/references/holzmann-test-rules.md`; generated coverage must be bounded/reproducible and test commands must execute.
- Isolation verified in `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5`; reviewer made no production-code, test-code, dependency, config, or source-checkout edits.

## Evidence Executed

- Isolation: `pwd -P && rtk git status --short || true && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5"` passed; git status reported the known non-git JJ workspace condition.
- Focused banned-pattern scan over `crates/vb_core/tests/vb_qi37_2_5_boundedness_adversarial.rs` and `fuzz/src/bin/resource_budget.rs`: no matches for bare `assert!(result.is_ok())`, bare `assert!(result.is_err())`, silent `let _ =`, `.ok();`, ignored tests, sleeps, shared mutable globals, mocks, or integration-private `use crate::`.
- Focused compile/tests/proptests: `rtk cargo test --package vb_core --test vb_qi37_2_5_boundedness_adversarial --no-run` passed; focused execution passed with `cargo test: 22 passed`; extended `PROPTEST_CASES=10000` proptest run passed with `3 passed, 19 filtered out`.
- Flake/order probes: `rtk cargo nextest run --package vb_core --test vb_qi37_2_5_boundedness_adversarial --retries 2 --flaky-result fail`, `--test-threads=1`, and `--test-threads=8` all passed with `22 passed`.
- Repaired hostile-input replay: `cargo build --manifest-path fuzz/Cargo.toml --features fuzz --bin resource_budget` passed, then the exact no-heredoc Python replay executed 1000 deterministic bounded stdin cases and printed `resource_budget stdin replay PASS cases=1000`.

## INV-008 / FUZZ-RESOURCE-001 Decision

- The stdin replay plus focused malformed-byte test plus extended proptest surrogate discharges the repaired State 7 `INV-008` test obligation for the current `resource_budget` stdin-once driver.
- This approval does not claim that `cargo fuzz ... -- -runs=1000` is meaningful for the current driver; the repaired plan correctly forbids that claim unless a true `libfuzzer_sys::fuzz_target!` harness is later implemented.

## Findings

- LETHAL: none.
- MAJOR: none.
- MINOR: none.

## Mandate

- No State 8 or State 7 repair is required for the repaired test-evidence path reviewed here.

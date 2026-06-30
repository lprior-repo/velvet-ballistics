# Test Plan Review — vb-qi37.2.5 State 9 Retry After State 7/8 Repair

STATUS: APPROVED

## Basis

- Mandatory startup read and applied: `/home/lewis/.claude/skills/test-reviewer/SKILL.md` and conflict-winner `/home/lewis/.agents/skills/test-reviewer/SKILL.md`. The reviewed rules require contract parity, exact assertions, bounded generated coverage, and meaningful hostile-input evidence.
- Evidence rules read and applied: `/home/lewis/.agents/skills/test-reviewer/references/holzmann-test-rules.md`; lines 32-49 require generated coverage to be bounded/reproducible, and lines 195-210 require compile/execute evidence.
- Isolation verified by command: `pwd -P && rtk git status --short || true && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-2-5"`; path matched the required isolated workspace and git status reported the known non-git JJ workspace condition.

## Review Result

- The repaired plan explicitly states that `fuzz/src/bin/resource_budget.rs` is stdin-once and that `cargo fuzz ... -- -runs=1000` is waived as invalid evidence for `FUZZ-RESOURCE-001` until a real libFuzzer harness exists.
- The replacement gate in `test-plan.md:251-277` is truthful: it builds the current stdin driver, runs exactly 1000 deterministic bounded stdin cases, requires the exact output `resource_budget stdin replay PASS cases=1000`, and pairs it with the focused malformed-byte test plus extended proptest command.
- BDD 22 in `test-plan.md:205-210` now maps `INV-008` / `FUZZ-RESOURCE-001` to the repaired hostile-input replay and property-test surrogate, not to hollow cargo-fuzz process launch evidence.
- The prior repair guide route is satisfied: the plan chose the allowed bounded stdin/corpus replay path with explicit count, reproducible inputs, and exact acceptance text.

## Findings

- LETHAL: none.
- MAJOR: none.
- MINOR: none.

## Decision

- The repaired `FUZZ-RESOURCE-001` plan is acceptable for the current stdin-once driver. A true libFuzzer harness remains a future improvement, but it is no longer required for this repaired State 7/8 test-evidence path.

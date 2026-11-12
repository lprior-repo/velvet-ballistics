bead_id: vb-zrop
phase: 11

# Regression Diff

Baseline: `moon run :verify-standard` failed at `GATE-IGNORED-FALLIBLE-RESULTS` before reaching later Kani checks.
Attempt 1 after ignored-result repair: ignored-result gate passed, then Kani accessor harness compile failed due non-exhaustive `PathSegment` matches.
Attempt 2 after Kani harness repair: `moon run :verify-standard` PASS.
`moon ci` PASS.

Classification: PASS. Prior attempt-1 Kani failure was BLOCK_RELEASE / REQUIRED_OBLIGATION_FAIL and was repaired in State 10 attempt 2. No remaining blockers and no deferred global debt.

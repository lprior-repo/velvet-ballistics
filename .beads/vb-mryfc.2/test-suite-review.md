# Independent lifecycle test review

STATUS: REJECTED

Candidate: fa5e084c2 (full source unchanged from published lifecycle candidate).
Reviewer workspace: /home/lewis/src/isoloated/velvet-ballistics-vb-mryfc-test-review-root.

## Blocker: per-tick budget test never distinguishes budget enforcement

`crates/workspace_tests/tests/vb_c1s0_orchestration_runtime_tests.rs:672`, `runtime_respects_step_budget_per_tick`, sets budget2 but immediately reaches an asynchronous Do boundary after SetConst. Subsequent drives also stop at Do or Finish. Every assertion checks action suspension/completion or run counters; no pure chain extends beyond the configured budget. Removing the effective limit therefore leaves its asserted observations unchanged. A controlled fixture experiment replaced only budget2 with u64::MAX: the exact test still passed (1 passed, 28 filtered, actual exit0). The source was restored byte-for-byte in finally; digest recorded in evidence. This does not prove production budget enforcement is broken; it proves this named test cannot detect its absence.

Required repair: exercise a deterministic non-boundary chain longer than the configured budget; assert exact progress and lack of completion after each bounded drive, then eventual correct output/completion. Include a budget boundary such as1 versus2 so ignoring configured budget changes an assertion. Preserve the real-ticket action scenario as separate behavior if useful. No production change or weaker assertions required by this finding.

Disposition: blocker, author repair required.

## Executed evidence

Both complete assigned targets passed independently: orchestration29 and lifecycle40, 69 total, zero failures/ignored/filtered. Full stdout/stderr and actual subprocess return codes are in evidence/results.json and target logs. The controlled probe is separately recorded and is not another full-suite pass. Production and test files have no residual reviewer mutation. No broad CI/proof/performance claim follows.

## Accepted improvements and limits

Padding contracts no longer invent unrelated capabilities. Positive completions and failures use runtime-issued tickets; wrong-action rejection now asserts exact InvalidActionCompletion. Four previously ignored cases execute. Lifecycle action scheduling checks the canonical ticket event and exact run/step/action/input/output. These repairs are useful, but do not resolve the separate AskTicket authority or durable shutdown findings in other beads.

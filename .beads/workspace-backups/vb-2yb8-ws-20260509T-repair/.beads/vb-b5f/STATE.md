bead_id: vb-b5f
bead_title: Phase 1: Core types — IDs, errors, limits, source locations, diagnostics
phase: phase-1
updated_at: 2026-04-29T13:15:00Z
current_state: 1
next_gate: State 2 — codebase exploration via explore agent
claim_status: claimed via bd update vb-b5f --claim
jj_workspace: vb-b5f (created via jj workspace add)
jj_working_copy: xqovqswq
jj_parent_commit: faeacc6c
artifacts:
  STATE.md: this file
  codebase-map.md: pending — carry forward from vb-blq if still valid, else produce new
  contract.md: pending
  test-plan.md: pending
  test-plan-review.md: pending
  failing_tests: pending
  implementation.md: pending
  manual-qa-smoke.md: pending
  moon-report.md: pending
  ci-failure-category.txt: pending (if red)
  qa-report.md: pending
  qa-review.md: pending
  test-suite-review.md: pending
  red-queen-report.md: pending
  black-hat-review.md: pending
  defects.md: pending (if black-hat rejects)
  kani-report.md: pending
  kani-justification.md: pending
  architectural-drift-review.md: pending
  manual-qa-final.md: pending
notes:
  - "[2026-04-29] explore: ids.rs exists but needs SeqNo, checked_add, ZERO constants; error.rs needs CoreError rename and E0101-E0409 codes; limits.rs/span.rs/diagnostic.rs entirely missing; RunId u128 vs u64 mismatch; no Arc<Mutex> found"
blocked_by: []
retry_budget_remaining: 7

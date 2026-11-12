bead_id: vb-qi37.13.4
bead_title: cli: Structured output contract tests
phase: State 3
updated_at: 2026-05-11T00:00:00Z

# TLA+ Temporal Model Plan

## Boundary
- Temporal/workflow behavior: none added; tests spawn bounded CLI processes.
- Rust/core behavior excluded from TLA+: CLI parser/emit behavior is covered by black-box tests and static gates.
- Non-applicability rationale: this bead adds executable process contract tests; no retry, scheduler, queue, lease, or lifecycle protocol is changed.

## Evidence Command
- No TLA+ command required for bead-local scope.

## Waivers
- TLA-WAIVE-001: Temporal modeling waived for test-only CLI process contracts; owner=go-skill State 3; expiry=bead close; compensating evidence=black-box tests plus moon ci.

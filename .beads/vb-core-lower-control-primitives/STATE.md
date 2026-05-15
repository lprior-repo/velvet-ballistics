bead_id: vb-core-lower-control-primitives
bead_title: "compiler: Lower v1 control primitives from YAML AST"
phase: 11
updated_at: 2026-05-15T00:00:00Z
attempt: 1

source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /tmp/vb-ws/vb-core-lower-control-primitives

workspace_path_proof: |
  pwd -P = /tmp/vb-ws/vb-core-lower-control-primitives
  case check: /tmp/vb-ws/vb-core-lower-control-primitives is NOT equal to /home/lewis/src/velvet-ballistics
  case check: /tmp/vb-ws/vb-core-lower-control-primitives is NOT nested under /home/lewis/src/velvet-ballistics
  Isolation: VERIFIED

state: 11
status: formal_verification_passed
next_gate: State 12 (black-hat-reviewer)

state_11_results:
  clippy: PASS (No issues found)
  cargo_test: PASS (289 passed, 1 suite, 2.22s)
  formal_lanes: DISCOVERY_BLOCKED for kani/miri/verus (vb-f04l)
  regression: NONE (+42 tests, no regressions)

artifacts_written:
  - implementation.md
  - machine-gate-report.md
  - regression-diff.md
  - formal-verification-report.md
  - verification-ledger.jsonl

notes: |
  State 10 (holzman-rust): No implementation changes needed — lower_* functions already
  existed. This bead added 42 tests covering all 11 lower_* functions.

  State 11 (formal verification): All executable gates pass.
  - Clippy: PASS
  - Unit tests: 289 PASS
  - Kani/Miri/Verus: DISCOVERY_BLOCKED via vb-f04l (not a local failure)

  Next: State 12 (black-hat-reviewer)

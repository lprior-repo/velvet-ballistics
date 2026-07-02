# Bead vb-vzo9b — Delivery State

- bead_id: vb-vzo9b
- source_checkout: /home/lewis/src/velvet-ballistics
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b
- controller: femdation
- current_state: 14
- attempts: 1
- started_at: 2026-07-01T15:21:37Z
- last_state: evidence-packaging + truth-serum (state 14, assurance bundle approved)
- status: approved (state 14 closed; ready for state 15 landing)

## Routing Ledger

- routing_ledger_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/.beads/vb-vzo9b/routing-ledger.jsonl
- agent_invocation_ledger_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/.beads/vb-vzo9b/agent-invocation-ledger.jsonl
- baseline_report_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/.beads/vb-vzo9b/baseline-report.md
- global_readiness_report_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/.beads/vb-vzo9b/global-readiness-report.md
- runtime_provenance_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b/.beads/vb-vzo9b/runtime-skill-provenance.json

## State History

- state 1: go-skill (controller bootstrap)
- state 2: explore (codebase scout)
- state 3: rust-contract (domain/type contract artifacts)
- state 4: proof-planner (proof obligations, lane decisions)
- state 4b: proof-plan-reviewer (lane review, disposition: accepted)
- state 5-10: elided (test-only repair; no proof-writer/proof-reviewer/proof-to-impl/test-planner/test-writer/test-reviewer)
- state 11: holzman-rust (P1 test fix, command_results: [pass, pass, pass])
- **state 12**: formal-verifier (3 obligations PASS, formal-waivers.jsonl empty) — combined with states 13-14
- **state 13**: black-hat-reviewer (STATUS: APPROVED, 0 CRITICAL/HIGH/MEDIUM, 1 LOW + 2 LOW pre-existing + 1 DEFERRED_GLOBAL)
- **state 14**: evidence-packaging + truth-serum (STATUS: APPROVED, final decision: APPROVED for landing)

## Workspace

- jj workspace: cheap25-vb-vzo9b
- jj workspace root: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b
- jj parent commit: rsvywymk 1d6c017f (AGENTS.md round10 forward-port)
- jj current change: lmywqxvt 2288ff54 (vb-vzo9b state11: holzman-rust exact-pin)
- git remote: origin/main @ 2c8ea33c9

## State 12-14 Artifacts (with SHA-256)

- `formal-verification-report.md`: `a80144f3ce34186433961a1f07d070507c225a12b879125b724d31b979f7595f`
- `verification-ledger.jsonl`: `c77bdd971bc398576162e16d8259d35eab6bcc7d070ecef5db703aee4f4c754b` (3 rows, all PASS)
- `formal-waivers.jsonl`: `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` (empty)
- `black-hat-review.md`: `a53719743e4d29aedce424abab938575b61ce6260fcbd05b4b589a70970efb7f`
- `defects.md`: `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` (empty)
- `assurance-bundle.md`: (state 14, see SHA-256 in evidence file)
- `truth-serum-report.md`: (state 14)
- `final-evidence-decision.md`: (state 14, STATUS: APPROVED)

## Agent Invocation Ledger (8 entries)

- seq 1: go-skill-vb-vzo9b-state1
- seq 2: explore-vb-vzo9b-state2
- seq 3: proof-planner-vb-vzo9b-state4
- seq 4: proof-plan-reviewer-vb-vzo9b-state4b-attempt1
- seq 5: holzman-rust-vb-vzo9b-state11
- seq 6: formal-verifier-vb-vzo9b-state12-attempt1 (entry_hash: 627d258b8ad0f5cb25de0e2a74a162152111b01abacd2acc3c3dce0d9f05e816)
- seq 7: black-hat-reviewer-vb-vzo9b-state13-attempt1 (entry_hash: aca01d63a26c6e5927a4cff863764078872da3e96cee8269344274e2572083ba)
- seq 8: evidence-packaging-truth-serum-vb-vzo9b-state14-attempt1 (entry_hash: 3bd144c2fef3a7b436a6a228412f9e6bc83ca20053f421498edbdbdc1fe88be8)


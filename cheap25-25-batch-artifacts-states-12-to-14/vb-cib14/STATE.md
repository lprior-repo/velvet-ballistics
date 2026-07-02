# Bead vb-cib14 — Delivery State

- bead_id: vb-cib14
- source_checkout: /home/lewis/src/velvet-ballistics
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14
- controller: femdation
- current_state: 11
- attempts: 1
- started_at: 2026-07-01T15:21:36Z
- state_entered_at: 2026-07-02T00:30:00Z
- status: implementation_complete_pending_landing
- state_7_bridge_invocation: femdation-p7-proof-to-implementation-vb-cib14
- state_7_bridge_review_invocation: femdation-p7b-proof-reviewer-vb-cib14
- state_7_outputs:
  - proof-to-rust-map.md: 3185b1eac289c3a2ce8d8181fdf4d3c5373775ac7c08c1f034fba8618a08dcac
  - rust-refinement-obligations.jsonl: 9fd888c193358fc8372fab324c16542103207de1417b85b92d17e1dc498f06d3
  - proof-to-rust-review.md: 8ae7e1fa0842f99e6b790bc385f728da2176320df5e41a9ed5edf73561d4215e (STATUS: APPROVED)
- state_11_holzman_rust_invocation: femdation-p11-holzman-rust-vb-cib14
- state_11_outputs:
  - implementation.md: see `.beads/vb-cib14/implementation.md`
  - evidence/:
    - cargo-vb-runtime-storage_event.log (1 passed)
    - cargo-vb-runtime-storage_event-feature.log (6 passed, includes all PO-001..PO-007 proof artifacts)
    - cargo-vb-runtime-resumed-timestamp.log (1 passed)
    - cargo-workspace-tests-resume-replay-feature.log (3 passed, includes State 12 PENDING proptest)
    - cargo-vb-runtime-typed-error-variant.log (1 passed, PO-007 error variant shape)
    - cargo-vb-runtime-full-default.log (1807 passed / 0 failed, default features)
    - cargo-vb-runtime-full-feature.log (1812 passed / 0 failed, vb-cib14 feature)
    - cargo-vb-runtime-build-all-features.log (warning-free build)

## Routing Ledger

- routing_ledger_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14/.beads/vb-cib14/routing-ledger.jsonl
- agent_invocation_ledger_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14/.beads/vb-cib14/agent-invocation-ledger.jsonl
- baseline_report_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14/.beads/vb-cib14/baseline-report.md
- global_readiness_report_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14/.beads/vb-cib14/global-readiness-report.md
- runtime_provenance_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14/.beads/vb-cib14/runtime-skill-provenance.json

## Workspace

- jj workspace: cheap25-vb-cib14
- jj workspace root: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14
- jj parent commit: rsvywymk 1d6c017f (AGENTS.md round10 forward-port)
- git remote: origin/main @ 2c8ea33c9

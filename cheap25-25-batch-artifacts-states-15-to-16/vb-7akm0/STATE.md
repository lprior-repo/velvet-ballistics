# Bead vb-7akm0 — Delivery State

- bead_id: vb-7akm0
- source_checkout: /home/lewis/src/velvet-ballistics
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0
- controller: femdation
- current_state: 16
- attempts: 1
- started_at: 2026-07-01T15:21:36Z
- completed_at: 2026-07-02T05:22:00Z
- status: landed-closed

## Routing Ledger

- routing_ledger_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0/.beads/vb-7akm0/routing-ledger.jsonl
- agent_invocation_ledger_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0/.beads/vb-7akm0/agent-invocation-ledger.jsonl
- baseline_report_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0/.beads/vb-7akm0/baseline-report.md
- global_readiness_report_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0/.beads/vb-7akm0/global-readiness-report.md
- runtime_provenance_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0/.beads/vb-7akm0/runtime-skill-provenance.json

## State 11 Artifacts

- implementation_report_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0/.beads/vb-7akm0/implementation.md
- decision_ack_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0/.beads/vb-7akm0/decision-ack.md
- evidence_dir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0/.beads/vb-7akm0/evidence/run-001/
  - lint-src-output.log
  - lint-src-exit-code.txt (exit=0)
  - cargo-test-output.log
  - cargo-test-exit-code.txt (exit=101, see implementation.md residual risk 3)
  - allow-suppressions-after.txt
  - source-length-output.log

## State 12-14 Artifacts (formal-verifier + black-hat + evidence-packaging + truth-serum)

- formal_verification_report_path: .beads/vb-7akm0/formal-verification-report.md
- black_hat_review_path: .beads/vb-7akm0/black-hat-review.md
- assurance_bundle_path: .beads/vb-7akm0/assurance-bundle.md
- truth_serum_report_path: .beads/vb-7akm0/truth-serum-report.md
- final_evidence_decision_path: .beads/vb-7akm0/final-evidence-decision.md (STATUS: APPROVED)

## State 15 Artifacts (landing)

- landing_report_path: .beads/vb-7akm0/landing-report.md
- evidence_dir: .beads/vb-7akm0/evidence/landing-state15/
  - bd-close-output.log, bd-close-exit-code.txt (exit=0)
  - bd-dolt-push-output.log, bd-dolt-push-exit-code.txt (exit=0)
  - bd-show-after-close.log (status CLOSED)
  - bd-dolt-status-after.log
  - jj-diff-stat.log, jj-work-commit.log
- bd_status: CLOSED (Updated 2026-07-02)
- dolt_push: complete (origin doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics)

## State 16 Artifacts (cleanup)

- cleanup_report_path: .beads/vb-7akm0/cleanup-report.md
- deferred_backlog: xtask ~173 unreachable_pub cascade; diag_codes CODE_* consts; diag_convert vestigial suppression
- preexisting_defects: PO-TEST-001 (vb_core proptest), PO-EXTERN-001 (production_inner drift) — separate beads

## Workspace

- jj workspace: cheap25-vb-7akm0
- jj workspace root: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0
- jj commit (work): qvlkvsyy d4476627 (vb-7akm0: p11-holzman-rust — remove 24 unreachable_pub suppressions (xtask binary root excluded due to cascade))
- jj parent commit: orvzyxqt 7617a003 (no description set)
- git remote: origin/main @ 2c8ea33c9
- work commit retained on bookmark cheap25-vb-7akm0 for batch integration

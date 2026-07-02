# Bead vb-uwxct — Delivery State

- bead_id: vb-uwxct
- source_checkout: /home/lewis/src/velvet-ballistics
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-uwxct
- controller: femdation
- current_state: 14
- attempts: 1
- started_at: 2026-07-01T15:21:37Z
- completed_at: 2026-07-02T03:25:00Z
- status: closed-pending-landing (states 12, 13, 14 all approved)

## Routing Ledger

- routing_ledger_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-uwxct/.beads/vb-uwxct/routing-ledger.jsonl
- agent_invocation_ledger_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-uwxct/.beads/vb-uwxct/agent-invocation-ledger.jsonl
- baseline_report_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-uwxct/.beads/vb-uwxct/baseline-report.md
- global_readiness_report_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-uwxct/.beads/vb-uwxct/global-readiness-report.md
- runtime_provenance_path: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-uwxct/.beads/vb-uwxct/runtime-skill-provenance.json

## Workspace

- jj workspace: cheap25-vb-uwxct
- jj workspace root: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-uwxct
- jj parent commit: rsvywymk 1d6c017f (AGENTS.md round10 forward-port)
- jj working copy: rkttsxlp a092e4fe (state 11 — vb-uwxct: p11-holzman-rust — tighten max-sequence tests)
- git remote: origin/main @ 2c8ea33c9

## State Path (states 1 → 14)

| State | Skill | Status | Artifact |
|-------|-------|--------|----------|
| 1 | go-skill | completed | STATE.md, runtime-skill-provenance.json, baseline-report.md |
| 2 | explore | completed | codebase-map.md, delivery-scope.jsonl |
| 3 | rust-contract | completed | contract.md, domain-model.md, error-taxonomy.md, type-contracts.md, workflow-model.md, hazard-analysis.md, boundary-map.md, proof-seeds.jsonl, traceability-matrix.jsonl |
| 4 | proof-planner | completed | proof-strategy.md, verifier-lane-matrix.md, verifier-lane-decisions.jsonl, proof-coverage-matrix.md, proof-obligations.planned.jsonl, trusted-base-plan.md, waiver-candidates.jsonl |
| 4b | proof-plan-reviewer | approved | verifier-lane-review.jsonl, proof-plan-review.md (STATUS: APPROVED) |
| 11 | holzman-rust | delivered | implementation.md, 4 file changes (Cargo.toml, lib.rs, kani_typed_partitioned_ids.rs, restate_journal_tail_scan_fallback_tests.rs), 7 evidence logs |
| 12 | formal-verifier | approved | formal-verification-report.md (STATUS: APPROVED), verification-ledger.jsonl (4 rows), formal-waivers.jsonl (empty) |
| 13 | black-hat-reviewer | approved | black-hat-review.md (STATUS: APPROVED), defects.md (empty) |
| 14 | evidence-packaging | approved | assurance-bundle.md, truth-serum-report.md, final-evidence-decision.md (STATUS: APPROVED) |

## Final Closure

- 4/4 proof obligations PASS
- 0 FAIL_LOCAL, 0 FAIL_REGRESSION, 0 WAIVED
- 1 FAIL_GLOBAL documented (workspace-wide strict clippy debt; pre-existing; out of scope)
- Production encoder at `crates/vb_storage/src/keys.rs:480-496` UNTOUCHED
- 0 VACUUM Verus proofs
- 0 VACUUM/fabricated evidence
- All reviewer findings use canonical `finding/v1.disposition` (`owner_approved_debt`)
- Bead ready for landing

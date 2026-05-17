# Mutation Classification Audit: vb-scxh

STATUS: PASS_NON_ADEQUACY

## Audited Artifacts

- `.beads/vb-gvmt/mutation-report.md`
- `.beads/vb-gvmt/verification-ledger.jsonl`

## Raw Markers

- Command marker present: `cargo mutants -p vb_codegen -f "crates/vb_codegen/src/lib.rs" -F 'emit_(journal_contract|generated_runtime_api|run_until_blocked|action_resume_api|ask_resume_api|action_completion_spec|ask_answer_spec)' --in-place --timeout 60 --baseline skip -- post_`.
- Status marker present: `FAIL_UNVIABLE / DEFERRED`.
- Exact unviable marker present: `35 mutants tested in 34s: 35 unviable`.
- Ledger marker present: `kind=mutation`, `status=FAIL_UNVIABLE`, `evidence=35 mutants tested in 34s: 35 unviable`.

## Classification

- `MUT-SCXH-001`: `PASS` for classification integrity only.
- `ERR-SCXH-007`: `PASS`; no adequacy PASS is claimed from `FAIL_UNVIABLE`.
- Mutation adequacy remains unsatisfied/deferred; this is not mutation adequacy evidence.

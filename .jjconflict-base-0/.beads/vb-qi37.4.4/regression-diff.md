bead_id: vb-qi37.4.4
phase: State 8 classification after State 13 refactor
updated_at: 2026-05-11

# Regression Diff

## Classification
- `DEFERRED_GLOBAL`

## Evidence
- Baseline already recorded isolated JJ/Moon global failure: absent Git ref `main`.
- Previous State 8 artifact already classified unrelated workspace formatting/global lint/feature-powerset debt as deferred global.
- Current `moon ci` output path: `/home/lewis/.local/share/opencode/tool-output/tool_e19d661b8001uZi7XItEOpIKJ6`.
- Current red items remain outside the revised delivery scope in `delivery-scope.jsonl`: unrelated fmt diffs, `vb_proof_kernels` lint, and `vb_ui_model` feature-powerset no-std failures.
- Bead-local focused tests passed; `moon run :test` reported `9831 tests run: 9831 passed, 0 skipped`; `source-length` passed.

## Follow-up text
- Fix unrelated workspace formatting/lint/feature-powerset debt separately; do not block this bead-local architectural repair on existing global debt.

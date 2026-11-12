bead_id: vb-qi37.15.2
phase: State 8

# Regression Diff

- `moon ci` failure matches State 1 baseline exactly: git cannot resolve `main` revision inside isolated JJ workspace.
- Classification: DEFERRED_GLOBAL.
- Follow-up text: Fix Moon/JJ isolated workspace base revision detection so `moon ci` can run when no local `main` ref exists.
- Bead-local scoped submit tests pass; the prior Fjall lock defect is fixed in scoped smoke.

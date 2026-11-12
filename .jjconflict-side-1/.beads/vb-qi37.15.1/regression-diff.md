bead_id: vb-qi37.15.1
phase: State 8

# Regression Diff

- `moon ci` failure matches State 1 baseline exactly: git cannot resolve `main` revision inside isolated JJ workspace.
- Classification: DEFERRED_GLOBAL.
- Follow-up text: Fix Moon/JJ isolated workspace base revision detection so `moon ci` can run when no local `main` ref exists.
- Bead-local scoped simulate tests pass.
- Process caveat: State 5 red was not separately captured for the schema assertion; record as TDD process gap for reviewers.

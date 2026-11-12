# Regression Diff

Current isolated change repairs the prior KANI-PARITY-006 failure by aligning `vb_compile` with `vb_validate` for side-effecting DeterministicPure contracts. Scoped clippy and 3070 scoped tests pass. FUZZ-ARTIFACT-011 remains a tooling waiver only.

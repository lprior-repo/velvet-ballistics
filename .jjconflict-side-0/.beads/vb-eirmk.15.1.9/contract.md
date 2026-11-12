# Contract: successful frame state marks preserve collateral

- Preserve the existing two test names, all 64 source/target pairs, all eight out-of-bounds checks, the independent v1 transition relation, and exact negative error assertions.
- For every accepted mark in the existing matrix, including idempotent marks, the resulting frame must equal the pre-call frame with only `StepIdx::ZERO` changed to the requested target state.
- Build that expected frame by cloning the pre-call frame and updating it through the existing test-only safe `set_state` helper; do not use a production mark method as the expected-value oracle.
- Keep the change limited to the matrix test and this contract; add no production changes, new dependencies, gates, or configuration.
- Keep every helper at or below 25 logical lines including its signature.
- Verify with pinned rustfmt, the requested scoped `vb_core` nextest filter, and `moon run :test-integrity`; report each native exit status and the observed case count.

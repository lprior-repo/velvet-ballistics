# Black Hat Review: vb-qi37.12.4

STATUS: APPROVED

## Findings

- No blocking findings for the bead scope.

## Attacks Performed

- Contract parity: user required all direct `DISCARD-*` violations handled. Direct gate now reports `NoViolationFound` and affected call sites use explicit assertions, returned errors, or error reporting.
- Fail-closed behavior: malformed and overbroad exception fixture rows exit 3.
- Verification propagation: `moon run :verify-standard` invokes the direct gate and exits 0 only after standard lint/unit/Kani lanes pass.
- Regression risk: touched packages have passing tests; excluded `vb_ui` compile debt is isolated and disclosed.

## Bitter Truth

The repair is acceptable because it removes silent fallible-result discards instead of hiding them behind allowlists. The remaining `vb_ui` manifest failure is real debt, but it predates and exceeds this static-gate bead.

bead_id: vb-qi37.15.1
bead_title: cli: Add simulate command
phase: State 3
updated_at: 2026-05-11T00:00:00Z

# TLA+ Temporal Model Plan

No TLA+ model is required. `simulate` is intentionally non-temporal from a runtime perspective: it enumerates compiled workflow nodes and must not mutate run lifecycle or storage state. Compensating evidence: black-box no-DB-side-effect tests and CLI structured output tests.

bead_id: vb-qi37.13.4
bead_title: cli: Structured output contract tests
phase: State 3
updated_at: 2026-05-11T00:00:00Z

# Verification Layers

- PRE-001, PRE-002 -> cargo integration tests.
- POST-001, POST-002, POST-003 -> black-box CLI assertions in `crates/velvet_ballistics/tests/cli_integration.rs`.
- INV-001, INV-002 -> black-box assertions plus static scan/moon ci.
- ERR-001, ERR-002 -> error-path CLI tests.
- Release critical workspace sensor -> `moon ci`, with pre-existing global `main` revision failure recorded from baseline.

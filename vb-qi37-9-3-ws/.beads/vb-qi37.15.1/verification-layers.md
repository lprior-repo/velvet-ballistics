bead_id: vb-qi37.15.1
bead_title: cli: Add simulate command
phase: State 3
updated_at: 2026-05-11T00:00:00Z

# Verification Layers

- PRE-001, ERR-001 -> black-box CLI test with missing path.
- PRE-002, ERR-002 -> black-box CLI test with invalid workflow.
- POST-001, POST-002 -> black-box CLI text/json/jsonl tests.
- POST-003, INV-002 -> no side-effect test proving no DB path required/written.
- INV-001 -> unit/black-box assertions on deterministic totals and step descriptions.

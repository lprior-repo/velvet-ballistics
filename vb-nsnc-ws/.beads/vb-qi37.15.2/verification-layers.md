bead_id: vb-qi37.15.2
bead_title: cli: Add submit command and job ledger
phase: State 3
updated_at: 2026-05-11T00:00:00Z

# Verification Layers

- PRE-001, ERR-001 -> black-box missing input/workflow tests.
- PRE-002, ERR-003 -> temp journal and unwritable path tests.
- PRE-003, ERR-004 -> parser/CLI unknown durability tests.
- POST-001, INV-001 -> black-box submit then inspect/events tests; TLA+ ledger ordering obligation.
- POST-002 -> structured/text output assertions.
- POST-003 -> persisted metadata inspection assertions.

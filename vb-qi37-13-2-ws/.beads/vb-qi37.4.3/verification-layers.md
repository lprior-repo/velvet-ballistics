# Verification Layers

## Boundary
- TLA+-owned: submit/admit/persist/ack/active lifecycle ordering.
- Rust shell: Fjall/storage journal append and recovery I/O, covered by integration tests and manual QA.
- Verus waiver: `WAIVER-VERUS-HEADER-ORDER`; owner `Lewis`; expires when a pure typestate submit-core is introduced or before release closure of parent `vb-qi37.4`; limitation: current property crosses fallible storage I/O and mutable runtime shell; compensating evidence: `TLA-ACK-001`, `TEST-DUR-001`, `REC-HEADER-001`, `moon ci`, and final manual QA.

## Layer Assignment
- PRE-001 -> `TEST-PRE-001` duplicate/unique run fixture coverage.
- PRE-002 -> `TEST-PRE-002` admission rejection before state allocation.
- POST-001 -> `TLA-ACK-001` + `TEST-DUR-001`.
- POST-002 -> `REC-HEADER-001`.
- POST-003 -> `TEST-DUR-001`.
- INV-001 -> `TLA-ACK-001` + `REC-HEADER-001`.
- INV-002 -> `TLA-ACK-001` + `TEST-DUR-001`.
- Release gate -> `REL-GATE-001`.

## Waivers
- `WAIVER-VERUS-HEADER-ORDER`: Verus not applied to I/O shell ordering in this bead. Owner: Lewis. Expiry/follow-up: parent `vb-qi37.4` release closure or introduction of pure submit typestate kernel, whichever comes first. Compensating evidence: TLA+ lifecycle model obligation, integration failure-injection tests, recovery tests, machine gates, and manual QA.

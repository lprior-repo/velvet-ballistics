# TLA+ Report — vb-f7k6 State 11 Retry

STATUS: PASS

- command: `tlc -config verification/tla/TimerWheel.cfg verification/tla/TimerWheel.tla`
- exit: 0
- result: PASS
- TLC version: 2.19 of 08 August 2024
- states generated: 4,209,522
- distinct states: 315,211
- states left on queue: 0
- complete depth: 16
- temporal checks: completed for current and complete state spaces
- final TLC line: `Model checking completed. No error has been found.`

## Obligations Covered

- TLA-TW-001: bounded deadline arithmetic overflow does not wrap.
- TLA-TW-002: insert/replacement one-active-timer and index consistency.
- TLA-TW-003: cancel removes run/deadline indexes.
- TLA-TW-004: due-only firing, destructive removal, progress property.
- TLA-TW-005: valid/stale delivery and no resurrection.
- TLA-TW-006: terminal/no-timer-mutation and deadlock-free idle semantics.

# Verification Layers - vb-0253.2

## Boundary
- **Verus-owned kernel**: Queue invariants, FIFO ordering
- **TLA+ temporal model**: Queue protocol (submit/recv/disconnect)
- **Theorem projection**: None needed
- **Runtime shell**: MemoryIngress bounded queue

## Layer Assignment
- INV-001 -> verus + kani
- INV-002 -> verus
- ERR-FULL -> unit test
- ERR-DISCONNECTED -> unit test

## Waivers
- None

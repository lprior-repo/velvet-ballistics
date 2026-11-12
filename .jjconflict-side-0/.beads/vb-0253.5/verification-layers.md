# Verification Layers - vb-0253.5

## Boundary
- **Verus-owned kernel**: StepState enum, transition validity, terminal/non-terminal classification
- **TLA+ temporal model**: State machine protocol
- **Theorem projection**: None needed
- **Runtime shell**: Runtime state usage

## Layer Assignment
- INV-001 -> verus
- INV-002 -> verus + kani + tla-plus
- INV-003 -> verus + unit test

## Waivers
- None

# Verification Layers — vb-2yb8

## Layer 0: Unit Tests (mandatory)
- Each matrix row has a corresponding unit test linking primitive → event → replay
- Gate test fails if any row is missing

## Layer 1: Integration Tests (mandatory)
- End-to-end shard handler tests verify persistence-before-ack for each command path
- Tests use VolatileRuntimeJournal to assert event ordering

## Layer 2: Property Tests (recommended)
- Proptest: random primitive sequences produce valid matrix rows
- Proptest: replay from arbitrary event stream recovers deterministic state

## Layer 3: Compile-Time Checks (optional)
- Static const matrix: missing primitives cause compile errors if using a macro
- For now: runtime gate test is sufficient

## Layer 4: CI Gate (mandatory)
- `moon run :test` includes durability matrix verification
- `moon run :ci` fails if matrix is incomplete

## Waivers
- None. This is P0 release-blocking.

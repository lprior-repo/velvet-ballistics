bead_id: vb-zo9d
bead_title: cli/storage: Report journal trim eligibility in doctor
phase: 3
updated_at: 2026-05-09T20:30:00Z

# Verification Layers

## Layer Assignment

| Clause | Unit Tests | Integration Tests | Property Tests | Kani | Miri | Manual QA |
|---|---|---|---|---|---|---|
| P1-P3 (preconditions) | yes | yes | - | - | - | yes |
| PO1-PO5 (diagnostic output) | yes | yes | - | - | - | yes |
| PO6 (no mutation) | yes | yes | yes | - | yes | yes |
| PO7-PO8 (exit codes) | yes | yes | - | - | - | yes |
| I1 (doctor read-only) | yes | yes | yes | - | yes | yes |
| I2 (structured/text parity) | yes | yes | - | - | - | yes |
| I3 (blockers fail closed) | yes | yes | yes | - | - | yes |
| I4 (pure diagnostic) | yes | yes | yes | yes | yes | yes |

## Defense-in-Depth

1. **Unit tests** cover each `TrimEligibility` variant and `TrimBlocker` variant.
2. **Integration tests** verify doctor JSON output contains the new check.
3. **Property tests** (proptest) verify that running the diagnostic N times produces identical results.
4. **Kani** verifies no panic paths in the diagnostic method (bounded).
5. **Miri** verifies no undefined behavior in the diagnostic scan loop.
6. **Manual QA** verifies real doctor output on a populated journal.

## Waiver Notes

- No Lean proof obligations (not a pure kernel, involves I/O scanning).
- No fuzz/Bolero obligations (interface is deterministic read-only).
- No Loom/Lockbud obligations (no concurrent mutation in diagnostic path).

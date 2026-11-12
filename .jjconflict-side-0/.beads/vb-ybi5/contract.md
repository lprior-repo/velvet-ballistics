STATUS: APPROVED

Requirements:
- R1: `verify-standard` ignored-fallible scanner must report `NoViolationFound` for scoped Kani file.
- R2: Kani mismatch harnesses must not swallow unexpected `RecoveryError` variants.
- R3: Repair must not use a scanner allow marker or weaken proof assertions.

Invariants:
- First matching digest entry remains accepted.
- Second mismatching digest entry returns the exact corresponding mismatch ID.

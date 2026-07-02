bead_id: vb-zrop
phase: 3

# Verification Layers

- REQ-001 -> static-scan + gauntlet-standard: `bash scripts/check-ignored-fallible-results.sh`; `moon run :verify-standard`.
- REQ-002 -> diff review: no edits to `scripts/check-ignored-fallible-results.sh` or Moon gate config.
- REQ-003 -> focused compile/check via canonical gate.
- REQ-004 -> diff review and dependency unchanged check.

Waivers: TLA+/Verus/Lean/Kani/Loom/Miri are not applicable because no algorithmic, temporal, concurrency, unsafe, parser, arithmetic, or public API behavior changes are in scope.

# Deferred UI / Makepad Command Center Track

The native Makepad command center is intentionally outside the current Backend /
IR Interpreter Complete milestone.

## Current Status

- Current product interface: CLI, direct Rust API, binary IPC, structured YAML/Postcard outputs.
- Current backend requirement: every future UI artifact must first exist as a CLI-emittable typed artifact.
# allow-removed-crate: deferred-scope doc enumerates the removed UI model crate as a release blocker
- Current release blockers exclude Makepad, `vb_ui_model`, UI screenshots, design tokens,
  overlap gates, UI perf gates, and UI accessibility gates.

## Deferred Scope

The following remain future work:

# allow-removed-crate: deferred-scope doc enumerates the removed UI model + UI app crates
# allow-removed-crate: deferred-scope doc enumerates the removed UI model + UI app crates
1. `vb_ui_model` typed artifact crate.
# allow-removed-crate: deferred-scope doc enumerates the removed UI model + UI app crates
2. `vb_ui_makepad` native Makepad app crate.
3. `velvet-ballistics ui --db <path>`.
4. `velvet-ballistics ui --socket <path>`.
5. `velvet-ballistics ui --demo-fixture <fixture>`.
6. Makepad app shell, design tokens, graph canvas, replay theater, incident console,
   action registry, storage doctor, AI context panel, bounded motion, snapshots, and UI hardening.
7. Figma/reference asset integration.
8. UI snapshot, overlap, clipping, spelling, token, accessibility, redaction, and performance gates.

## Future Screen Set

The preserved future command-center screen taxonomy is:

1. Execution Observatory Overview.
2. Workflow Graph Authoring.
3. Execution Details Graph View.
4. Verification Certificate View.
5. Replay Theater.
6. Incident Failure Console.
7. Action Registry / Contract Inspector.
8. Storage / Journal Doctor + AI Context.

## Reactivation Contract

UI may return to the master scope only through dedicated reactivation beads. Those beads must prove:

- every displayed fact is emitted by backend typed artifacts first,
- CLI/UI schema parity exists,
- secret values are redacted or summarized,
- Makepad dependencies do not enter runtime core crates,
- screenshots are deterministic,
- animation is bounded and can be frozen,
- UI code cannot mutate runtime truth outside explicit CLI/API lifecycle commands.

Until then, UI and Makepad are documentation-only future tracks.

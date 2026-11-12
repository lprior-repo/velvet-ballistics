# Main landing contract: vb-eirmk.13.13

The user requested all session-owned changes on main. This integration uses origin/main 69bcabd8fba0250d2f6dbd3953fb2604a8ba1178 and the reviewed source family ce6646a421bcb684128364b281091605162a6917, whose common base is 1dbcbe96cacdc0fad91edbba641f7ad3ad542fb7. Component Rust behavior remains governed by its existing bead contracts; this document records the landing boundary.

Preserve upstream helper operand order and lifecycle registrations. Keep the upstream split shard types module; port the canonical TimerWheel and step/iteration-order sidecar into its private parts. Preserve timer authority, checked capacity/generation failures, peek-before-commit ownership, journal-before-mutation behavior, snapshot selection/order, and existing public APIs and persisted formats.

All implementation, composition, reviews, and verification stay in isolated colocated Git/JJ workspaces. The detached coordination checkout and its existing working/index changes are excluded from commits and source writes. Its captured file fingerprints and index semantics must be preserved. Failed local-model prototypes are evidence, not landing inputs.

Protected files remain unchanged until explicit user approval of concrete reviewed patches: the replay-test relocation ledger, the nightly probe output paths, and recognition of an exact file-level test guard in the panic scanner. No test is removed or weakened, no gate is disabled, no suppression or timeout increase is permitted.

Acceptance requires exact component and final composition review bindings, raw repository gate commands and exits, canonical moon ci evidence with every unexecuted or failed obligation retained as unaccepted, and no introduced test/lint/build failures. The final main ref and remote must point to the reviewed integrated source through a non-force update, with the upstream and source packet accounted for.

Literal-text Has compilation and versioned symbol-payload storage remain proposed work; no unapproved API/IR-format change is included. This landing does not itself declare all master requirements, Kani/Verus obligations, dynamic fuzzing, benchmarks, or phase acceptance complete.

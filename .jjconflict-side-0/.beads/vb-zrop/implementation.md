bead_id: vb-zrop
bead_title: quality: fix verify-standard ignored fallible result gate
phase: 10
updated_at: 2026-05-18T00:00:00Z
attempt: 1-of-7

# Implementation

Holzman Rust references read:
- /home/lewis/.agents/skills/holzman-rust/SKILL.md
- /home/lewis/.claude/skills/holzman-rust/SKILL.md
- /home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md
- /home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md
- /home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md
- /home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md
- /home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md
- /home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md

Changes:
- Replaced ignored cleanup Results with explicit `if let Err(...)` handling in scoped Drop impls and best-effort CLI write fallbacks.
- Replaced `.ok()` fixture setup calls with explicit assertions / one fixture write expect in source-tree tests.
- Replaced `let _ = rx.try_recv()` with an assertion on the drained warning contents.

No scanner, Moon config, dependencies, public APIs, or runtime product behavior changed.

Attempt 2 repair: `moon run :verify-standard` progressed past ignored-result gate and exposed Kani non-exhaustive-match compile errors in `vb_validate` Gate 8 harnesses. Added explicit wildcard arms using `kani::assume(false)` to satisfy `#[non_exhaustive] PathSegment` matching without weakening the proof harness.

bead_id: vb-ib8i
phase: 3
updated_at: 2026-05-17T22:07:00Z
attempt: 1-of-7

# Contract

REQ-001: The canonical `moon ci --force --summary normal` gate shall pass for fmt/check and no longer stop at the vb-c3k9-unrelated blockers.

REQ-002: Repairs shall preserve runtime behavior; formatting-only changes shall not alter semantics.

REQ-003: Stale benchmark code shall compile against current public APIs rather than relying on removed fields, wrong ID widths, or missing direct dev-dependencies.

REQ-004: Fuzz support code shall obey zero `expect` in linted source paths.

Invariant: no excluded beads (`vb-c3k9`, `vb-8ma2`, `vb-hxm0`, `vb-hjvq`, `vb-ogwh`) are claimed or edited.

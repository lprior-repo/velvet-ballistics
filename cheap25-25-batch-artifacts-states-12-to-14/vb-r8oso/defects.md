bead_id: vb-r8oso
bead_title: Storage: enforce next-sequence-at-write before durable append (P1 bug)
phase: 13
updated_at: 2026-07-01T22:00:00Z
attempt: 1-of-1

No black-hat defects requiring reroute. All eight attack vectors (silent rewrite, variant arm omission, diagnostic code conflict, C-6 test regression, Kani feature isolation, downstream caller breakage, no-panic contract, key-only lookup discipline) pass. See black-hat-review.md.

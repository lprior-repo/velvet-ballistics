bead_id: vb-ib8i
phase: 7
updated_at: 2026-05-17T22:09:00Z
attempt: 1-of-7

Test plan:
1. Reproduce baseline CI failure.
2. Repair compile/lint/fmt blockers.
3. Run canonical `moon ci --force --summary normal` and require every action to pass.

bead_id: vb-tw3b
bead_title: expr: Bytecode vs generated Rust parity evidence
phase: 1
updated_at: 2026-05-18T00:00:00Z
attempt: 1-of-7

# Baseline report

Source checkout: `/home/lewis/src/velvet-ballistics`
Isolated workspace: `/tmp/opencode/go-skill-vb-tw3b-close`

Captured evidence:

- `jj workspace add /tmp/opencode/go-skill-vb-tw3b-close --name go-skill-vb-tw3b-close -m "go-skill vb-tw3b dependency closure"` succeeded.
- `pwd -P` in isolated workspace returned `/tmp/opencode/go-skill-vb-tw3b-close`.
- `bd show vb-tw3b` from source checkout showed bead status `BLOCKED`, title `expr: Bytecode vs generated Rust parity evidence`, dependencies all closed, and blocker edge to `vb-qi37.23`.
- `bd context` from source checkout showed server mode backend reachable at `127.0.0.1:40763`.
- `bd context` from isolated jj workspace failed because `bd` cannot resolve a Git repository root in this jj-only workspace; bead lifecycle mutations must be performed by landing-skill/source bd context after truth-serum approval.

No production/test/proof code changed during baseline capture.

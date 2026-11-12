# Baseline Report

Date: 2026-05-17

Workspace:

- Source checkout: `/home/lewis/src/velvet-ballistics`
- Isolated workspace: `/tmp/opencode/go-skill-vb-qi37-10`
- Current jj workspace revision: `xxoyykps 795f4f64 (empty) go-skill vb-qi37.10 final IR coverage`
- Parent: `rzqkqlvx 9c1fe5c6 main | docs: expand Xtask PRD policy`

Baseline command evidence:

- `jj status`: working copy has no changes.
- `jj diff --stat`: `0 files changed, 0 insertions(+), 0 deletions(-)`.
- `bash scripts/check-beads-server-mode.sh`: `beads server-mode check passed`.
- `bd --db "/home/lewis/src/velvet-ballistics/.beads/dolt" show vb-qi37.10 --json`: succeeded from the isolated workspace; output was large and captured by OpenCode tool log.

Master-doc baseline:

- `velvet-ballistics-MASTER.md` lines 748-853 require Fjall/Postcard persistence envelopes, durable journal/snapshot records, bounded decode order, and typed storage/decode errors.
- Lines 1445-1499 define mandatory implementation phases; `vb-qi37.10` maps to Phase 32 generated Rust mode and unblocks Phase 33+ parity evidence.
- Lines 1500-1514 state current remaining gaps: generated Rust is not yet accepted for full final IR, and storage/runtime/API surfaces need executable evidence rather than API existence alone.
- Lines 1766-1910 require bead-scoped evidence before closure.

No code, test, proof, or config edits existed before State 2.

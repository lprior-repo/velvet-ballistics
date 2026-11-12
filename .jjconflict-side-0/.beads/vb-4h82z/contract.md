# Contract — vb-4h82z: Migrate beads database naming to `velvet_ballistics`

## Goal

Canonical naming (`AGENTS.md`) requires Bead database `velvet_ballistics`.
The live tracker runs on Dolt database `velvet-ballistics` (hyphen, requires
backtick-quoting in SQL). Migrate the live database to `velvet_ballistics`
(underscore) with zero tracker-history loss, then point metadata at it.

## Non-goals / prohibitions

- No rename of metadata to an empty or nonexistent database.
- No deletion of the live `velvet-ballistics` database (kept as fallback until landing).
- No switch to embedded mode; server mode only (`scripts/check-beads-server-mode.sh` must pass).
- No pinned `dolt_server_port` in metadata (separate gate fix vb-fqv81 owns the port question).
- No `bd dolt stop`/restart of the shared managed server (sibling agents are active on it).
- No force-push, no `bd dolt push` from this bead (coordinator owns remote sync at land).
- No new dependencies; no unsafe/unwrap/expect/panic/todo/unimplemented/dbg in any touched Rust.

## Procedure

1. Baseline source DB `velvet-ballistics` via `bd sql`:
   `dolt_log` count + `main` hash, per-status issue counts, `dolt_remotes`.
2. Live history-preserving copy while the server runs:
   `CALL DOLT_CLONE('file://<datadir>/velvet-ballistics', 'velvet_ballistics')`
   (file-path clone copies all refs/history; no restart, no dump/load loss).
3. Verify target `velvet_ballistics`: identical `dolt_log` count, identical
   `main` hash, identical per-status issue counts, origin remote present.
   Verify end-to-end reads with `BEADS_DOLT_DATABASE=velvet_ballistics bd …`
   (env override; coord-checkout metadata stays untouched until landing).
4. Landable repo change (uncommitted, coordinator lands):
   `.beads/metadata.json`: `dolt_database` → `velvet_ballistics`, mode stays
   `server`, no port key, same `project_id`.

## Acceptance

- `bd sql` against `velvet_ballistics`: issue counts equal baseline;
  `dolt_log` count equal; `main` hash equal to source hash at clone time.
- `BEADS_DOLT_DATABASE=velvet_ballistics bd stats` succeeds, total 3106.
- `scripts/check-beads-server-mode.sh` passes in the worktree.
- Old `velvet-ballistics` database still present (rollback path).
- Cutover note: siblings keep writing to the old DB until landing, so the
  coordinator must re-sync (re-clone or file-remote pull) immediately before
  flipping metadata at land time; this bead records source hash + counts.

## Baselines (source `velvet-ballistics`, 2026-11-03)

- `dolt_log`: 8873 commits; branch `main` @ `41to8gh54k6s98u4u0qura9c7tkdg9aj`
- issues: blocked 8, closed 2778, in_progress 297, open 23 (total 3106)
- remotes: origin → `https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics`
- server: 127.0.0.1:37225, PID 35142, data `/home/lewis/src/velvet-ballistics/.beads/dolt`
- `bd doctor`: 66 passed, 10 warnings (pre-existing), 0 errors

## Migration record (executed 2026-11-03, worker fleet-w9)

Method: live SQL-only copy, no server restart. `DOLT_CLONE` from the live
repo path failed (`no Dolt data` — server-layout dirs are not file remotes),
as did `DOLT_FETCH` from a file remote (`no branches found`). Working path:
`CALL DOLT_BACKUP('add','mig','file:///tmp/vb-mig-bak')` +
`CALL DOLT_BACKUP('sync','mig')` on the source, then
`CALL DOLT_CLONE('file:///tmp/vb-mig-bak','velvet_ballistics')`.
Clone inherited `origin → file:///tmp/vb-mig-bak`; replaced with the exact
source origin URL (dolthub remote name stays `velvet-ballistics`, external).
Backup entry `mig` removed from source afterwards; `/tmp` scratch removed.

Verification (target vs source, same instant): `main` hash identical
(`tjejtuiadnua4319ju79fo4oqv7t3rl2`), `dolt_log` 8875 = 8875, issues
blocked 8 / closed 2778 / in_progress 299 / open 21 on both (total 3106).
End-to-end: coord metadata flipped to `velvet_ballistics`, `bd stats`
(total 3106), `bd show vb-4h82z`, `bd history vb-4h82z --limit 3` (claim
commits present) all green, then metadata restored byte-identical
(`git status -- .beads/metadata.json` clean in coord).
`scripts/check-beads-server-mode.sh` passes in this worktree.

## Cutover note for coordinator

Siblings keep writing to `velvet-ballistics` until landing. Before flipping
coord metadata at land: re-run the backup+clone (or file-remote pull) to
catch up, re-verify hash + counts, keep old DB as rollback, do NOT
`bd dolt push` from this bead. Landing diff: `.beads/metadata.json`
(`dolt_database` → `velvet_ballistics`) + this contract.

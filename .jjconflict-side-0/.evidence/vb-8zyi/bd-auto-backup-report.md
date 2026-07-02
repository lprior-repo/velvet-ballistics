# vb-8zyi bd auto-backup report

## Root cause

`bd` is correctly configured for Dolt server mode in `.beads/metadata.json` with `dolt_mode: server` and database `velvet_ballistics` on `127.0.0.1`.

The recurring warning was not caused by embedded-mode selection or by the main Dolt remote. The local Dolt repository has an auto-backup remote named `backup_export` configured in `.beads/dolt/velvet_ballistics/.dolt/repo_state.json`:

```text
backup_export -> file:///home/lewis/src/velvet-ballistics/.beads/backup
```

Historical Dolt server log entries show `backup_export` sync failures while Dolt was writing temporary spill files under `/tmp/buffered_file_byte_sink_*`:

```text
write /tmp/buffered_file_byte_sink_*: disk quota exceeded
```

At diagnosis time `/tmp` and the workspace both had free space, no live `/tmp/buffered_file_byte_sink_*` files remained, and the local backup directory existed as ignored runtime state. The failure was stale local auto-backup hygiene/runtime state around the `backup_export` file backup, not a current server-mode or remote-sync failure.

## Actions

No destructive cleanup was performed.

Confirmed server-mode operation with `bash scripts/check-beads-server-mode.sh`.

Confirmed `.beads/backup/`, `.beads/dolt/`, and `.beads/dolt-server.log` are ignored Git runtime state and are not staged for commit.

Confirmed `.beads/embeddeddolt/` trap directory is absent.

Ran `bd dolt pull` and `bd dolt push` successfully to exercise current remote sync and local auto-backup behavior. Subsequent acceptance probes emitted no auto-backup warning.

Used `bd create --dry-run` for the create acceptance check to avoid adding a test bead.

## Command Evidence

| Command | Exit | Result |
| --- | ---: | --- |
| `rtk df -h /tmp .` | 0 | `/tmp` had 30G available; workspace filesystem had 850G available. |
| `bd where` | 0 | Beads directory resolved to `.beads`; command emitted no auto-backup warning. |
| `bd context` | 0 | Backend type `dolt`, mode `server`, database `velvet_ballistics`, server `127.0.0.1:36155`. |
| `bd show vb-8zyi` | 0 | Target bead loaded; command emitted no auto-backup warning. |
| `bd ready` | 0 | Ready queue loaded; command emitted no auto-backup warning. |
| `rtk du -sh .beads/dolt .beads/backup .beads/embeddeddolt .beads/dolt-server.log 2>/dev/null || true` | 0 | `.beads/dolt` 271M, `.beads/backup` 404M, `.beads/dolt-server.log` 648K; no embedded trap size reported. |
| `bash -lc 'shopt -s nullglob; files=(/tmp/buffered_file_byte_sink_*); if (( ${#files[@]} == 0 )); then printf "%s\n" "no buffered_file_byte_sink files"; else du -sh "${files[@]}"; fi'` | 0 | No `/tmp/buffered_file_byte_sink_*` files remained. |
| `bd dolt pull` | 0 | `Pull complete`; command emitted no auto-backup warning. |
| `bd dolt push` | 0 | `Push complete`; command emitted no auto-backup warning. |
| `bd create "vb-8zyi dry-run create acceptance probe" --description "Dry-run only: bd create acceptance probe for vb-8zyi." --type task --priority P4 --labels beads,dolt,infrastructure --dry-run` | 0 | Dry-run create succeeded; no test bead created; command emitted no auto-backup warning. |
| `bash scripts/check-beads-server-mode.sh` | 0 | `beads server-mode check passed`. |
| `bd doctor` | 0 | 68 passed, 4 warnings, 0 errors; warnings were CLI version, uncommitted tree, detached HEAD, and pre-existing test pollution, not auto-backup. |

Final acceptance harness results:

```text
COMMAND: bd ready
EXIT: 0
EMITTED_WARNING_MATCH: no
STDERR_BYTES: 0
STDOUT_BYTES: 970
---
COMMAND: bd show vb-8zyi
EXIT: 0
EMITTED_WARNING_MATCH: no
STDERR_BYTES: 0
STDOUT_BYTES: 738
---
COMMAND: bd create --dry-run
EXIT: 0
EMITTED_WARNING_MATCH: no
STDERR_BYTES: 0
STDOUT_BYTES: 259
---
COMMAND: bd dolt pull
EXIT: 0
EMITTED_WARNING_MATCH: no
STDERR_BYTES: 0
STDOUT_BYTES: 43
---
COMMAND: bd dolt push
EXIT: 0
EMITTED_WARNING_MATCH: no
STDERR_BYTES: 0
STDOUT_BYTES: 41
---
```

## Git Runtime State

`rtk git status --short --ignored .beads/dolt .beads/backup .beads/embeddeddolt .beads/dolt-server.log .evidence/vb-8zyi` reported:

```text
!! .beads/backup/
!! .beads/dolt-server.log
!! .beads/dolt/
```

This confirms the Dolt runtime database, local backup, and server log are ignored runtime state, not staged Git content.

## Residual risks

The workspace has unrelated uncommitted/staged changes and is in detached HEAD state. Those were left untouched.

`bd doctor` reports one potential pre-existing test-pollution warning. It is unrelated to the auto-backup `/tmp` quota warning and was not changed.

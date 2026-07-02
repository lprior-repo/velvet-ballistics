# Runbook: Repair `replacement_seq` Schema Error (vb-t0iw9)

**Bead**: vb-t0iw9 (P1, BUG)
**Target user**: Lewis (repo owner)
**Config-only scope**: NO binary, scripts/, metadata.json, or embeddeddolt changes.

## Summary

Femdation first-wave dispatch fails with:

```
no such column: replacement_seq
```

This is a **forward schema-skew guard** in the installed `bd 1.0.5 (dev)` binary.
The v49 Dolt schema in `.beads/dolt/` does not contain the `replacement_seq` column
that the binary's guard checks for. The binary's own `bd migrate --inspect` reports
`Registered Migrations: 0`, so the binary has no migration to advance the schema
to add the column. Three verified facts (see `evidence/repro.txt`):

| check                                       | result                                         |
|---------------------------------------------|------------------------------------------------|
| `bd info`                                   | bd v1.0.5 (dev)                                 |
| `bd migrate --inspect`                      | `Schema Version: 1.0.5`, `Registered Migrations: 0` |
| `bd sql "DESCRIBE issues"`                  | 54 columns, no `replacement_seq`                |
| `bd sql "SELECT replacement_seq FROM issues LIMIT 1"` | `Error 1105: column "replacement_seq" could not be found in any table in scope` |
| `bd --ignore-schema-skew supersede <a> --with <b>` | works (proves only the guard, not the column, blocks operations) |

## Two Validated User Actions

The user MUST pick exactly one. Both unblock femdation; they differ in risk profile.

### Action A — One-time SQL ALTER TABLE (preferred, schema-only, no binary change)

Apply a typed addition to the `issues` and `wisps` tables so the column the
bd guard expects actually exists. This is a Dolt working-set change and is
committed via `bd dolt commit` afterward.

```bash
# From /home/lewis/src/velvet-ballistics (the source checkout; do NOT run
# from an isolated worktree, since the Dolt server is the shared
# velvet-ballistics database at /home/lewis/src/velvet-ballistics/.beads/dolt)

# 1. Confirm the column is missing.
bd sql -q "DESCRIBE issues;" | grep -E "replacement_seq|Field"
# Expected: no row containing "replacement_seq".

# 2. Apply the ALTER TABLE. The column type mirrors what bd v1.0.5 (dev)
#    expects (BIGINT, NULL, default 0). Mirroring the existing
#    `depends_on_id` STORED generated-column pattern from migration 0041
#    is NOT required; replacement_seq is a plain nullable column.
bd sql -q "ALTER TABLE issues ADD COLUMN replacement_seq BIGINT DEFAULT NULL;"
bd sql -q "ALTER TABLE wisps  ADD COLUMN replacement_seq BIGINT DEFAULT NULL;"

# 3. Verify the column now exists.
bd sql -q "SHOW COLUMNS FROM issues LIKE 'replacement_seq';"
bd sql -q "SHOW COLUMNS FROM wisps  LIKE 'replacement_seq';"
# Expected: 1 row each, Type=BIGINT, Null=YES, Default=NULL.

# 4. Verify the original error is gone.
bd sql -q "SELECT replacement_seq FROM issues LIMIT 1;"
# Expected: 1 row with NULL value, no error.

# 5. Commit the schema change to Dolt so it persists across server restarts
#    and is visible to all workspaces that share the velvet-ballistics DB.
bd dolt commit -m "schema: add replacement_seq column for bd v1.0.5 forward-skew guard"

# 6. Verify the femdation probe command now succeeds.
bd --ignore-schema-skew sql -q "SELECT COUNT(*) FROM issues;" # sanity
bd sql -q "SELECT COUNT(*) FROM issues;"                       # raw path no longer errors
```

**Why this is safe**:

- It is a purely additive schema change (nullable column, no default behaviour
  change for existing rows).
- It does not modify the bd binary, scripts/, `metadata.json`, or
  `embeddeddolt/`.
- It does not change `dolt_mode` (remains `server`).
- It does not require `bd dolt push` to the remote; the local Dolt commit
  is sufficient for the local server.

**Risk**:
- A future `bd migrate` to a newer binary version may try to re-add the
  column. Verify with `bd migrate --inspect` after any bd upgrade; the
  binary should detect the column as already present and skip the migration.

### Action B — Upgrade bd to a version that ships migration 50+ (long-term, binary change)

`bd info --whats-new` confirms the forward schema-skew guard was added in
v1.0.5. A newer release (>= 1.0.6 or a "dev" build that includes the
`replacement_seq` migration) is expected to bundle a migration that adds
the column. Steps:

```bash
# 1. Snapshot the current bd binary's version + commit for rollback.
bd version
which bd
sha256sum "$(which bd)"

# 2. Stop the shared Dolt server so it releases any open handles.
bd dolt stop

# 3. Install a newer bd. The exact release channel is TBD; check
#    https://github.com/steveyegge/beads/releases for a build that mentions
#    "replacement_seq" in the changelog or migration notes. The
#    go-github-com-steveyegge-beads-cmd-bd mise install is the current
#    delivery channel; bump it via:
#       mise use bd@<new-version>
#    (or `mise install bd@<new-version> && mise reshim`)

# 4. Restart the server and apply the new migrations.
bd dolt start
bd migrate --inspect       # expect 1+ registered migrations now
bd migrate                 # apply them
bd sql -q "SHOW COLUMNS FROM issues LIKE 'replacement_seq';"   # verify

# 5. Re-run the femdation probe.
bd sql -q "SELECT replacement_seq FROM issues LIMIT 1;"
```

**Why this is the long-term fix**: Action A is a local workaround; the
upstream bd project is expected to add the column via its migration system.
Once upstream lands it, Action A's ALTER TABLE will be a no-op (the column
already exists) and `bd migrate` will skip.

**Risk**:
- A bd upgrade can change the on-disk wire format of `dolt_*.json` files
  in ways that block rollback. Keep the sha256 from step 1 and the current
  `bd version` output.
- The Dolt server's persisted working set may need to be re-cloned from
  the remote if the upgrade touches the on-disk format.

## What the user MUST NOT do (per bead MUST NOT list)

- Remove `.beads/embeddeddolt/` (it is a trap directory; if it appears,
  the bead description says remove it, but this runbook assumes it is
  absent — verify with `ls -la .beads/embeddeddolt 2>&1`).
- Change `dolt_mode` in `.beads/metadata.json` (must remain `server`).
- Widen `.beads/metadata.json` beyond minimal (no new keys).
- Modify anything under `scripts/`.
- Modify the `bd 1.0.5` binary.

## Related Drift Discovered (NOT in this bead's scope)

While investigating, the following config drift was observed and recorded
in `evidence/repro.txt` for a follow-up bead:

- `.beads/config.yaml` line 56: `dolt.server-port: 43643` (stale).
- Actual shared Dolt server is listening on port 45645 (per `bd dolt status`
  and `lsof -i :45645`).
- `.beads/dolt/config.yaml` (inner) has `port: 43627` (also stale).

bd finds the server via `metadata.json` (`dolt_server_host: 127.0.0.1`,
no port pin), so this mismatch is currently harmless. A follow-up
bead should align `config.yaml` with the actual port (e.g. 45645) and
delete the inner `.beads/dolt/config.yaml` if it is no longer used by the
beads runtime.

## Verification Commands (run after Action A or B)

```bash
bash scripts/check-beads-server-mode.sh          # exit 0; dolt_mode=server
ls -la .beads/embeddeddolt 2>&1                  # No such file or directory
test ! -e .beads/embeddeddolt                    # exit 0
bd update vb-t0iw9 --status in_progress --claim  # exit 0; bead claims successfully
bd --ignore-schema-skew sql -q "SELECT replacement_seq FROM issues LIMIT 1;"  # exit 0; no error
```

## Reference Evidence

- `evidence/repro.txt` — raw reproduction commands and outputs.
- `evidence/schema-before.txt` — `bd sql "DESCRIBE issues"` before ALTER.
- `evidence/supersede-flag.txt` — proof that `--ignore-schema-skew` bypasses
  the guard at the bd layer.
- `evidence/bd-version.txt` — exact `bd version` and `bd info --whats-new` excerpt.
- `implementation.md` — chosen repair, diff summary, and gate evidence.

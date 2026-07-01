# Contract: vb-t0iw9 — femdation `replacement_seq` schema-error repair

## Acceptance contract

The `replacement_seq` schema-error class surfaced by femdation first-wave dispatch on ready P0 beads MUST be classified, repaired, and verified using only metadata, configuration, dispatch-sandbox pin, or operator-recipe edits — never via production code, runtime IR, or workflow changes. Every repair decision MUST cite captured evidence, every repair MUST keep `dolt_mode=server`, and every repair MUST pass `bash scripts/check-beads-server-mode.sh` and a fresh `bd supersede … --with …` smoke.

## Downstream implementation obligations

1. **OB-001 Dispatch-sandbox capture**: Capture the `bd` binary path, version, env, and `.beads/` discovery root actually invoked by the failing femdation child. Persist under `.beads/vb-t0iw9/sandbox-snapshot/` with sha256 hashes.
2. **OB-002 Schema introspection**: Capture `bd dolt status`, `bd info`, `bd info --whats-new`, `bd migrate`, `bd sql "SHOW FULL TABLES"`, `bd sql "SHOW COLUMNS FROM …"`, `bd sql "SELECT … FROM schema_migrations"`, `bd sql "SELECT … FROM ignored_schema_migrations"`, and `bd sql "SHOW CREATE TABLE dependencies"` into `.beads/vb-t0iw9/schema-introspection/`.
3. **OB-003 Reproduction**: Re-invoke the failing bd subcommand with the captured binary; capture exit_code and stderr verbatim into `.beads/vb-t0iw9/reproduction/`. Parse the error into a closed `SchemaErrorClass` or force `Unclassified → Escalate`.
4. **OB-004 Repair decision**: Select exactly one `RepairDecision` from the repair-decision table in `type-contracts.md`. No decision is legal without a `Reproduced` trace.
5. **OB-005 Server-mode preservation**: `.beads/metadata.json` MUST keep `dolt_mode=server`, MUST NOT gain a `dolt_server_port` key, and `.beads/embeddeddolt/` MUST NOT be created.
6. **OB-006 STORED-column respect**: Any `AddSchemaMigration` decision MUST NOT `ALTER TABLE … DROP COLUMN depends_on_id` or re-add it as a plain column; `SHOW CREATE TABLE dependencies` must show STORED-generated or the decision must escalate.
7. **OB-007 Config precedence**: Repairs to `dolt.server-port`/`dolt.server-host` go into `.beads/config.yaml`; repairs to `dolt_database`/`project_id` go into `.beads/metadata.json`. Cross-mixing is forbidden.
8. **OB-008 Git-cleanliness**: `.beads/dolt/`, `.beads/backup/`, `.beads/dolt-server.port`, and any runtime lockfile MUST NOT be added to git; verify step MUST pass `git status --porcelain` for those paths.
9. **OB-009 Post-repair verification**: Re-run `bd dolt status`, `bd dolt test`, `bd info`, `bd migrate`, `bd sql "SHOW COLUMNS FROM dependencies"`, `bash scripts/check-beads-server-mode.sh`, and `bd supersede vb-qryp7 --with vb-t0iw9`; persist raw outputs into `.beads/vb-t0iw9/post-repair-verification/`.
10. **OB-010 Failure routing**: If any verification command fails, the workflow routes to `Escalate` with at least three `EvidenceRef`s; no "best-effort" recovery is permitted.

## Non-goals

- No production Rust, tests, verifier harnesses, TLA specs, workflow IR, or IPC envelope edits.
- No addition of `dolt_server_port` to `.beads/metadata.json`.
- No creation of `.beads/embeddeddolt/`.
- No `git add` of `.beads/dolt/`, `.beads/backup/`, or runtime lockfiles.
- No claim that any repair closes the femdation dispatch error until `VerifyPostRepair` succeeds.

## Open domain decisions

1. Whether the failing bd binary in the dispatch sandbox is `bd v1.0.5` (host-resolved) or an older pinned binary. Decision deferred to `OB-001` capture.
2. Whether the literal `replacement_seq` maps to `dependencies.depends_on_id` (now STORED-generated) or to a separate replacement-tracking column that the dispatcher queries. Decision deferred to `OB-002` introspection.
3. Whether the legal repair is `EditBeadsConfig { key: DoltServerPort, action: Unset }`, `PinDispatchBinary`, `DocumentExpectedUserAction`, or `Escalate`. Decision deferred to `OB-003` reproduction.
4. Whether to extend `scripts/check-beads-server-mode.sh` with a port-pin assertion. Decision deferred to implementation review.
5. Whether to author a reusable `bd-dispatch-sandbox-probe.md` runbook under `.beads/vb-t0iw9/` for future fleet operators. Decision deferred to State 11 implementer.
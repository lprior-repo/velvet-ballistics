# Domain Model: vb-t0iw9 — femdation `replacement_seq` schema-error repair

## Scope

This contract models the metadata, configuration, and dispatch-sandbox repair surface for the `bd` schema-error class surfaced by femdation's first-wave dispatch on ready P0 beads. It does **not** model or authorize production application code, runtime core, IPC, or workflow IR edits; the present bead is config/policy-only.

The contract names the domain entities, value objects, repair decisions, and forbidden states that the State 4+ proof plan, State 11 implementation, and State 12 verification stages must respect. It is intended to make the *legal* repair surface for the `replacement_seq` schema-error class explicit so the implementer cannot silently introduce one of the *illegal* repairs (e.g. flipping `dolt_mode` to `embedded`, hardcoding a port in `metadata.json`, writing `.beads/dolt` into git, or claiming schema parity without `bd info`/`bd sql` evidence).

## Ubiquitous language

| Term | Meaning | Forbidden interpretation |
|---|---|---|
| `DispatchSandbox` | The runtime context in which femdation executes `bd` for first-wave child dispatch: `PATH`, `BEADS_DOLT_*`, resolved `bd` binary path/version, `.beads/` discovery root, working directory. | "It's whatever `bd` finds in the local checkout." (rejected; the sandbox may pin a different binary or `.beads/` than the source checkout reports.) |
| `BeadsConfig` | The team-level `.beads/config.yaml` read by `bd` for defaults and server connection. May pin `dolt.server-port`, `dolt.server-host`, `actor`, `json`, `no-db`, `backup`. | "Config is irrelevant; only `metadata.json` matters." (rejected; per AGENTS.md the precedence order is `metadata.json` → `BEADS_DOLT_*` → `config.yaml`, so all three are authoritative inputs.) |
| `BeadsMetadata` | The project-level `.beads/metadata.json` declaring `database`, `backend`, `dolt_mode`, `dolt_server_host`, `dolt_database`, `project_id`. Must keep `dolt_mode=server` and must not pin `dolt_server_port`. | "Any JSON shape works." (rejected; `check-beads-server-mode.sh` hard-fails on `embedded` mode or any `dolt_server_port` key.) |
| `BeadsSchemaState` | Observed Dolt server-side schema facts: `schema_migrations` rows, `ignored_schema_migrations` rows, table list, column lists for `issues`/`dependencies`/replacement-tracking tables. | "If `bd migrate` returns 0 it's fine." (rejected; `bd migrate` only reports `Registered Migrations: 0` and the `Schema Version: 1.0.5` headline, not whether any column the dispatch flow queries is missing.) |
| `BdBinaryVersion` | The resolved `bd` binary path plus version string (`bd version 1.0.5 (dev)`) inside the dispatch sandbox, not the version inferred from the host `PATH`. | "Use whichever `bd` is first on `PATH`." (rejected; mise-installed and per-sandbox shadow `bd` binaries have produced version skew before.) |
| `SchemaErrorClass` | Closed taxonomy of bd schema-related errors: `NoSuchColumn`, `NoSuchTable`, `NoSuchMigration`, `StalePortPin`, `UnsupportedMode`, `IgnoredMigrationConflict`, `GenerationColumnDrift`. | "The error is `no such column: replacement_seq`." (rejected; that string may be a literal column miss or a placeholder for a renamed/replaced column the dispatch path now expects.) |
| `RepairDecision` | Sum type: `AddSchemaMigration(version, statement)`, `EditBeadsConfig(path, key, action)`, `EditBeadsMetadata(path, key, action)`, `PinDispatchBinary(path, version)`, `DocumentExpectedUserAction(recipe)`, `Escalate(reason, evidence_refs)`. | "Just patch `config.yaml` and move on." (rejected; the legal repair is selected by the captured evidence, not by editor preference.) |
| `ColumnReference` | A column that dispatch queries, with name, owning table, generation rule (`VIRTUAL`/`STORED`/`plain`), and the `bd info --whats-new` migration that introduced it. | "Any string is a column." (rejected; columns can be STORED-generated in 1.0.5 and not legally re-added as plain columns.) |
| `ConfigKey` | A closed set of `BeadsConfig` keys the present bead may touch: `dolt.server-host`, `dolt.server-port`, `actor`, `json`, `no-db`, `backup.enabled`. | "Edit any YAML key." (rejected; unknown keys are an illegal repair state.) |
| `MetadataKey` | A closed set of `BeadsMetadata` keys the present bead may touch: `dolt_database`, `project_id`. | "Edit any JSON key." (rejected; `database`, `backend`, `dolt_mode`, `dolt_server_host` are policy-locked by AGENTS.md and `check-beads-server-mode.sh`.) |

## Entities and value objects

- Aggregate: `BeadsDispatchSurface` owns the joint state of `BeadsMetadata`, `BeadsConfig`, `BeadsSchemaState`, `BdBinaryVersion`, and `DispatchSandbox` for one `.beads/` root.
- Aggregate: `RepairPlan` owns one `SchemaErrorClass`, its captured `ReproductionEvidence`, the selected `RepairDecision`, and the `PostRepairVerification` commands.
- Value objects: `BdVersion(String)`, `ColumnName(String)`, `TableName(String)`, `MigrationVersion(u32)`, `Port(u16)`, `Host(String)`, `ProjectId(Uuid)`, `DatabaseName(String)`, `ConfigKey(ConfigKey)`, `MetadataKey(MetadataKey)`, `ErrorClass(SchemaErrorClass)`, `ReproductionTrace { command, exit_code, captured_at, raw_log_path }`.

## Domain invariants

1. **INV-001 Server-mode immutability**: `BeadsMetadata.dolt_mode` MUST remain `"server"` and `.beads/embeddeddolt/` MUST NOT be created. Any repair that violates this is forbidden by `scripts/check-beads-server-mode.sh` and AGENTS.md Beads Dolt Remote clauses.
2. **INV-002 No port pin in metadata**: `BeadsMetadata` MUST NOT gain a `dolt_server_port` key; bd auto-discovers via `.beads/dolt-server.port`.
3. **INV-003 Git-cleanliness of server data**: `.beads/dolt/`, `.beads/backup/`, and runtime lockfiles MUST NOT be added to source control. Edits target only `.beads/metadata.json`, `.beads/config.yaml`, and any newly authored `.beads/vb-t0iw9/*.md` evidence files.
4. **INV-004 Schema-error class explicit**: Any repair MUST identify one `SchemaErrorClass` and cite its `ReproductionEvidence`; an `Escalate` is the only legal repair when no class matches the captured evidence.
5. **INV-005 STORED-generation respect**: `dependencies.depends_on_id` is a STORED generated column as of `bd v1.0.5` per `bd info --whats-new` migrations 0041-0042. Any repair MUST NOT attempt to `ALTER TABLE ... DROP COLUMN depends_on_id` or re-add it as a plain column; the column's absence from a downstream view is not the same as the column's absence from the base table.
6. **INV-006 Config precedence honored**: Repairs to `dolt.server-host`/`dolt.server-port` go into `BeadsConfig`, never `BeadsMetadata`. Repairs to `dolt_database`/`project_id` go into `BeadsMetadata`. Cross-mixing precedence is illegal.
7. **INV-007 Binary-version determinism**: A captured `BdBinaryVersion` is binding for the dispatch sandbox it was captured in; subsequent captures must agree or the dispatch sandbox itself must be repaired.
8. **INV-008 Ignored-migration visibility**: `BeadsSchemaState` MUST report `ignored_schema_migrations` rows by version and applied_at; ignoring them silently is forbidden because ignored migrations can mask the schema-drift that surfaces as `no such column: …`.
9. **INV-009 Evidence-before-repair**: No `RepairDecision` is legal without a `ReproductionEvidence` row whose `bd dolt status`, `bd migrate`, `bd info --whats-new`, and `bd sql "SHOW COLUMNS FROM …"` outputs are saved under `.beads/vb-t0iw9/`.
10. **INV-010 Fail-closed on missing evidence**: If the captured binary cannot be probed, the schema cannot be introspected, or `bd dolt status`/`bd sql` returns nonzero, the repair is `Escalate(reason, evidence_refs)`; no speculative repair is allowed.
11. **INV-011 Repair scope discipline**: The bead touches only `.beads/metadata.json`, `.beads/config.yaml`, `.beads/vb-t0iw9/*.md` evidence, and `scripts/check-beads-server-mode.sh` if a CI gate is warranted. Production `crates/**`, `verification/**`, `tests/**`, `fuzz/**`, `xtask/**` are out of scope and must not be modified.

## Commands and events

- Commands: `ProbeDispatchSandbox`, `CaptureBdVersion`, `IntrospectSchemaState`, `ReproduceSchemaError`, `ClassifySchemaError`, `ApplyRepairDecision`, `VerifyPostRepair`.
- Events: `DispatchSandboxCaptured`, `SchemaStateIntrospected`, `SchemaErrorReproduced`, `SchemaErrorClassified`, `RepairDecisionApplied`, `PostRepairVerified`.

## Policies

- `ProbeDispatchSandbox` MUST run inside the same shell environment as the failing femdation child invocation; it MUST NOT trust the host-shell `PATH` alone.
- `ApplyRepairDecision` MUST be preceded by `ReproduceSchemaError` whose exit_code is the actual bd failure mode (currently observed as a non-pickled error string in femdation logs).
- `ApplyRepairDecision` MUST keep `dolt_mode=server` and MUST NOT introduce `.beads/embeddeddolt/`, a `dolt_server_port` key in `metadata.json`, or any `.beads/dolt/` git add.
- `VerifyPostRepair` MUST rerun `bd dolt status`, `bd dolt test`, `bd info`, `bd migrate`, `bd sql 'SHOW COLUMNS FROM dependencies'`, `bash scripts/check-beads-server-mode.sh`, and the dispatched `bd supersede … --with …` smoke (already green today per codebase-map §38-41).

## Open domain decisions

1. Whether the literal `replacement_seq` is a real missing column on a shadow or older schema, or whether the femdation log captured a placeholder/error-shape string that maps to `dependencies.depends_on_id` (now STORED-generated in 1.0.5). Decision deferred to the State 4 evidence capture.
2. Whether the femdation dispatch sandbox pins a `bd` binary older than `1.0.5` via mise shim, shell alias, or per-bead path export, or whether the dispatch sandbox inherits the host `bd 1.0.5` cleanly. Decision deferred to `CaptureBdVersion` evidence.
3. Whether the legal repair is `EditBeadsConfig` to drop the stale `dolt.server-port: 43643` pin and let bd auto-discover `45645`, or `PinDispatchBinary` to lock `bd` to `1.0.5`, or `DocumentExpectedUserAction` if the cause is purely environmental. Decision deferred to `ClassifySchemaError`.
4. Whether to add a `SchemaRepairPlan.md` evidence artifact that documents the `BdBinaryVersion` capture protocol so future fleet operators do not re-hit this class.
5. Whether to extend `scripts/check-beads-server-mode.sh` with a port-pin assertion (`grep -v 'dolt.server-port: 43643' .beads/config.yaml` or similar) or to leave the port-pin concern out of the CI gate and treat it as runbook policy.
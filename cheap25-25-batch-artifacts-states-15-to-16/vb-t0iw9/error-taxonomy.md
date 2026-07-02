# Error Taxonomy: vb-t0iw9

Typed errors must be raised before any `RepairDecision` is selected. The taxonomy is closed; any error not on the list is reported as `UnclassifiedError` and forces the workflow into `Escalate`.

## Probe and capture errors

| Variant | Meaning | Mutation |
|---|---|---|
| `BdBinaryUnresolvable` | `which bd`/`command -v bd`/`type bd` returns nothing, or the resolved path is not a regular file. | No |
| `InvalidBdVersionOutput` | `bd version` returns empty, multi-line, or non-bd output. | No |
| `SandboxEnvCaptureFailed` | `env | grep -E '^(BD_|BEADS_|PATH=)'` fails or returns empty. | No |
| `BeadsRootDiscoveryFailed` | `bd where` returns no `.beads/` discovery root. | No |
| `BeadsConfigParseError` | `.beads/config.yaml` is not valid YAML or contains unknown keys. | No |
| `BeadsMetadataParseError` | `.beads/metadata.json` is not valid JSON, has unknown keys, or `dolt_mode != "server"`. | No |
| `MetadataServerModeViolation` | `dolt_mode` is `embedded` or missing, or `.beads/embeddeddolt/` exists. | No |
| `MetadataPortPinViolation` | `.beads/metadata.json` contains a `dolt_server_port` key. | No |
| `ServerModeCheckFailed` | `bash scripts/check-beads-server-mode.sh` exits nonzero. | No |

## Schema introspection errors

| Variant | Meaning | Mutation |
|---|---|---|
| `BdDoltStatusNonzero` | `bd dolt status` returns nonzero exit or unreadable output. | No |
| `BdInfoNonzero` | `bd info` returns nonzero exit. | No |
| `BdMigrateNonzero` | `bd migrate` returns nonzero exit or reports a missing column. | No |
| `BdSqlParseError` | `bd sql` returns nonzero exit or empty result for any of the required introspections. | No |
| `SchemaIntrospectionIncomplete` | One or more required introspections (`SHOW FULL TABLES`, `SHOW COLUMNS FROM …`, `SELECT … FROM schema_migrations`, `SELECT … FROM ignored_schema_migrations`, `SHOW CREATE TABLE dependencies`) are missing. | No |
| `DependenciesColumnMissing` | `bd sql "SHOW COLUMNS FROM dependencies"` does not include `depends_on_id`. (Hypothetical until evidence proves the column is genuinely absent.) | No |
| `DependenciesColumnNotStored` | `SHOW CREATE TABLE dependencies` shows `depends_on_id` as plain or virtual instead of STORED generated. | No |
| `IgnoredMigrationUnreadable` | `bd sql "SELECT version, applied_at FROM ignored_schema_migrations"` returns nonzero or empty rows when migrations exist. | No |

## Reproduction errors

| Variant | Meaning | Mutation |
|---|---|---|
| `ReproductionTraceMissing` | No `ReproductionTrace` captured before repair decision selection. | No |
| `RawErrorParseFailed` | `SchemaErrorClass::parse` cannot map the raw error string to a closed class. | No |
| `RawErrorEmpty` | The captured error string is empty. | No |
| `ProbeFailedDuringReproduction` | The reproduction command itself failed for non-schema reasons (network, auth). | No |
| `NotReproducible` | Reproduction ran but did not reproduce the failing query; recorded as `NotReproduced` and routed to `Escalate`. | No |

## Repair decision errors

| Variant | Meaning | Mutation |
|---|---|---|
| `RepairDecisionBlocked` | Selected decision requires evidence not yet captured. | No |
| `EditConfigKeyUnknown` | `RepairDecision::EditBeadsConfig { key }` references a `ConfigKey` outside the closed enum. | No |
| `EditMetadataKeyUnknown` | `RepairDecision::EditBeadsMetadata { key }` references a `MetadataKey` outside the closed enum. | No |
| `AddSchemaMigrationStatementInvalid` | `AddSchemaMigration { statement }` re-declares a STORED column as plain, or alters a policy-locked table. | No |
| `PinBinaryVersionMismatch` | `PinDispatchBinary` references a path that does not match the captured `BdBinaryPath`. | No |
| `RepairRecipeEmpty` | `DocumentExpectedUserAction { recipe }` provides an empty recipe. | No |

## Post-repair verification errors

| Variant | Meaning | Mutation |
|---|---|---|
| `VerifyCommandNonzero` | Any verification command exits nonzero. | No |
| `VerifyServerModeRegression` | `bash scripts/check-beads-server-mode.sh` exits nonzero after repair. | No |
| `VerifyStoredColumnRegression` | `depends_on_id` is no longer STORED after repair. | No |
| `VerifySupersedeFailed` | `bd supersede vb-qryp7 --with vb-t0iw9` exits nonzero. | No |
| `VerifyMetadataRegression` | `.beads/metadata.json` gained or lost a `dolt_server_port` key. | No |
| `VerifyDoltRegression` | `.beads/dolt/` gained tracked files in git. | No |

## Existing error mapping

- `bd` errors shaped `no such column: <ident>` parse to `NoSuchColumn`.
- `bd` errors shaped `no such table: <ident>` parse to `NoSuchTable`.
- `bd` errors shaped `migration <digits> not registered` parse to `NoSuchMigration`.
- `bd` errors shaped `connection refused: <host>:<port>` parse to `StalePortPin` when the configured `BeadsConfig.dolt.server-port` differs.
- `bd` errors that mention `embedded` or `embeddeddolt` parse to `UnsupportedMode`.
- `bd` errors that mention `ignored migration` parse to `IgnoredMigrationConflict`.

## Railway rules

- Every transition out of `SchemaKnown` MUST succeed before any `RepairDecision` is constructed.
- Every `RepairDecision` MUST be paired with a `ReproductionTrace` before `ApplyRepairDecision`.
- Every `ApplyRepairDecision` MUST be followed by `VerifyPostRepair`; the workflow cannot reach `Documented` without a passing verification.
- Every failure path routes through `Escalate`; there is no implicit "best-effort" recovery.
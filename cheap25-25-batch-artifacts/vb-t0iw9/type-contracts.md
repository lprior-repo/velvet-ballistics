# Type Contracts: vb-t0iw9

## Primitive-obsession rejection

No bare `String`, `u32`, `bool`, free-form YAML/JSON keys, or unchecked casts may represent dispatch-sandbox state, schema error class, or repair decisions. External parser input crosses a smart constructor before it can become a value object in this contract, so the rust-contract core cannot accept a malformed repair or unclassified error.

The present bead does NOT add Rust production types — it defines the *contract* types that any downstream wrapper, repair tool, or verification script must obey. Those tools will live outside `crates/**`.

## Value objects

| Type | Constructor contract | Error seed |
|---|---|---|
| `BdVersion` | `parse(raw)` requires `bd version <semver>(<channel>)` shape; rejects empty, multi-line, or non-bd output. | `InvalidBdVersionOutput` |
| `BdBinaryPath` | `resolve(path)` requires non-empty, absolute, and the file must be a regular file that `bd version` accepts. | `BdBinaryUnresolvable` |
| `Port(u16)` | `Port::new(n)` requires `1 ≤ n ≤ 65535`; rejects well-known ports below 1024 unless explicitly typed `PrivilegedPort`. | `PortOutOfRange` |
| `Host(String)` | `Host::new(h)` requires non-empty, parses as IPv4/IPv6 literal or `localhost`/`127.0.0.1`; rejects `0.0.0.0` for client-facing dispatch. | `InvalidHost` |
| `DatabaseName(String)` | `DatabaseName::new(n)` requires non-empty, kebab/snake ASCII, matches `[a-z0-9_-]{1,63}`; rejects spaces and unicode. | `InvalidDatabaseName` |
| `ProjectId(Uuid)` | `ProjectId::new(u)` requires RFC-4122 UUID; rejects nil UUID. | `NilProjectIdRejected` |
| `MigrationVersion(u32)` | `MigrationVersion::new(v)` requires `v ≥ 1`; rejects 0 and `> u32::MAX`. | `InvalidMigrationVersion` |
| `ColumnName(String)` | `ColumnName::new(n)` requires non-empty, snake_case ASCII, length ≤ 64. | `InvalidColumnName` |
| `TableName(String)` | `TableName::new(n)` requires non-empty, snake_case ASCII, length ≤ 64. | `InvalidTableName` |
| `ConfigKey` | Closed enum: `DoltServerHost`, `DoltServerPort`, `Actor`, `Json`, `NoDb`, `BackupEnabled`. No stringly keys. | `UnknownConfigKey` |
| `MetadataKey` | Closed enum: `DoltDatabase`, `ProjectId`. (Other keys are policy-locked by AGENTS.md.) | `UnknownMetadataKey` |
| `MetadataAction` | Closed enum: `Set(value)`, `Unset`. | `InvalidMetadataAction` |
| `ConfigAction` | Closed enum: `Set(value)`, `Unset`. | `InvalidConfigAction` |
| `ReproductionTrace` | All of `command`, `exit_code`, `captured_at`, `raw_log_path` required. `raw_log_path` must exist at construction. | `MissingReproductionLog` |
| `EvidenceRef` | `EvidenceRef::new(path)` requires the path to exist under `.beads/vb-t0iw9/` and be a regular file. | `EvidenceRefMissing` |
| `RepairRecipe` | `RepairRecipe::new(text)` requires non-empty text and explicit step list. | `EmptyRepairRecipe` |

## Closed enums / sum types

- `SchemaErrorClass` = `NoSuchColumn(ColumnName, TableName)` | `NoSuchTable(TableName)` | `NoSuchMigration(MigrationVersion)` | `StalePortPin { configured: Port, live: Port }` | `UnsupportedMode { mode: String }` | `IgnoredMigrationConflict { versions: Vec<MigrationVersion> }` | `GenerationColumnDrift { column: ColumnName, observed_kind: ColumnKind }` | `Unclassified { raw_error: String }`.
- `ColumnKind` = `Plain` | `Virtual` | `Stored`. (Closed; new kinds require a schema-migration contract add.)
- `RepairDecision` = `AddSchemaMigration { version: MigrationVersion, statement: String }` | `EditBeadsConfig { key: ConfigKey, action: ConfigAction }` | `EditBeadsMetadata { key: MetadataKey, action: MetadataAction }` | `PinDispatchBinary { path: BdBinaryPath, version: BdVersion }` | `DocumentExpectedUserAction { recipe: RepairRecipe }` | `Escalate { reason: String, evidence_refs: Vec<EvidenceRef> }`.
- `ReproductionOutcome` = `Reproduced { trace: ReproductionTrace }` | `NotReproduced { reason: String }` | `ProbeFailed { reason: String }`.
- `PostRepairVerification` = `Verified { captured_at: Timestamp, evidence_refs: Vec<EvidenceRef> }` | `VerificationFailed { evidence_refs: Vec<EvidenceRef>, failure_reason: String }`.
- `SandboxProbeOutcome` = `Captured { version: BdVersion, binary_path: BdBinaryPath, env: EnvSummary }` | `CaptureFailed { reason: String }`.

## Illegal states made unrepresentable

- **Unknown config key**: `ConfigKey` is a closed enum; a YAML key outside the enum cannot construct a `RepairDecision::EditBeadsConfig` argument.
- **Cross-precedence edits**: `BeadsMetadata.dolt_mode`, `database`, `backend`, `dolt_server_host`, and any `dolt_server_port` cannot be selected by `MetadataKey`; the enum simply has no variant for them. A repair that would touch them is forced to be either an `Escalate` or a contract violation rejected by `check-beads-server-mode.sh`.
- **Stored column reinstallation**: `AddSchemaMigration` cannot accept a `statement` whose `ALTER TABLE ... ADD COLUMN` re-declares `dependencies.depends_on_id` as a plain column; the parser must reject statements that mention `depends_on_id` outside a STORED/COMMENTED context.
- **Embedded-mode flip**: `BeadsMetadata` value objects do not expose `dolt_mode`; the only legal value remains the existing string literal, parsed at load time.
- **Unclassified errors silently treated as classified**: `SchemaErrorClass::Unclassified` forces the `RepairDecision` to be `Escalate`; no other decision is legal until the captured raw error string is parsed into a closed class.
- **Evidence-less repair**: `RepairDecision` requires `Vec<EvidenceRef>` either inline (`Escalate`) or implied by the prior `ReproductionTrace`. A bare `ApplyRepairDecision(d)` without a `ReproductionOutcome::Reproduced` is a typestate violation.
- **Stale-port-only repair**: `SchemaErrorClass::StalePortPin` requires both `configured` and `live` ports to be present; the dispatch sandbox must surface both via `bd dolt status` and `BeadsConfig.dolt.server-port` before the class can be constructed.

## Typestate

```text
ExternalYamlKeys   --ConfigKey::parse-->      ConfigKey
ExternalJsonKeys   --MetadataKey::parse-->    MetadataKey
RawErrorString     --SchemaErrorClass::parse-->  SchemaErrorClass
CapturedRawLogs    --ReproductionTrace::new--> ReproductionTrace
SchemaErrorClass + ReproductionTrace --ClassifySchemaError--> RepairDecision
RepairDecision     --ApplyRepairDecision--> AppliedRepair
AppliedRepair      --VerifyPostRepair-->     PostRepairVerification
```

Only `RepairDecision` constructed from a `Reproduced` trace may flow into `AppliedRepair`. Only `AppliedRepair` may flow into `PostRepairVerification`. The implementation must reject any state that skips the typestate, so a "patch `config.yaml` and move on" sequence cannot type-check.

## Boundary parsers

- `BeadsConfig::load(path)` validates YAML into `BTreeMap<String, Value>` and then maps known keys into `ConfigKey` value objects; unknown keys produce `UnknownConfigKey` and the parser must refuse to construct a partial `BeadsConfig` that hides the unknown key.
- `BeadsMetadata::load(path)` validates JSON into a `serde_json::Value` and then projects known keys into `MetadataKey` value objects; `dolt_mode != "server"`, presence of `dolt_server_port`, or presence of `.beads/embeddeddolt/` are construction-time errors.
- `BdVersion::parse(text)` rejects empty lines, multi-line output, and any version string that does not match `<digits>.<digits>.<digits>( <channel>)?`.
- `SchemaErrorClass::parse(raw)` parses the captured error string with a closed grammar: `no such column: <ident>.<ident>` → `NoSuchColumn`; `no such table: <ident>` → `NoSuchTable`; `migration <digits> not registered` → `NoSuchMigration`; `connection refused: <host>:<port>` with the live port differing from the configured port → `StalePortPin`; anything else → `Unclassified { raw_error }`.

## Serialization

- Repair recipes serialize as UTF-8 Markdown files under `.beads/vb-t0iw9/`. They are evidence, not executable; the contract does not authorize parsing recipe text into shell at runtime.
- Reproduction traces serialize as JSON with required fields: `command`, `exit_code`, `captured_at`, `raw_log_path`, `bead_id`, `dispatch_sandbox_hash`.
- `BeadsConfig` and `BeadsMetadata` are persisted via the host `bd` and YAML/JSON parsers; the contract layer only validates them, never re-serializes a normalized form (preserves human-edited shape).

## Repair decision table

| Error class | Required evidence before repair | Default legal decision |
|---|---|---|
| `NoSuchColumn` | `bd sql "SHOW COLUMNS FROM <table>"` log + column-classification grep on `bd info --whats-new` | `Escalate` until column provenance is confirmed STORED/legacy/plain |
| `NoSuchTable` | `bd sql "SHOW FULL TABLES"` log | `Escalate` (table drop is not in this bead's scope) |
| `NoSuchMigration` | `bd migrate` + `bd sql "SELECT version FROM schema_migrations"` logs | `Escalate` (add-migration requires a bd binary that emits it) |
| `StalePortPin` | `BeadsConfig.dolt.server-port` parsed value + `bd dolt status` reported port | `EditBeadsConfig { key: DoltServerPort, action: Unset }` |
| `UnsupportedMode` | `BeadsMetadata.dolt_mode` parsed value + `ls -la .beads/embeddeddolt` log | `Escalate` (mode flip is forbidden by AGENTS.md and `check-beads-server-mode.sh`) |
| `IgnoredMigrationConflict` | `bd sql "SELECT version, applied_at FROM ignored_schema_migrations"` log | `Escalate` (ignored-migration cleanup requires Dolt admin action) |
| `GenerationColumnDrift` | `bd sql "SHOW COLUMNS FROM dependencies"` log + `bd info --whats-new` for migration 0041-0042 | `DocumentExpectedUserAction` if `depends_on_id` is STORED; `Escalate` if not |
| `Unclassified` | Raw error string in `ReproductionTrace` | `Escalate` (no closed-class mapping) |
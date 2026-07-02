# Workflow Model: vb-t0iw9

## Lifecycle

```text
Unknown      --ProbeDispatchSandbox-->   SandboxProbed
SandboxProbed --CaptureBdVersion-->      VersionCaptured
VersionCaptured --IntrospectSchemaState--> SchemaKnown
SchemaKnown    --ReproduceSchemaError-->  Reproduced | NotReproduced | ProbeFailed
Reproduced     --ClassifySchemaError-->   Classified(Class, Trace)
Classified     --SelectRepairDecision-->  PlannedDecision
PlannedDecision --ApplyRepairDecision--> AppliedRepair | RepairFailed
AppliedRepair  --VerifyPostRepair-->     Verified | VerificationFailed
Verified       --DocumentEvidence-->     Documented
RepairFailed | VerificationFailed | ProbeFailed | NotReproduced --Escalate--> Escalated
Escalated      --OperatorDecision-->     OpenOrClosed
```

## Probe and capture workflow

1. Receive the dispatch-sandbox capture request from femdation's first-wave child invocation that surfaced `no such column: replacement_seq`.
2. Resolve the `BdBinaryPath` actually invoked by the failing child: `which bd`, `command -v bd`, `type bd`, and `ls -la` of the resolved path. Capture full env (`env | grep -E '^(BD_|BEADS_|PATH=)' | sort`).
3. Capture `bd version` output as `BdVersion`.
4. Capture the dispatch sandbox's `.beads/` discovery root via `bd where` and `bd config get dolt.server-host`.
5. Snapshot `BeadsConfig` (`.beads/config.yaml`) and `BeadsMetadata` (`.beads/metadata.json`) into `.beads/vb-t0iw9/sandbox-snapshot/{config.yaml,metadata.json}` with sha256 hashes.
6. Persist the captured state as `SandboxProbeOutcome::Captured`.

## Schema introspection workflow

1. Run `bd dolt status` and capture port, host, database, branch, and `Last commit`.
2. Run `bd info` and capture `Schema Version`, `Registered Migrations`, `Issue Count`.
3. Run `bd info --whats-new` and capture the migration log up to and including 0041-0042.
4. Run `bd sql "SHOW FULL TABLES"` and persist the table list.
5. Run `bd sql "SHOW COLUMNS FROM issues"`, `bd sql "SHOW COLUMNS FROM dependencies"`, `bd sql "SHOW COLUMNS FROM ready_issues"`, `bd sql "SHOW COLUMNS FROM schema_migrations"`, `bd sql "SHOW COLUMNS FROM ignored_schema_migrations"` and persist each.
6. Run `bd sql "SELECT version FROM schema_migrations ORDER BY version"` and persist.
7. Run `bd sql "SELECT version, applied_at FROM ignored_schema_migrations ORDER BY version"` and persist.
8. Run `bd sql "SHOW CREATE TABLE dependencies"` to confirm whether `depends_on_id` is STORED, VIRTUAL, or plain.
9. Persist introspection outputs under `.beads/vb-t0iw9/schema-introspection/` and produce `BeadsSchemaState` evidence.

## Reproduction workflow

1. From the captured `SandboxProbeOutcome`, re-invoke the failing bd subcommand path that produced `no such column: replacement_seq`.
2. If the failing bd invocation path is not recorded, attempt the most likely candidates: `bd supersede <id> --with <id>`, `bd duplicate <id>`, `bd ready`, and `bd show <id> --json`.
3. Capture exit_code, stderr, and stdout verbatim into `ReproductionTrace`.
4. Parse the captured error string into `SchemaErrorClass` via `SchemaErrorClass::parse(raw)`.
5. If the parser returns `Unclassified`, persist the raw string and force the workflow to `Escalate`.
6. On `Reproduced`, persist the trace under `.beads/vb-t0iw9/reproduction/{trace.json, raw.log}`.

## Repair decision workflow

1. Given `Classified(Class, Trace)`, select one `RepairDecision` according to the repair-decision table in `type-contracts.md`.
2. The selection MUST honor the table's "default legal decision"; an alternative decision requires explicit `Escalate` first.
3. For `EditBeadsConfig` and `EditBeadsMetadata`, the implementer MUST regenerate the file with only the targeted key changed; no incidental edits.
4. For `PinDispatchBinary`, the implementer MUST persist the binary path/version pair as a `DispatchSandboxPin` artifact under `.beads/vb-t0iw9/` and document the export recipe.
5. For `DocumentExpectedUserAction`, the implementer MUST author `RepairRecipe` Markdown with explicit operator steps.
6. For `Escalate`, the implementer MUST populate `evidence_refs` with at least three `EvidenceRef`s from the introspection outputs.
7. For `AddSchemaMigration`, the implementer MUST stage the `statement` for review by `proof-planner`/`proof-reviewer` before any `bd` apply; this bead currently treats this decision as blocked until the State 4 evidence proves a column is actually missing.

## Post-repair verification workflow

1. Run `bd dolt status`; require nonzero exit if the underlying connection is broken.
2. Run `bd dolt test`; require `OK` aggregate.
3. Run `bd info`; require `Schema Version` and `Registered Migrations` to match the pre-repair snapshot unless the repair was `AddSchemaMigration` or `PinDispatchBinary`.
4. Run `bd migrate`; require exit 0 and no "missing column" wording in stdout.
5. Run `bd sql "SHOW COLUMNS FROM dependencies"`; require `depends_on_id` to remain a STORED generated column unless an explicit migration declared otherwise.
6. Run `bash scripts/check-beads-server-mode.sh`; require `beads server-mode check passed` on stdout.
7. Run `bd supersede vb-qryp7 --with vb-t0iw9`; require `Marked vb-qryp7 as superseded by vb-t0iw9 (closed)`.
8. Persist each command's raw output under `.beads/vb-t0iw9/post-repair-verification/` and produce `PostRepairVerification::Verified`.

## Outcomes

- `SandboxProbed`
- `VersionCaptured`
- `SchemaKnown`
- `Reproduced(Trace)` | `NotReproduced(reason)` | `ProbeFailed(reason)`
- `Classified(Class, Trace)`
- `PlannedDecision(RepairDecision)`
- `AppliedRepair` | `RepairFailed(reason)`
- `Verified(evidence_refs)` | `VerificationFailed(reason, evidence_refs)`
- `Documented(recipe_path)`
- `Escalated(reason, evidence_refs)`
- `OpenOrClosed` (operator terminal)

## Guards

- Server-mode guard: every transition out of `SchemaKnown` MUST be preceded by `bash scripts/check-beads-server-mode.sh` having printed `beads server-mode check passed` for the same `.beads/` root.
- Evidence-before-repair guard: `PlannedDecision` is unreachable without a `Reproduced` or `ProbeFailed` trace.
- Schema-class closure guard: `Unclassified` class flows only to `Escalate`; no other downstream state is reachable.
- Stored-column respect guard: `AppliedRepair` for `dependencies.depends_on_id` MUST NOT include an `ALTER TABLE … DROP COLUMN` or `ALTER TABLE … ADD COLUMN … <plain>` statement.
- Git-cleanliness guard: `.beads/dolt/`, `.beads/backup/`, and runtime lockfiles MUST NOT be touched by any state transition.
- Port-pin authority guard: any change to `dolt.server-port` MUST go through `BeadsConfig`, never `BeadsMetadata`.

## Temporal hazards (covered by loom/proptest lanes later)

- A `bd` binary shadowed by mise shim or shell alias can change between capture and apply; the verification workflow re-probes immediately before `VerifyPostRepair`.
- `.beads/dolt-server.port` is rewritten by bd at server start; reading it after the server is running may capture a stale value. The probe workflow records the capture timestamp to disambiguate.
- An ignored-migration row applied in the past can be un-ignored by a future `bd migrate`; the verify workflow re-reads `ignored_schema_migrations` post-repair.

## Idempotence requirements

- `ApplyRepairDecision(EditBeadsConfig { key, action: Unset })` MUST be idempotent: running twice on a file that already lacks the key yields the same on-disk file (byte-equal after whitespace normalization).
- `ApplyRepairDecision(DocumentExpectedUserAction)` MUST be idempotent: writing the same recipe twice produces byte-equal outputs.
- `VerifyPostRepair` MUST be safe to re-run; it MUST NOT mutate `BeadsSchemaState`, only read it.
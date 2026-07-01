# Boundary Map: vb-t0iw9

## Pure core

- Parse `BeadsConfig` YAML keys into the closed `ConfigKey` enum.
- Parse `BeadsMetadata` JSON keys into the closed `MetadataKey` enum and assert `dolt_mode == "server"`.
- Parse `BdVersion` from `bd version` output.
- Parse `SchemaErrorClass` from raw error strings via the closed grammar.
- Classify captured errors into `RepairDecision` candidates per the repair-decision table.
- Validate that `RepairDecision` arguments are typed-correctly (no unknown keys, no `ALTER TABLE … DROP COLUMN` on `depends_on_id`).
- Compose `PostRepairVerification` evidence references.

The pure core has no I/O. It accepts already-loaded strings and returns typed values or typed errors.

## Imperative shell

- Resolve `BdBinaryPath` via `which bd`/`command -v bd`/`type bd`.
- Capture environment via `env | grep -E '^(BD_|BEADS_|PATH=)' | sort`.
- Run `bd where`, `bd config get …`, `bd dolt status`, `bd info`, `bd info --whats-new`, `bd migrate`, `bd sql …`, `bd dolt test`, `bd supersede …`.
- Read and write `.beads/config.yaml` and `.beads/metadata.json` via small Rust-style helper scripts; this is the only file-write boundary in this contract.
- Persist evidence under `.beads/vb-t0iw9/{sandbox-snapshot, schema-introspection, reproduction, post-repair-verification}/`.

## Tool boundaries

- **`bd` binary**: read-only by default; the only mutation channel is `bd migrate` (gated by `AddSchemaMigration`) and `bd supersede … --with …` (gated by the supersede-smoke verification step). No direct `dolt sql` writes.
- **`bash scripts/check-beads-server-mode.sh`**: invoked at probe time and at verify time; output is parsed for `beads server-mode check passed` and exit code is captured.
- **Filesystem**: `.beads/config.yaml` and `.beads/metadata.json` are the only writable config files. `.beads/dolt/`, `.beads/backup/`, and `.beads/dolt-server.port` are read-only for this contract.
- **Git**: the contract layer MUST NOT `git add` `.beads/dolt/`, `.beads/backup/`, `.beads/dolt-server.port`, `.beads/embeddeddolt/`, or any runtime lockfile. The `git status` clean check at the verify step enforces this.

## Data boundaries

- **Inputs**: the captured dispatch-sandbox state (env, binary path, version, raw logs), the existing `.beads/config.yaml`, the existing `.beads/metadata.json`, the bd introspection outputs.
- **Outputs**: edited `.beads/config.yaml` (only the targeted `ConfigKey`), edited `.beads/metadata.json` (only the targeted `MetadataKey`), and new Markdown/JSONL evidence files under `.beads/vb-t0iw9/`.
- **No runtime state**: this contract does not author any production Rust behavior, workflow IR, IPC envelope, or storage schema migration (other than the explicitly gated `AddSchemaMigration` decision).

## Trust boundaries

- **Dispatch-sandbox binary**: trusted only after `CaptureBdVersion` returns a parseable `BdVersion`. A misparse forces `ProbeFailed` → `Escalate`.
- **`.beads/config.yaml`**: trusted only after `BeadsConfig::load` succeeds and only the targeted `ConfigKey` is touched.
- **`.beads/metadata.json`**: trusted only after `BeadsMetadata::load` succeeds, asserts `dolt_mode == "server"`, and confirms no `dolt_server_port` key.
- **`bd` introspection outputs**: trusted only when the corresponding command exits 0 and the captured output matches the expected shape (e.g. `bd info` must contain `Schema Version: 1.0.5`, `Registered Migrations: 0`, `Issue Count: 2650`).
- **Prior capped evidence**: the codebase-map §38-41 finding that `bd supersede vb-qryp7 --with vb-t0iw9` succeeds is contextual only; it is not proof that the failing dispatch sandbox will succeed.

## Async/concurrency boundary

- The bead is single-threaded per femdation child invocation. The probe → introspection → reproduction → repair → verify sequence MUST be linear; there is no parallel dispatch.
- Multiple femdation children running concurrently against the same Dolt server must serialize through the existing bd server lock; this contract does not introduce new locking.

## FFI/unsafe/parser boundary

- The YAML parser for `.beads/config.yaml` MUST reject unknown keys; this is enforced by `BeadsConfig::load` failing the parse on any key outside the closed `ConfigKey` enum.
- The JSON parser for `.beads/metadata.json` MUST reject any key outside the closed `MetadataKey` enum, plus the policy-locked keys (`dolt_mode`, `database`, `backend`, `dolt_server_host`, `dolt_server_port`).
- The `bd version` parser MUST reject non-bd output.
- The `SchemaErrorClass::parse` parser MUST reject anything outside the closed grammar; unknown shapes force `Unclassified` → `Escalate`.

## Config precedence boundary

Per AGENTS.md Beads Dolt Remote policy, the precedence order for `dolt.*` settings is:

1. `BEADS_DOLT_*` environment variables (highest precedence).
2. `.beads/metadata.json`.
3. `.beads/config.yaml` (lowest precedence).

This contract respects that order. Any repair that would resolve a value out of order (e.g. setting `dolt_database` in `config.yaml` instead of `metadata.json`) is a typestate violation and rejected.

## Repair surface boundary

The contract authorizes exactly these writes:

- `.beads/config.yaml` — only the keys in `ConfigKey`; only `Set`/`Unset`.
- `.beads/metadata.json` — only the keys in `MetadataKey`; only `Set`/`Unset`.
- `.beads/vb-t0iw9/*.md` and `.beads/vb-t0iw9/*.json` — evidence and recipes.
- `.beads/vb-t0iw9/dispatch/<NN>-<state>.json` — routing-ledger dispatch evidence.

The contract forbids writes to `crates/**`, `verification/**`, `tests/**`, `fuzz/**`, `xtask/**`, `kani/**`, `fixtures/**`, `bd/`, `.beads/dolt/`, `.beads/backup/`, `.beads/embeddeddolt/`, `.beads/dolt-server.port`, and any hook under `.beads/hooks/` (other than read).
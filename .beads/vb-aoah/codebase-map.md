# Codebase Map - vb-aoah

## Bead
- Bead: `vb-aoah`
- Title: `storage: Add explicit migration skeleton and cleanup tests`
- State: go-skill State 2 / explore
- Isolated workspace: `/home/lewis/isolated/femdation-velvet-ballistics/vb-aoah`
- Source checkout is control-plane only: `/home/lewis/src/velvet-ballistics`

## Scope summary
Add tests and the smallest storage/CLI/tooling surface for an explicit storage migration skeleton. Runtime open must not silently mutate old schema. The explicit migration path must name supported old-version migrations, verify migrated records, clean old keyspace entries when required, and advance manifest/version state only after verification.

## Authoritative constraints read
- `velvet-ballistics-MASTER.md:21-25`: runtime is Rust nightly, no-unsafe/no-panic, Fjall persistence and Postcard records; runtime cannot interpret YAML/JSON/HTTP.
- `velvet-ballistics-MASTER.md:43-60`: storage failures are typed; speed claims require evidence.
- `velvet-ballistics-MASTER.md:82-105`: first-party Rust forbids unsafe, unwrap, expect, panic, todo, unimplemented, dbg, unchecked indexing/slicing/casts/arithmetic, ignored Results, unbounded resources.
- `velvet-ballistics-MASTER.md:199-230`: Fjall is required storage, Postcard is required internal record encoding, serde only for binary/data schema or cold diagnostics.
- `Cargo.toml:31-63`: `fjall`, `postcard`, `crc32c`, `blake3`, `thiserror`, `tempfile`, `proptest` already available in workspace.

## Bead acceptance surface
From `bd show vb-aoah --json`:
- New test file requested: `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs` (currently MISSING).
- Scoped test command requested by bead text: `cargo nextest run -p velvet-ballastics-workspace-tests --test restate_explicit_migration_skeleton_tests`.
- Actual workspace package name in `crates/workspace_tests/Cargo.toml:1-3` is `velvet-ballistics-workspace-tests`; downstream should correct the bead command spelling in evidence or note the bead typo.
- Global gate remains `moon ci` per bead text and repo rules.

## Relevant crates and files

### Primary crate: `crates/vb_storage`
- `crates/vb_storage/src/lib.rs:20-34,64-95`: public module declarations. There is no `migrations` module today. Any new public migration API likely needs module registration and re-export here.
- `crates/vb_storage/src/lib.rs:152-160`: `open_store` and `init_keyspaces` call `FjallJournal::open(path, None)`. These are runtime-open paths that must reject old schema with typed `MigrationRequired` and must not run mutation/migration implicitly.
- `crates/vb_storage/src/journal/core.rs:50-67`: `FjallJournal` owns nine Fjall keyspaces plus process lock.
- `crates/vb_storage/src/journal/core.rs:69-122`: `FjallJournal::open` opens/creates database and nine declared keyspaces. This is the choke point for manifest/version checks on runtime open.
- `crates/vb_storage/src/journal/core.rs:124-138`: `FjallJournal::declared_keyspaces()` returns exactly nine current keyspaces. A migration manifest keyspace would collide with existing tests unless handled separately or tests updated deliberately.
- `crates/vb_storage/src/constants.rs:7-24`: current keyspace names: `workflow_source`, `compiled_ir`, `run_header`, `run_event`, `run_snapshot`, `blob`, `index_status`, `index_workflow`, `index_action`.
- `crates/vb_storage/src/constants.rs:45-89`: record header/schema constants, especially `CURRENT_SCHEMA_VERSION: u16 = 1`, record magic values, and payload size limits.
- `crates/vb_storage/src/error/mod.rs:75-88`: existing `JournalError::UnsupportedSchemaVersion` and `JournalError::MigrationRequired { from, to }`. No cleanup/verification/migration-failed error variant currently exists.
- `crates/vb_storage/src/error/codes.rs`: diagnostic mapping exists for `MigrationRequired` (grep evidence); if a new typed migration cleanup/verification error is added, diagnostic code work may be needed.
- `crates/vb_storage/src/codec/validation.rs:10-21`: old record schema versions return `JournalError::MigrationRequired`; future versions return `UnsupportedSchemaVersion`.
- `crates/vb_storage/src/keys.rs:21-130`: typed key encoders and keyspace prefixes. Explicit tests may use these rather than raw unchecked key construction.
- `crates/vb_storage/src/types.rs:173-186`: `FjallConfig`; no storage manifest/version type currently present.
- `crates/vb_storage/Cargo.toml:7-21`: `fjall`, `postcard`, `crc32c`, `blake3`, `thiserror`, `tempfile`, and `proptest` are already in scope; avoid dependency changes unless proven necessary.

### CLI/tool surface
- `crates/vb_cli/src/args.rs:67-216`: command enum has no migration command variant.
- `crates/vb_cli/src/args.rs:218`: valid command list has no migration command.
- `crates/vb_cli/src/app_impl.rs:57-101`: help text has no migration command.
- `crates/vb_cli/src/app_impl.rs:103-208`: dispatch has no migration command.
- `crates/vb_cli/src/app_impl.rs:259-290`: storage open errors map to storage errors; `MigrationRequired` can flow through here if runtime open rejects old schema.
- `crates/vb_cli/src/bench.rs:67-113`: `doctor` opens the journal and writes/reads a test event; cold diagnostic command exists, but not explicit migration.
- `crates/vb_cli/src/storage.rs:17-24,110-160,229-266`: older command implementation also opens `FjallJournal`; current binary dispatch is in `app_impl.rs`, so treat this as secondary/dead or library-adjacent until confirmed by compile references.
- `crates/vb_cli/Cargo.toml:7-23`: CLI already depends on `vb_storage`; no new crate dependency needed for a CLI migration command.

### Workspace tests
- Requested new file: `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs` is MISSING.
- `crates/workspace_tests/Cargo.toml:51-93`: only three explicit `[[test]]` entries for Restate-derived tests; Cargo can still discover integration files, but if this workspace uses explicit entries for targeted tests, adding a `[[test]]` entry may be required for stable command behavior.
- `crates/workspace_tests/tests/restate_fjall_keyspace_manifest_tests.rs:1-11`: existing manifest/prefix test context and command comment.
- `crates/workspace_tests/tests/restate_fjall_keyspace_manifest_tests.rs:169-214`: asserts exactly nine declared keyspaces and required names; this will fail if `declared_keyspaces()` starts including a manifest keyspace.
- `crates/workspace_tests/tests/vb_test_cli_storage_io_behavior.rs` (grep evidence): many reopen tests use `FjallJournal::open`; migration changes must not break normal current-version reopen behavior.

### Existing tests/proofs relevant to schema migration behavior
- `crates/vb_storage/src/tests.rs:947-980`: `decode_rejects_migration_required_schema` patches schema version 0 and expects `MigrationRequired`.
- `crates/vb_storage/src/tests.rs:1355-1389`: `decode_record_returns_migration_required_for_old_schema` duplicates the old-schema decode expectation.
- `crates/vb_storage/src/tests.rs:4625-4645`: adversarial migration test for schema 0 exact fields.
- `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs` (grep evidence): checks `MigrationRequired` diagnostic code `0x400D`.
- `crates/vb_storage/src/kani_record_schema.rs:1-120`: Kani harnesses cover decode schema-version old/current/future behavior, but use fixed header bytes and do not prove explicit migration workflow.
- `crates/vb_storage/src/kani_storage_invariants.rs:248-260`: Kani covers `validate_schema_version` over arbitrary `u16` and expects old versions to map to `MigrationRequired`.
- No located TLA+/Verus/Flux artifact for explicit storage migration workflow in this scope.

## Public/API candidates downstream may need
- New storage API candidate: an explicit function such as `vb_storage::migrations::migrate_store(path, options)` or similar. Must be cold-path/tooling only and not called from runtime `open_store`/`FjallJournal::open` except to reject old versions.
- New metadata/type candidates: manifest/current storage version, old-version fixture version, migration outcome, named migration registry, cleanup/verification result, typed cleanup/verification errors.
- New CLI candidate: `velvet-ballistics storage migrate --db <path>` or a similarly explicit command. Existing CLI command list lacks any storage subcommand hierarchy, so adding a top-level `migrate`/`storage-migrate` command may be less invasive, but command shape needs contract approval by downstream design/implementation.
- Test-only old-version fixture should stay in `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs` unless production parsing needs a public fixture-independent API.

## Risks and collision notes
- `migration`, `persistence`, `public-api`, `user-visible-behavior`: adding a public migration API and/or CLI command changes operator surface and error behavior.
- `manifest/keyspace collision`: `FjallJournal::declared_keyspaces()` currently returns exactly nine keyspaces and existing tests assert that count. A manifest keyspace included there will break `restate_fjall_keyspace_manifest_tests.rs`; if metadata is stored elsewhere or in an existing keyspace, document why.
- `runtime-mutation risk`: `FjallJournal::open`, `open_store`, and `init_keyspaces` currently create keyspaces. Bead requires old schema runtime open returns `MigrationRequired`, not implicit mutation.
- `cleanup atomicity risk`: Fjall migration may need batch/transaction-like ordering. Existing write batch code may be relevant if old cleanup and new writes need fail-closed behavior.
- `typed-error risk`: Existing `MigrationRequired` exists, but cleanup failure / verification failure / unknown old version may need new non-panicking variants and diagnostic codes.
- `No-copy fence`: Restate source is failure-mode inspiration only. Do not import `/tmp/opencode/restate/...` code, API shape, module names, storage layout, wire formats, HTTP/JSON paths, or distributed assumptions.
- `clippy hazards`: existing tests contain `unwrap`/`expect`, but source lint is strict. Production implementation must avoid `unwrap`, `expect`, `panic!`, unchecked indexing/slicing/casts/arithmetic, and ignored Results.
- `package-name mismatch`: bead command says `velvet-ballastics-workspace-tests`; actual package is `velvet-ballistics-workspace-tests`.

## Likely downstream commands
- Focused compile/test after adding the requested file: `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_explicit_migration_skeleton_tests`
- If Cargo cannot discover the new integration file, add a `[[test]]` stanza in `crates/workspace_tests/Cargo.toml` and rerun the same command.
- Storage crate unit blast radius: `cargo nextest run -p vb_storage --lib migration schema` or narrower exact test names once added.
- CLI parse/dispatch blast radius if a command is added: `cargo nextest run -p velvet-ballistics --lib args:: app_impl::` or exact tests once named.
- Formal/proof follow-up candidates: Kani for migration registry/version routing and cleanup outcome lattice; property test for old-keyspace-empty/new-keyspace-exact; maybe Miri if migration code manipulates raw bytes/slices heavily.
- Required final gate per repo: `moon ci`.

## Files inspected
- `.beads/vb-aoah/STATE.md`
- `.beads/vb-aoah/baseline-report.md`
- `.beads/vb-aoah/global-readiness-report.md`
- `.beads/vb-aoah/dispatch-state2-explore-attempt1.json`
- `velvet-ballistics-MASTER.md`
- `Cargo.toml`
- `.moon/tasks/all.yml`
- `crates/vb_storage/Cargo.toml`
- `crates/vb_storage/src/lib.rs`
- `crates/vb_storage/src/mod.rs`
- `crates/vb_storage/src/journal/mod.rs`
- `crates/vb_storage/src/journal/core.rs`
- `crates/vb_storage/src/constants.rs`
- `crates/vb_storage/src/types.rs`
- `crates/vb_storage/src/error/mod.rs`
- `crates/vb_storage/src/codec/validation.rs`
- `crates/vb_storage/src/keys.rs`
- `crates/vb_storage/src/tests.rs` selected migration ranges
- `crates/vb_storage/src/kani_record_schema.rs`
- `crates/vb_storage/src/kani_storage_invariants.rs`
- `crates/workspace_tests/Cargo.toml`
- `crates/workspace_tests/tests/restate_fjall_keyspace_manifest_tests.rs`
- `crates/vb_cli/Cargo.toml`
- `crates/vb_cli/src/args.rs`
- `crates/vb_cli/src/app_impl.rs`
- `crates/vb_cli/src/bench.rs`
- `crates/vb_cli/src/storage.rs`
- `crates/vb_cli/src/commands.rs`
- `to-fix/09-restate-architecture-steal-plan.md` via scoped grep
- `to-fix/10-restate-exhaustive-sweep.md` via scoped grep

## Unknowns / blocked facts
- UNKNOWN: exact intended manifest storage location; no existing storage manifest/version type was found in inspected files.
- UNKNOWN: exact CLI migration command shape; no current command exists.
- UNKNOWN: whether a migration module should be public API or CLI-only wrapper over internal API; downstream contract/design must choose.
- MISSING: `crates/workspace_tests/tests/restate_explicit_migration_skeleton_tests.rs`.

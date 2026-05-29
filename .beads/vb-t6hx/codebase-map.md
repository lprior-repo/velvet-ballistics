# Codebase Map — vb-t6hx State 2 Explore

## Bead
- `vb-t6hx`: `cli: Add doctor storage scan get and envelope decode tests`.
- Scope from `bd show`: add tests in `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs` for CLI doctor/inspect bounded scanner: read-only storage open, bounded preview, envelope decode, projection skip-decode, safe numeric filters, typed parse/decode errors.
- Target test file is currently **MISSING** in this isolated checkout.
- Restate inspiration file `/tmp/opencode/restate/tools/restate-doctor/src/commands/partition_store/scan.rs` is **MISSING** on this host; no Restate code was inspected or copied.

## Master/Repository Constraints
- `velvet-ballistics-MASTER.md` lines 21-25: runtime/core remains single-server, numeric, Fjall durability, Postcard records; no runtime YAML/JSON/HTTP. CLI/doctor diagnostics are cold-path only.
- Master lines 45-60 and 82-105: no unsafe/unwrap/expect/panic/todo/dbg, no unchecked indexing/slicing/casts/arithmetic; errors must be typed.
- AGENTS.md: production code lives under `crates/`; workspace tests under `crates/workspace_tests/tests`; canonical gate is `moon ci`.

## Relevant Crates and Files

### CLI surface (`crates/vb_cli`)
- `crates/vb_cli/Cargo.toml`: package/bin name is `velvet-ballistics`; crate lib name is `vb_cli`; depends on `vb_storage`, `postcard`, `serde_json`, `crc32fast`, `blake3`.
- `crates/vb_cli/src/main.rs`: binary entrypoint delegates to `app_impl::run_from_env()`.
- `crates/vb_cli/src/app_impl.rs`:
  - `run_from_env` dispatches `Command::Doctor { db, output }` to `cmd_doctor(db.as_deref(), output)` at line 173.
  - HELP documents `doctor [--db <path>] [--emit text|yaml|postcard]` at lines 79 and 95-99.
  - `open_doctor_journal(db)` at lines 5495-5510 uses `vb_storage::FjallJournal::open(db, None)` with retry on `ProcessLockHeld`; no read-only open exists here today.
  - `cmd_doctor` at lines 5512-5792 currently opens the journal, calls `persist_strict`, appends a test `RunAccepted` event, reads it back, and runs trim eligibility. This is explicitly mutating and conflicts with this bead’s read-only scanner contract.
  - `cmd_doctor_without_db` at lines 5795-5822 returns stateless success and remediation.
  - Existing structured output machinery uses `OutputFormat`, `emit_json_or_return!`, `json_error`, `write_failure_message`, and `CliExitCode`.
- `crates/vb_cli/src/args.rs`:
  - `Command::Doctor { db: Option<PathBuf>, output: OutputFormat }` at lines 168-171.
  - `parse_args` routes `doctor` to `parse_doctor` at line 372.
  - `parse_doctor` at lines 1357-1361 only accepts known `doctor` flags, optional `--db`, and output `--emit`; no subcommand/scan/get/key/limit/no-color flags exist yet.
  - `validate_known_flags` recognizes `doctor` flags as output plus `--db` at line 1670.
- `crates/vb_cli/src/bench.rs` has an older exported `cmd_doctor(db: &Path)` at lines 67-114 that mutates storage; however active binary dispatch uses `app_impl.rs`. Keep collision risk in mind if refactoring exports.
- `crates/vb_cli/src/io.rs` has older help text and `errln/outln` helpers; active binary help is in `app_impl.rs`.

### Storage surface (`crates/vb_storage`)
- `crates/vb_storage/src/lib.rs` re-exports `FjallJournal`, `JournalError`, `decode_record`, `decode_record_header`, `encode_record`, constants, records, and key APIs.
- `crates/vb_storage/src/journal/core.rs`:
  - `FjallJournal` owns private Fjall `database` and keyspace fields (`workflow_source`, `compiled_ir`, `run_header`, `events`, `run_snapshot`, `blob`, indexes) at lines 50-67.
  - `FjallJournal::open(path, config)` at lines 70-122 opens/creates the database and then acquires an exclusive process lock. No public read-only/open-existing mode was found.
  - `declared_keyspaces()` at lines 124-138 returns all nine keyspace names.
  - Public index existence helpers exist (`has_action_index_entry`, `has_status_index_entry`, `has_workflow_index_entry`) but no generic scan/get APIs.
- `crates/vb_storage/src/constants.rs` defines keyspace names, key prefixes, magic values, `RECORD_HEADER_BYTES=60`, and max payload sizes.
- `crates/vb_storage/src/keys.rs` defines typed key encoders: `workflow_source_key`, `compiled_ir_key`, `run_header_key`, `run_event_key`, `run_snapshot_key`, `blob_key`, index keys, and `encode_key(StorageKey)`. This is likely the safe bridge for CLI hex/key parsing without duplicating layouts.
- `crates/vb_storage/src/journal/replay.rs` provides `get_event_bytes(run, seq)` and bounded `events_for_run_bounded`; this only covers run events, not arbitrary keyspaces.
- `crates/vb_storage/src/journal/source.rs` provides typed `workflow_source(digest)` and `compiled_ir(digest)` retrieval, not raw scanner/get.
- `crates/vb_storage/src/codec/mod.rs` is the canonical envelope path: `encode_record`, `decode_record`, `decode_journal_event`; `decode_record` validates envelope and only then runs `postcard::from_bytes`, returning `JournalError::PostcardDecodeFailed`.
- `crates/vb_storage/src/error/mod.rs` includes typed decode/length/storage errors: `UnexpectedEof`, `BadMagic`, `PayloadTooLarge`, `PayloadDigestMismatch`, `PostcardDecodeFailed`, etc.

### Existing tests relevant to this bead
- `crates/workspace_tests/Cargo.toml`: package is `velvet-ballistics-workspace-tests` (note bead command has typo `velvet-ballastics-workspace-tests`); currently lists explicit test targets only through `restate_runtime_version_barrier_tests`. A new explicit `[[test]]` may be required if the workspace uses explicit test declarations only.
- `crates/workspace_tests/tests/vb_test_cli_run_lifecycle_behavior.rs` lines 698-728 has only shallow doctor behavior tests: no scan/get/decode/readonly assertions.
- `crates/workspace_tests/tests/vb_test_cli_storage_io_behavior.rs` covers storage/replay/resolver behavior through `vb_storage`; it has tempdir and fixture patterns but does not invoke doctor scanner.
- `crates/workspace_tests/tests/restate_postcard_envelope_wire_tests.rs` covers envelope wire proptests for storage `decode_record`/`encode_record`, not CLI doctor decode behavior.
- `crates/vb_storage/src/codec_miri_tests.rs`, `crates/vb_storage/src/kani_codec.rs`, and `crates/vb_storage/src/kani_postcard_envelope_wire.rs` provide existing formal/dynamic safety coverage for decoder ordering and panic-free malformed input, but they are storage-level and not CLI command tests.

## Missing or Unknown
- `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs`: MISSING.
- `/tmp/opencode/restate/tools/restate-doctor/src/commands/partition_store/scan.rs`: MISSING.
- No public `FjallJournal::open_read_only`, `scan_keyspace`, `raw_get`, or generic keyspace iterator API was found. Existing Fjall keyspace fields are private to `vb_storage`.
- No CLI doctor subcommand grammar for `storage scan`, `get`, raw key hex, limit, projection skip-decode, or no-color was found.
- No existing colorized output path was found in inspected doctor code; no-color should likely be stable/plain output by default unless a color layer exists elsewhere.

## Risks for Downstream States
- **persistence/user-visible behavior**: current `cmd_doctor --db` writes a test event and persists; bead requires read-only doctor scan/get and assertion that no key is written.
- **public API**: implementing scanner may require exposing read-only/raw scan APIs from `vb_storage` without leaking raw Fjall internals or runtime doctor types into core/runtime.
- **parser/codec**: hex key parsing and envelope decoding need typed CLI errors and must check length before Postcard decode; reuse storage codec constants/errors.
- **bounded resources/performance**: scan limit and preview length must be bounded; avoid unbounded iteration/collection and avoid speed claims.
- **collision**: two doctor implementations exist (`app_impl.rs` active, `bench.rs` exported older). Avoid changing the inactive path accidentally or leaving divergent semantics if shared exports are introduced.
- **contract drift**: runtime core must not receive doctor-specific types; keep formatting in CLI/cold path.

## Likely Downstream Commands
- Targeted acceptance (corrected package spelling): `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests`.
- Bead text command contains package typo: `velvet-ballastics-workspace-tests`; actual package is `velvet-ballistics-workspace-tests`.
- Existing related smoke: `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_test_cli_run_lifecycle_behavior -- doctor_command`.
- Existing storage codec tests: `cargo nextest run -p vb_storage codec_miri_tests` or crate-scoped equivalent if nextest recognizes lib tests; downstream should choose exact cargo-nextest syntax.
- Final gate per repo: `moon ci`.

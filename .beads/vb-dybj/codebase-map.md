# Codebase Map - vb-dybj State 2 Explore

## Bead
- ID: `vb-dybj`
- Title: `core: Add Postcard newtype compatibility tests`
- Scoped goal: add compatibility/golden tests for Postcard encoding of selected numeric newtypes, digest newtypes, and storage record identifiers. No production/test/proof changes were made in State 2.

## Authoritative constraints inspected
- `velvet-ballistics-MASTER.md:21-25`: runtime core is no-unsafe/no-panic, no YAML/JSON/HTTP runtime interpretation; Fjall persistence and Postcard compact binary records are required.
- `velvet-ballistics-MASTER.md:47-55`: runtime/core must avoid JSON/HTTP and uses Postcard for journal/snapshot/IPC/compiled artifact records.
- `velvet-ballistics-MASTER.md:199-230`: `postcard` is the required compact binary library; `serde` is allowed for binary/data schema serialization.
- `AGENTS.md`: production code lives under `crates/`; cross-crate tests belong under `crates/workspace_tests/`; canonical gate is `moon ci`; no unsafe/unwrap/expect/panic/todo/unimplemented/dbg in production code.

## Bead inputs inspected
- `.beads/vb-dybj/STATE.md`: State 2 explore is ready; isolated workspace is `/home/lewis/isolated/femdation-velvet-ballistics/vb-dybj`.
- `.beads/vb-dybj/baseline-report.md`: source checkout is control-plane only; active isolated worktree branch `femdation/vb-dybj-20260525-h1`.
- `.beads/vb-dybj/global-readiness-report.md`: no known global blocker for State 2.
- `.beads/vb-dybj/dispatch-state2-explore-attempt1.json`: required outputs are this file and `delivery-scope.jsonl`.
- `bd show vb-dybj --json`: target test file is `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs`; scoped commands are the named `cargo nextest` test and `moon ci`.

## Relevant public API and source files

### `crates/vb_core/src/ids/mod.rs`
- Symbols:
  - `numeric_id!` macro at lines 9-40 derives `Serialize`/`Deserialize` for transparent numeric newtypes.
  - `RunId` declared at line 65 as `u64`, with `RunId::ZERO` at lines 229-232 and `RunId::new/get` from the macro at lines 18-29.
  - Other numeric newtypes in the same macro family: `WorkflowId`, `StepIdx`, `SlotIdx`, `ExprIdx`, `ActionId`, `AccessorIdx`, `ConstIdx`, `SymbolId`, `ListId`, `ObjectId`, `BlobId`, `EventSeq`, `SeqNo` at lines 54-67.
  - `WorkflowDigest([u8; 32])` at lines 339-356 derives `Serialize`/`Deserialize` and exposes `from_bytes` / `as_bytes`.
- Existing local tests in the same file cover getter/constructor behavior and digest accessors, but do not provide fixed Postcard golden bytes for these newtypes.
- Contract note: `RunId::new(0)` is permitted by constructor; `RunId::ZERO` is explicit. Bead edge-case tests should assert zero behavior matches that existing contract, not invent a non-zero invariant.

### `crates/vb_storage/src/records.rs`
- Symbols:
  - `RunHeaderStatus(u8)` at lines 11-13; lossless persisted byte status newtype with constants at lines 44-63.
  - `RecordKind` at lines 135-188 is `#[repr(u16)]`, derives `Serialize`/`Deserialize`, and has explicit stable IDs.
  - `RecordKind::id()` at lines 190-221 maps all known variants to their persisted `u16` IDs.
  - `RunHeaderRecord` at lines 241-258 includes `run: RunId`, `workflow_id: WorkflowId`, `compiled_digest: WorkflowDigest`, `status: u8`, `accepted_at_ms: u64`.
- Bead-required happy path explicitly mentions `RecordKind` golden bytes. Golden tests should likely cover `RecordKind::RunAccepted` and at least one persisted record family variant such as `RecordKind::RunHeader` or `RecordKind::CompiledIr`, depending on downstream contract choice.

### `crates/vb_storage/src/codec/mod.rs`
- Symbols:
  - `encode_record<T: Serialize>` lines 20-32 serializes payload using `postcard::to_allocvec` behind the fixed 60-byte envelope.
  - `decode_record<T: DeserializeOwned>` lines 34-44 obtains a bounded payload slice, then maps any `postcard::from_bytes` failure to `JournalError::PostcardDecodeFailed`.
  - `decode_journal_event` lines 46-64 adds semantic validation for `JournalEvent`.
- Scope relevance: direct fixed Postcard newtype tests may use `postcard::to_allocvec`/`from_bytes` directly; envelope tests should use these storage APIs only if testing envelope behavior.

### `crates/vb_storage/src/codec/payload.rs`
- Symbols:
  - `payload_len_u32` lines 20-32 bounds encoded payload length.
  - `decode_record_payload` lines 56-82 checks header, computes checked payload slice boundaries, verifies digest, and returns the payload bytes consumed by Postcard.
- Error mapping: missing/truncated envelope bytes return `JournalError::UnexpectedEof` before Postcard decode.

### `crates/vb_storage/src/codec/header.rs`
- Symbols:
  - `decode_record_header` lines 26-58 rejects short headers, bad magic, schema migration/version issues, unknown kind, wrong family, bad header length, oversized payload, and CRC mismatch before payload decoding.
  - `build_record_header` lines 60-78 writes fixed little-endian fields and payload digest.
- Collision warning: bead asks for Postcard newtype compatibility, not full envelope header compatibility. Do not duplicate broad envelope property coverage unless needed for typed short-decode acceptance.

### `crates/vb_storage/src/error/mod.rs` and `crates/vb_storage/src/error/codes.rs`
- Symbols:
  - `JournalError::UnexpectedEof` at lines 123-125 is the typed short-record error.
  - `JournalError::PostcardDecodeFailed` at lines 126-128 is the typed postcard decode error.
  - Diagnostic codes: `UNEXPECTED_EOF_CODE` is `0x4014`; `POSTCARD_DECODE_FAILED_CODE` is `0x4015` at `codes.rs:45-48`.
- Scope relevance: acceptance says trailing bytes should return typed decode error and missing bytes typed short decode error. Direct `postcard::from_bytes` returns `postcard::Error`; storage `decode_record` maps postcard failures to `JournalError::PostcardDecodeFailed` only after envelope validation.

### `crates/vb_storage/src/lib.rs`
- Re-exports `decode_record`, `decode_record_header`, `encode_record`, `encode_record_header`, `RecordKind`, `JournalError`, constants, and record types at lines 101-140. Workspace tests can import from `vb_storage` public surface.

## Relevant existing tests
- `crates/workspace_tests/Cargo.toml:87-89` registers `restate_postcard_envelope_wire_tests`; no entry exists yet for `restate_postcard_newtype_compat_tests`.
- `crates/workspace_tests/tests/restate_postcard_envelope_wire_tests.rs` exists and exercises storage envelope encoding/decoding via `encode_record`/`decode_record`, `RecordKind`, `JournalEvent`, and `RunId`.
- `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` is MISSING and is the bead-specified new test file.
- `crates/vb_core/src/value_store/extended_tests.rs` and `crates/vb_core/src/value.rs` contain existing Postcard roundtrip tests for `SlotValue` and `Taint`, but not fixed golden-byte compatibility tests for `RunId`, `WorkflowDigest`, or `RecordKind`.
- `crates/vb_storage/src/codec/tests.rs` contains extensive encode/decode roundtrip and error coverage for storage records; it uses `expect`/`unwrap` allowances in tests and should not be treated as production style.

## Relevant verification/proof artifacts
- `crates/vb_storage/src/kani_postcard_envelope_wire.rs` is registered under `#[cfg(kani)]` in `crates/vb_storage/src/lib.rs:61-62` and verifies decode ordering for the storage envelope.
- The Kani proof file contains unchecked indexing/slicing inside harness code (for example header byte writes), so downstream proof work must respect repository GOD RULES if adding/altering Kani. This bead appears test-focused; do not mutate proof artifacts unless a later state explicitly expands scope.
- No Verus/TLA+/Flux artifact was found or needed for this fixed-wire compatibility test scope during focused exploration.

## Missing or external references
- `/tmp/opencode/restate/crates/encoding/src/common.rs`: MISSING in this environment. Treat Restate as inspiration only per bead no-copy fence; do not block State 3 on this file if unavailable.
- `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs`: MISSING; expected to be created by downstream test/implementation state.
- `crates/workspace_tests/Cargo.toml`: currently lacks a `[[test]]` entry for `restate_postcard_newtype_compat_tests`; downstream must add the test target or Cargo will not run the named `--test` command.

## Risks and constraints for downstream states
- Risk tags: `parser/codec`, `persistence`, `public API`, `migration`, `dependency`.
- Golden bytes are wire-format commitments. Changing them should require a named migration; tests should make this visible in test names/messages.
- Postcard varint behavior means `u64` newtype fixed value bytes are not simply eight little-endian bytes. Downstream should generate/confirm exact fixtures from the current `postcard` version and then freeze them in tests.
- `RecordKind` uses enum serialization, not `RecordKind::id()` directly unless wrapped/converted. Downstream must decide whether the compatibility contract is Postcard enum representation of `RecordKind` or explicit persisted kind ID (`u16`) used in storage headers; acceptance says `RecordKind Postcard bytes`, while storage envelope writes `kind.id()` via little-endian header helpers.
- Trailing bytes: direct `postcard::from_bytes::<RunId>(&[valid..., extra])` should fail with postcard's own trailing-byte error; storage `decode_record` maps postcard decode failure to `JournalError::PostcardDecodeFailed` only when the envelope declares a longer payload that passes digest/header checks. Tests must pick the intended public surface explicitly.
- Missing bytes: direct Postcard missing bytes returns `postcard::Error`; storage envelope short records return `JournalError::UnexpectedEof`. Acceptance says typed short decode error, so storage API may be required for that assertion unless a local helper maps raw postcard errors.
- No Bilrost/Protobuf should be introduced; no dependency changes are indicated.
- No JSON wrapper: tests should assert Postcard byte fixtures directly and avoid serde_json or text wrappers in runtime/core paths.
- `cargo nextest` command in bead description has a package typo: `velvet-ballastics-workspace-tests`; actual package in `crates/workspace_tests/Cargo.toml:2` is `velvet-ballistics-workspace-tests`. Downstream should use the actual package name unless controller explicitly requires reproducing the typo as evidence.

## Likely commands for later states
- Scoped test after adding target: `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests`
- Canonical repository gate: `moon ci`
- Optional focused compile check before nextest if needed: `cargo test -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests --no-run`

## Recommended downstream focus
1. Add `[[test]]` entry to `crates/workspace_tests/Cargo.toml` for `restate_postcard_newtype_compat_tests`.
2. Create `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` with golden fixtures for:
   - `vb_core::RunId` zero, representative, and `u64::MAX` values.
   - `vb_core::WorkflowDigest` or another digest newtype if contract chooses digest coverage.
   - `vb_storage::records::RecordKind` or its persisted ID, with explicit migration-test naming.
   - Typed decode errors for trailing and short inputs on the chosen public decode surface.
3. Avoid production changes unless tests reveal missing typed API; if production is needed, keep it minimal and public-surface scoped to typed decode helpers.

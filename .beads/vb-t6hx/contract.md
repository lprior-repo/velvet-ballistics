# Contract — vb-t6hx

## Purpose

Add domain/type contract for tests and minimal implementation of CLI doctor storage scan/get and envelope decode behavior. This artifact defines required semantics only; it does not write tests, production code, verifier artifacts, or proof plans.

## Functional Contract

1. `doctor storage scan` parses into a typed scan request with declared keyspace, bounded scan limit, bounded preview limit, optional prefix/filter, output format, no-color flag, and decode mode.
2. `doctor storage get` parses into a typed get request with declared keyspace, exact `HexKey`, bounded preview limit, output format, no-color flag, and decode mode.
3. Invalid keyspace, invalid hex key, invalid numeric limit/filter, and conflicting flags fail before opening storage.
4. Storage scan/get uses a read-only capability. It must not append events, persist as a write test, create synthetic run IDs, delete keys, compact, or migrate records.
5. Scan emits at most the requested `ScanLimit` rows.
6. Get returns `Found` or typed `NotFound`; missing key is not a generic storage error.
7. Large values render as bounded previews with explicit truncation metadata and a hint to use raw get or larger bounded preview.
8. `--no-color` renders stable plain output. If color is not implemented elsewhere, `--no-color` is an accepted no-op over already-plain output.
9. Projection scan defaults to skip-decode: it must not Postcard-decode every value.
10. Envelope decode validates header length, magic, schema, record kind family, payload length bound, header CRC, payload availability, and payload digest before Postcard decode.
11. Decode errors preserve categories corresponding to `JournalError` decode variants.

## Non-Functional Contract

- Cold CLI formatting may use diagnostic serialization, but `vb_core`, `vb_runtime`, `vb_storage` runtime hot paths, and `vb_ipc` must not gain JSON/YAML/HTTP behavior.
- All new first-party Rust must preserve repository bans: no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing/casts/arithmetic, ignored fallible results, or unbounded resources.
- Any storage API added for this bead must be capability-separated: read-only query type separate from writer journal type, or an equivalent type-level restriction.

## Acceptance-Behavior Contract Seeds

Downstream tests should prove these externally visible behaviors:

- scan limit: fixture with more rows than limit renders no more than limit;
- raw get: exact key returns bounded preview and length metadata;
- missing key: exact absent key returns typed not-found diagnostic;
- invalid hex: bad key fails as parse error before storage open;
- large value: preview is truncated and includes omitted-byte/hint information;
- no-color: output contains stable plain legend and no color escapes;
- read-only: doctor storage command does not add journal events or keys;
- envelope good fixture: decode prints expected header fields;
- envelope malformed/truncated: length/eof error occurs before Postcard failure;
- projection skip-decode: scan can list keys/preview malformed envelope values without surfacing Postcard decode failure unless decode requested.

## Source Binding

- Current active doctor dispatch: `crates/vb_cli/src/app_impl.rs::cmd_doctor` is mutating and must not be reused for read-only scan/get as-is.
- Current parser: `crates/vb_cli/src/args.rs::parse_doctor` accepts only `--db` and `--emit`; typed storage subcommands are absent.
- Current storage: `FjallJournal::open` acquires a process lock and exposes writer methods; no read-only/raw scan/get public API was found.
- Current codec: `vb_storage::decode_record` and `decode_journal_event` are canonical decode paths and must anchor envelope behavior.

## Out of Scope

- No implementation in this state.
- No behavior tests in this state.
- No verifier harnesses or proof obligations in this state.
- No Restate code/API copying; Restate remains failure-mode inspiration only.

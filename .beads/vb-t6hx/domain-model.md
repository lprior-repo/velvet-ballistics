# Domain Model — vb-t6hx

## Bead

- Bead: `vb-t6hx`
- Scope: `cli: Add doctor storage scan get and envelope decode tests`
- State: go-skill State 3, rust-contract domain/type contract only

## Ubiquitous Language

| Term | Meaning | Boundary |
|---|---|---|
| Doctor storage scanner | Cold CLI diagnostic that opens an existing Fjall journal without mutating it and inspects bounded rows from declared keyspaces. | `vb_cli` shell plus storage query API |
| Scan | Bounded iteration over a declared storage keyspace, optionally filtered by typed numeric/key prefix filters. | CLI command and storage read API |
| Get | Read one exact key from one declared keyspace and render a bounded preview or raw/decode diagnostic. | CLI command and storage read API |
| Envelope decode | Decode a storage record envelope by validating header, family, length, checksum, digest, then Postcard payload. | `vb_storage::decode_record`/`decode_journal_event` |
| Projection skip-decode | Scanner mode that reads keys and bounded value previews without Postcard decode. Decode must be opt-in per row/value. | CLI formatting policy |
| Read-only open | Open-existing storage handle that cannot create user records, append events, call persist as a write check, or acquire a writer-only mutation path. | `vb_storage` capability type |
| Preview | Bounded, stable text/structured representation of key/value bytes. Large values are truncated with explicit omitted-byte count and raw-get hint. | CLI formatting |
| Typed diagnostic | User-visible parse/storage/decode failure with stable category/code and no stringly control flow. | CLI shell |

## Actors

- Operator: runs `velvet-ballistics doctor ...` to inspect local storage.
- CLI parser: turns argv into typed doctor commands; rejects invalid hex, keyspaces, limits, and filters before storage access.
- Storage read boundary: exposes read-only scan/get over declared keyspaces without leaking Fjall internals into CLI.
- Codec boundary: validates record envelopes using existing `vb_storage` rules.
- Workspace tests: prove observable CLI behavior, read-only non-mutation, scan limits, previews, and decode errors.

## Entities and Value Objects

- `DoctorCommand`: `HealthCheck` or `Storage(StorageDoctorCommand)`; storage scanner/get is not a boolean flag on old doctor behavior.
- `StorageDoctorCommand`: `Scan(ScanRequest)` or `Get(GetRequest)`.
- `StorageKeyspaceName`: closed enum over the nine declared keyspaces: `workflow_source`, `compiled_ir`, `run_header`, `run_event`, `run_snapshot`, `blob`, `index_status`, `index_workflow`, `index_action`.
- `HexKey`: even-length, non-empty byte string parsed from hex; invalid nybbles and odd lengths are impossible after construction.
- `ScanLimit`: non-zero bounded row count; no implicit unlimited scan.
- `PreviewLimit`: non-zero bounded byte count; no unbounded value rendering.
- `DecodeMode`: `SkipDecode`, `EnvelopeHeader`, `EnvelopePayload`; scan defaults to `SkipDecode` unless explicitly requested.
- `NoColorMode`: stable plain output mode; if no color layer exists, it is an idempotent formatting request, not a behavior flag that changes data.
- `ReadOnlyStorage`: capability type exposing only `scan_bounded` and `get_exact`; it has no append/persist/delete methods.
- `DoctorRow`: key bytes plus value metadata, bounded preview, and optional decode summary.
- `EnvelopeDecodeSummary`: magic, schema version, record kind, header length, payload length, sequence, and decode result.

## Aggregates

### Doctor Storage Inspection Session

Root: `DoctorStorageInspection`

State held for one CLI invocation:
- parsed storage command
- read-only storage capability
- bounded output accumulator
- stable diagnostics

Invariants:
- storage path is required for storage scan/get;
- keyspace is declared before opening or querying;
- scan limit and preview limit are explicit bounded values;
- `Get` returns exactly one of `Found`, `NotFound`, or typed failure;
- scan rows emitted are `<= ScanLimit`;
- skip-decode mode never calls Postcard payload decode;
- envelope decode validates length/checksum/digest before Postcard;
- invocation does not append, persist, create test events, or mutate any key.

## Policies

- CLI diagnostics are cold path only; no doctor types may enter `vb_core`, `vb_runtime`, or hot storage writer paths.
- Storage key layout must be reused through `vb_storage` key/keyspace APIs; CLI must not duplicate key encoders.
- Unknown keyspace, invalid hex, missing key, decode failure, oversized preview, and lock/open failures are distinct typed errors.
- Text output in `--no-color` mode is stable and plain; structured output may include the same categories without affecting runtime core.
- Bead implementation starts with failing tests in `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs`; this contract does not write those tests.

## Illegal States to Make Unrepresentable

- A storage scan without a positive bounded limit.
- A preview request without a positive bounded byte cap.
- A raw key represented as `String` after parse.
- A keyspace represented as arbitrary `String` after parse.
- A read-only doctor path that has access to append/persist/delete methods.
- A scan row carrying unbounded `Vec` output solely for display.
- Decode success without validated envelope header and payload digest.
- Missing-key returned as generic storage error.
- Parse errors collapsed into one catch-all message.

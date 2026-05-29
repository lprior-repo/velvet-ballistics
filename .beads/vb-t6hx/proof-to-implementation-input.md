# Proof-to-Implementation Input — vb-t6hx (Reduced Scope)

This State 4 handoff tells State 7 which planned proof claims must map to Rust source, behavior tests, refinement harnesses, and final evidence. Reduced scope: proptest, Kani, cargo-fuzz, and behavior tests only. Verus/Flux/TLA+/Loom/Miri excluded. This is not a bridge approval.

## Source Targets from Contract

- CLI parser/dispatch: `crates/vb_cli/src/args.rs`, `crates/vb_cli/src/app_impl.rs`.
- Storage read boundary: `crates/vb_storage/src/journal/core.rs`, public storage module exports, and any new read-only diagnostic API.
- Storage key layout: `crates/vb_storage/src/keys.rs`; CLI must not duplicate unchecked key encoders.
- Codec spine: `crates/vb_storage/src/codec/mod.rs`, `crates/vb_storage/src/error/mod.rs`.
- Workspace behavior tests: `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs` (primary evidence channel).
- Existing related proof/safety artifacts to reuse or extend: `crates/vb_storage/src/kani_postcard_envelope_wire.rs`, `crates/vb_storage/src/codec_miri_tests.rs`.
- Fuzz targets: `fuzz/fuzz_targets/` for new fuzz harnesses.

## Claim Mapping Seeds

| Claim | Planned proof IDs | Expected implementation mapping |
|---|---|---|
| Read-only doctor scan/get cannot mutate records or user keys. | `PO-vb-t6hx-R17`, `PO-vb-t6hx-R18` | Map to typed `ReadOnlyStorage` or equivalent boundary; no access to append/persist/delete/compact/migrate. Behavior tests compare before/after inventory. Kani harness uses arbitrary generators for command/capability selection. |
| Scan emits at most `ScanLimit` rows and does not collect all then truncate. | `PO-vb-t6hx-R01`, `PO-vb-t6hx-R02`, `PO-vb-t6hx-R03` | Map to `ScanLimit` constructor, bounded iterator loop/adapter, bounded row accumulator. Kani harness with arbitrary generators; proptest with generated fixture sizes; fuzz with hostile argv. |
| Invalid hex and invalid parser inputs fail before storage open. | `PO-vb-t6hx-R04`, `PO-vb-t6hx-R05`, `PO-vb-t6hx-R06` | Map to `HexKey` parser, typed `DoctorParseError`, storage-open spy showing no open on parse error. Kani harness with arbitrary bounded input; proptest covers odd/empty/invalid nybble cases. |
| Envelope decode order preserves length/integrity-before-Postcard. | `PO-vb-t6hx-R07`, `PO-vb-t6hx-R08`, `PO-vb-t6hx-R09`, `PO-vb-t6hx-R10` | Map to canonical `vb_storage::decode_record`/`decode_journal_event` or wrapper. Extend existing `kani_postcard_envelope_wire.rs`. Fuzz targets for both raw envelope bytes and CLI doctor decode path. |
| Projection scan defaults to skip-decode. | `PO-vb-t6hx-R14`, `PO-vb-t6hx-R15`, `PO-vb-t6hx-R16` | Map to `DecodeMode::SkipDecode` default and row projection branch that returns key/value preview without calling payload decode. Kani harness with arbitrary bounded malformed values. |
| Large values render bounded previews with truncation metadata and hint. | `PO-vb-t6hx-R11`, `PO-vb-t6hx-R12`, `PO-vb-t6hx-R13` | Map to `PreviewLimit`, `BoundedPreview`, omitted-byte calculation. Kani harness with arbitrary bounded byte arrays; fuzz for adversarial value bytes/preview args. |
| Doctor storage types/formatting stay outside runtime core. | (behavior tests + source inspection only) | Map to crate/module placement and source checks excluding `vb_core`, `vb_runtime`, `vb_ipc`, action ABI, and hot workflow validation/compile paths. No formal obligation; seed 7 is source-inspection evidence. |

## Required Bridge Checks for State 7

1. Every planned proof obligation must have a concrete `source_refs` entry or an explicit reviewer-accepted non-applicable bridge reason.
2. Kani harnesses must generate/arbitrate core structures with `kani::Arbitrary` or `kani::any()`; no fixed dummy shape proof is acceptable. This includes existing harnesses that may need extension.
3. Fuzz harnesses must target production parser/codec/preview functions, not duplicated models. Fuzz artifacts go under `.beads/vb-t6hx/fuzz-artifacts/`.
4. Postcard decoder is an external dependency; we prove only that it is called after envelope validation, not its internals. CRC/BLAKE3 treated as external equality oracles.
5. Behavior tests remain independent of verifier harnesses; verifier harnesses are not acceptance tests. Behavior tests are the primary evidence channel for all 10 contract seeds.
6. The non-behavior waiver candidate (WC-vb-t6hx-001) is invalidated by any dependency manifest change or any runtime/core boundary drift.

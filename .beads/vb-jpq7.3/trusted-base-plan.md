# Trusted Base Plan: vb-jpq7.3

## Trusted Components

- `fjall::Database::persist(PersistMode::SyncAll)` implements the storage durability barrier; this bead verifies that its `Result` is exposed, not Fjall's internals.
- `fjall` prefix/range iteration honors key ordering and prefix boundaries.
- `crate::codec::decode_record` validates record magic, schema, size, checksum/digest, and postcard payload before returning typed records.
- `postcard` serialization/deserialization is trusted for byte-to-type decoding once `decode_record` admits the payload family.
- `vb_core::RunFrame` enforces slot/taint bounds and reports `CoreError` variants.

## Untrusted Inputs

- Journal event records.
- Snapshot records and snapshot keys.
- Tail event sequences after a snapshot.
- Existing frame taint reads during recovery hydration.
- Storage persistence failures.

## Fail-Closed Requirements

- Snapshot authority is trusted only after key and payload decode agree on `run` and `seq`.
- Replay success requires exact contiguous sequence starting from `0` without snapshot or `snapshot.seq + 1` with snapshot.
- Collection growth is bounded by `EventReplayLimit` and checked/try-reserved.
- Taint read failures other than `SlotUninitialized` become `RecoveryError::SlotTaintReadFailed`.
- Durability failure is observable via explicit `close()`/`persist_strict()` result.

## Residual Trusted-Base Risk

- No disk-fault simulator was introduced; strict persist failure is represented by a test-only hook at the Rust boundary.
- Verus replay and recovery artifacts are auxiliary/spec-seam evidence until proof-review explicitly accepts their scope or they are bound to production exec functions.
- Full repository formatting and canonical `moon ci` are currently passing after the versioned slot-write extra envelope plus full-journal taint/scanner/runtime-encode/supply-chain repair; latest closure evidence is `/home/lewis/.local/share/opencode/tool-output/tool_e54cfc867001em3UkY7dnDZZ7z`.

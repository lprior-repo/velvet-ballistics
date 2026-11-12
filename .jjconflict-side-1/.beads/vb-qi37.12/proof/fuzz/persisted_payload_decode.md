# Fuzz Target Evidence: Persisted Payload Decode

- Obligation: `FUZZ-DECODE-009`.
- Status: `PASS_WITH_ENV_REPAIR`.
- Target: `vb_qi37_12_persisted_payload_decode` wired in `fuzz/Cargo.toml`, `fuzz/src/bin/vb_qi37_12_persisted_payload_decode.rs`, and `fuzz/src/lib.rs`.
- Command attempted first: `TMPDIR=target/tmp cargo fuzz run vb_qi37_12_persisted_payload_decode -- -runs=1000`; target was present, but local default musl/static sanitizer config failed before execution.
- Executed command: `TMPDIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12/target/tmp RUSTC_WRAPPER= CARGO_TARGET_DIR=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12/target/tmp/cargo-fuzz cargo fuzz run vb_qi37_12_persisted_payload_decode --target x86_64-unknown-linux-gnu -- -runs=1000`.
- Result: built `velvet-ballistics-fuzz` and launched the target with `-runs=1000`; no crash artifact was reported.
- Oracle: arbitrary malformed bytes are decoded through `vb_storage::decode_record::<JournalEvent>` and must return typed `JournalError`; generated truncated records must return `UnexpectedEof`; generated corrupted records must return `PayloadDigestMismatch`; neither path can hydrate as empty success.
- Model link: `.beads/vb-qi37.12/proof/verus/recovery_decode_class.rs` proves the abstract corrupt/truncated-not-success classifier separation used by this fuzz oracle.

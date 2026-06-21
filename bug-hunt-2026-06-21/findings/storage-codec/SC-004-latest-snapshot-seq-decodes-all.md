# SC-004: `latest_durable_snapshot_seq` decodes every snapshot value just to find the max key sequence

- **Severity**: Medium
- **Category**: perf
- **Location**: `crates/vb_storage/src/trimming/logic.rs:17-56`
- **Confidence**: confirmed

## Description

`latest_durable_snapshot_seq` scans all snapshots for a run and returns the one with the highest sequence. The desired sequence number is already encoded in the key (bytes 9..17). Despite this, the function fully decodes each snapshot value via `decode_record` (which runs BLAKE3 verification and postcard deserialization) purely to assert that `snapshot.run == run` and `snapshot.seq == key_seq`. This is O(N · payload_size) per run on every trim pass.

## Evidence

```rust
// crates/vb_storage/src/trimming/logic.rs:21-53
for item in self.run_snapshot.prefix(prefix_key) {
    let (key, value) = item.into_inner().map_err(TrimError::from)?;
    if key.len() < 17 {
        return Err(TrimError::IncompleteTrim { deleted_count: 0 });
    }
    let slice = key.get(9..17).ok_or(...)?;
    let seq_bytes: [u8; 8] = slice.try_into().map_err(...)?;
    let key_seq_u64 = u64::from_be_bytes(seq_bytes);
    let key_seq = EventSeq::new(key_seq_u64);
    let (_, snapshot): (_, crate::recovery::RunSnapshot) =
        decode_record(value.as_ref(), MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES)   // <-- BLAKE3 + postcard
            .map_err(TrimError::from)?;
    if snapshot.run != run { ... }
    if snapshot.seq != key_seq { ... }
    latest = Some(match latest {
        Some(current) if current.get() >= key_seq_u64 => current,
        _ => key_seq,
    });
}
```

The `decode_record` call hashes up to `MAX_SNAPSHOT_BYTES` (64 MiB) per snapshot. Because `prefix_key` already constrains by run id, and the keyspace is sorted in big-endian order, the maximum-seq snapshot is the **last** key in the prefix scan — a single reverse iterator step would replace the entire loop.

## Adversarial Check

The defensive decode guards against key/value inconsistency (e.g., a snapshot written under the wrong key). That guard belongs at write time (in `put_snapshot`) or in a one-shot integrity sweep, not in a hot trim-decision path. The current loop re-hashes every snapshot on every `trim_events_for_run`/`trim_eligibility_diagnostic` call. With long-lived runs that accumulate dozens of snapshots, this turns trim decisions into multi-second BLAKE3 traversals. The perf claim is justified because the function sits behind `trim_all_eligible_runs` (`crates/vb_storage/src/trimming/logic.rs:116`) which itself iterates every run header.

## Suggested Fix

Either (a) replace the forward scan with a single `iter().reverse().next()` over the run-prefix range (Fjall supports reverse iteration) and trust the key, or (b) split the function into a fast `peek_latest_snapshot_seq(run)` that reads only the last key, plus an opt-in `verify_snapshot_consistency` invoked by doctor.

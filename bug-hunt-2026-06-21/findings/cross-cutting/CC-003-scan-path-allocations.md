# CC-003: Byte-allocation patterns across scan paths produce per-iteration `Vec` allocations

- **Severity**: Low
- **Category**: perf
- **Location**: `crates/vb_storage/src/trimming/logic.rs:87` (`key.to_vec()`), `crates/vb_storage/src/queue/batch.rs:5-37` (`Vec<JournalEvent>` growth), `crates/vb_storage/src/keys/encode.rs:125-151` (`encode_key` `.to_vec()` per variant), `crates/vb_storage/src/headers.rs:51-62` and `artifacts.rs:16-26` (Vec::new for results), `crates/vb_storage/src/preview.rs:93` (`value_bytes.clone()`)
- **Confidence**: confirmed

## Description

Multiple scan paths in the storage layer allocate a fresh `Vec` per iteration where a single reused scratch buffer or a borrowed slice would suffice. None of these is in a tight CPU-bound loop, but several sit behind hot Fjall keyspace scans and contribute measurable allocator pressure under load.

## Evidence

1. **`trimming/logic.rs:87`**: `batch.remove(&self.events, key.to_vec())` allocates a 17-byte `Vec` per trim candidate. See SC-008 for the full finding.

2. **`keys/encode.rs:125-151`** `encode_key`:
   ```rust
   pub fn encode_key(key: crate::types::StorageKey) -> Result<Vec<u8>, JournalError> {
       let encoded = match key {
           StorageKey::WorkflowSource { digest } => workflow_source_key(digest)?.to_vec(),
           StorageKey::CompiledIr { digest } => compiled_ir_key(digest)?.to_vec(),
           ...
       };
       Ok(encoded)
   }
   ```
   Each variant produces a fixed-size array via the typed encoder, then `.to_vec()` heap-allocates it. Callers that immediately pass the result to a Fjall insert (which takes `Into<Vec<u8>>`) pay one extra allocation.

3. **`headers.rs:50-63`** `run_headers` and **`artifacts.rs:16-27`** `list_artifacts`: build a `Vec` of results without `try_reserve` or any cap. `run_headers` is the same function used by the O(N²) trim scan (SC-005) — every additional allocation compounds.

4. **`preview.rs:93`**: `result_entries.push((key, value_bytes.clone(), PreviewPayload::Raw))` clones every value bytes vector unconditionally. Preview is bounded but the bound is per-call; on a 10 000-row keyspace the function performs 10 000 `Vec` clones.

5. **`queue/batch.rs:16-18`** `BatchBuilder::push`: grows `Vec<JournalEvent>` without `try_reserve_exact`. See SA-006.

## Adversarial Check

Each allocation in isolation is small (17 bytes for keys, ~60+payload bytes for values, ~100 bytes for an event struct). The defect is aggregate: Fjall keyspace scans routinely visit thousands of rows, and the storage layer is meant to be the high-throughput durability layer. Under jemalloc/system allocator, these small allocations hit the size-class free lists efficiently but still cost cycles; under mimalloc or scudo, the picture is similar. The functional-rust doctrine (Holzman NASA/JPL rule "Minimize allocations in service of deterministic latency") calls for reusing scratch buffers in scan bodies. The hot-path justification is that all five sites sit directly inside Fjall `prefix`/iterators that may visit many rows per call.

## Suggested Fix

- `trim_events_for_run`: reuse a single `key_buf: Vec<u8>` outside the loop (see SC-008).
- `encode_key`: add an `encode_key_into(key, &mut Vec<u8>)` variant that callers can use to share a scratch buffer.
- `run_headers`/`list_artifacts`: `try_reserve(initial_capacity)` based on a configured hint.
- `preview_keyspace`: keep the clone (the caller owns `entries` and the result must outlive it) but document that the function performs N clones bounded by `max_records`.
- `BatchBuilder`: bound the growth (see SA-006).

# Boundary Map — vb-7m21

## Pure Core Boundary

- Fixture metadata validation: IDs, coverage, expected outcome compatibility.
- Deterministic mutation descriptions.
- Outcome classification from typed errors.

## Parser / Codec Boundary

- `decode_record_header` and `decode_record` are the primary binary envelope boundary.
- Header validation order is part of the contract: length, magic, schema, kind, family, header length, declared payload bound, CRC, payload digest, Postcard decode.
- No payload allocation before declared length is checked against family max.

## Storage Boundary

- Fjall temporary stores are allowed only under isolated test tempdirs.
- Fixture runner may write corrupt/missing index state only in disposable stores.
- Production data mutation is forbidden.

## Index Boundary

- Missing-index fixtures must use `run_event_key`, `index_status_key`, `index_workflow_key`, and `index_action_key`-style typed key constructors or public APIs.
- String keys or Restate-style key layouts are forbidden.

## Snapshot / Recovery Boundary

- Snapshot fixtures use VB `RunSnapshot` and snapshot envelope constants.
- Stale/corrupt snapshot classification must use existing typed recovery/storage errors or an explicit local corpus classification.

## External Reference Boundary

- `/tmp/opencode/restate/.../record_format.rs` was unavailable during exploration.
- Restate is only a source of failure-mode names. No implementation/test artifact may copy Restate bytes, layouts, APIs, or storage model.

## Async / Network / HTTP / JSON Boundary

- Out of scope. The corpus is local, deterministic, synchronous test infrastructure.
- Runtime JSON/YAML/HTTP interpretation remains forbidden.

## Unsafe / FFI Boundary

- No first-party unsafe, FFI, raw pointer handling, or unchecked slicing/indexing/casts/arithmetic may be introduced for fixture generation.

## Performance / Allocation Boundary

- Fixture code is cold-path, but oversize fixtures specifically enforce allocation-before-bound-check safety.
- Any large buffers used for valid boundary tests must be bounded by existing storage constants.

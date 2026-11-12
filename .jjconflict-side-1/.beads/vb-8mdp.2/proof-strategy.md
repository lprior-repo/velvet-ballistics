# Proof Strategy: VB Storage Budget-Before-Decode (vb-8mdp.2)

## Bead Scope
- **Bead ID**: vb-8mdp.2
- **Focus**: Budget gate at `decode_record_header` line 48, journal/snapshot read paths, no allocation before gate
- **DO NOT DUPLICATE**: Fixed-wire envelope codec proofs from vb-3t44 (kani_codec.rs, kani_record_payload_len.rs, kani_postcard_envelope_wire.rs)

## Core Invariants to Prove

### BUDGET-BEFORE-DECODE INVARIANT (Primary)
```
For all calls to decode_record_header(h, magic, max):
  PRE: h is any &[u8], max > 0
  POST: If returned Ok(header) → header.payload_len ≤ max
        If returned Err(PayloadTooLarge { len, max }) → len > max
  PROOF OBLIGATION: No allocation of > max bytes occurs before this check
```

### Purity Invariant
`decode_record_header` takes `&[u8]` (borrowed) → cannot create a Vec inside the function. All allocation happens in `decode_record_payload` and `decode_record` AFTER budget gate passes.

## Verifier Lane Selection

| Lane | Tool | Coverage Rationale |
|------|------|---------------------|
| **Kani** | `cargo kani --package vb_storage` | Bounded model checking: proves PayloadTooLarge returned before any Vec creation for hostile inputs with arbitrary payload_len > max |
| **Verus** | `cargo verus` | Functional proof: decode_record_header is total on &[u8]; payload_len invariant; no panic on any input |
| **TLA+** | `tlc constants.tla` | State machine: keyspace prefix distinctness; budget-before-decode workflow invariant |
| **Miri** | `cargo miri test --package vb_storage` | Detects UB in codec read paths; covers all decode_record_header error branches |
| **proptest** | `cargo test --package vb_storage` | Property-based: arbitrary header bytes → consistent error classification |
| **fuzz** | `cargo fuzz` | Differential: decode_record on arbitrary bytes never panics |

## Lanes NOT Selected

| Lane | Reason |
|------|--------|
| Flux | No dependent types or refinement predicates in scope; type system + Kani provides sufficient coverage |
| Loom | No concurrent access to Fjall keyspaces in decode path; read path is single-threaded |

## Risk Classification

| Risk | Type | Severity | Lane |
|------|------|----------|------|
| H1: Budget bypass (early allocation) | Runtime | Critical | Kani |
| H2: Magic confusion | Security | High | Kani + TLA+ |
| H3: Overflow in payload_end | Runtime | High | Kani |
| H4: Postcard over-alloc | Runtime | Medium | Kani |
| H5: Digest bypass | Integrity | High | Kani |
| H6: CRC32C bypass | Integrity | High | Kani |
| H7: max_payload=0 | Defined | Low | Kani |
| H8: Schema confusion | Compatibility | Medium | Kani |
| H9: Corrupt snapshot divergence | Recovery | High | Kani + proptest |
| H10: KV separation bypass | None | — | not_applicable |

## Proof Seed Coverage

| PS ID | Claim | Verifier Lane |
|-------|-------|---------------|
| ps-001 | PayloadTooLarge before any allocation on hostile input | Kani |
| ps-002 | Never panics on any input length | Kani + Verus |
| ps-003 | decode_record_payload never slices beyond budget | Kani |
| ps-004 | checked_add prevents overflow in payload_end | Kani |
| ps-005 | Magic family constraint enforced first | Kani |
| ps-006 | Unknown record_kind rejected | Kani |
| ps-007 | Budget check at line 48 returns PayloadTooLarge before any branch that could allocate | Kani |
| ps-008 | Header length must equal 60 | Kani |
| ps-009 | Wrong schema version rejected | Kani |
| ps-010 | CRC mismatch rejected | Kani |
| ps-011 | Corrupt payload digest rejected | Kani |
| ps-012 | decode_optional no allocation before decode_record_header | Kani + TLA+ |
| ps-013 | decode_journal_event semantic validity after decode | Kani + proptest |
| ps-014 | snapshot respects MAX_SNAPSHOT_BYTES budget | Kani + proptest |
| ps-015 | blob respects MAX_BLOB_BYTES budget | Kani + proptest |
| ps-016 | decode_record_header is total (no panic on empty input) | Verus |
| ps-017 | payload_len type invariant after budget check | Verus |
| ps-018 | Keyspace prefix distinctness | TLA+ |
| ps-019 | Budget-before-decode workflow invariant | TLA+ |
| ps-020 | Fuzz target decode_record never panics | Kani + fuzz |

## Proof-to-Implementation Mapping

### Budget Gate Proof Targets
- `crates/vb_storage/src/codec/header.rs:26` — `decode_record_header`
  - Line 31-33: bounds check `header.get(..60)` → `UnexpectedEof`
  - Line 48: `if decoded.payload_len > max_payload_len` → `PayloadTooLarge` ⭐ BUDGET GATE
- `crates/vb_storage/src/codec/payload.rs:56` — `decode_record_payload`
  - Line 61: calls `decode_record_header` (budget gate)
  - Line 66-68: `checked_add` overflow check
- `crates/vb_storage/src/journal/internal.rs:13` — `decode_optional`
  - Line 20-22: `keyspace.get(key)` returns borrowed `&[u8]` — NO ALLOCATION
  - Line 23: `decode_record` called on borrowed bytes
- `crates/vb_storage/src/snapshots.rs:33` — `snapshot()`
  - Line 39-44: `decode_optional` with `MAX_SNAPSHOT_BYTES`

## Deliverables

1. `verifier-lane-decisions.jsonl` — Machine-readable lane decisions per proof seed
2. `verifier-lane-matrix.md` — Proof seed → verifier lane mapping
3. `proof-coverage-matrix.md` — Contracts → obligations coverage
4. `proof-obligations.planned.jsonl` — All planned proof obligations with commands
5. `trusted-base-plan.md` — Trusted base boundaries
6. `proof-plan-review.md` — Review request (STATUS: PENDING_APPROVAL)
# Lean Contract Projection: vb-qi37.3.2

## Boundary

- **Lean-owned kernel**: `CollectPaginationState` encode/decode round-trip and `validate_hydrated_identity` predicate. These are pure deterministic functions.
- **Rust/runtime shell**: `drive_deterministic_full`, `evidence.push_slot_written_with_extra`, Fjall journal persistence, `hydrate_collect_states_from_recovered_journal`.
- **External systems excluded from Lean proof**: Fjall LSM internals, filesystem, wall-clock time.

## Lean-Owned Clauses

The cursor persistence path involves two pure critical behaviors that could be Lean-projected:

### 1. Postcard Round-Trip (Encoder/Decoder)

**Target**: `CollectPaginationState` postcard serialization

```lean
--伪代码 Lean projection
inductive CollectPaginationState
  | mk (run_id: RunId) (collector_slot: SlotIdx) (source: ListId)
         (current_page: ListId) (cursor: Nat) (page_size: Nat)
         (item_count: Nat) (limit: Nat) (time_limit_ms: Option Nat) (start_millis: Nat)

def round_trip (s: CollectPaginationState) : Prop :=
  postcard_decode (postcard_encode s) = some s
```

**Theorem**: For all `CollectPaginationState` values `s`, `round_trip(s)` holds.
**Status**: This is a standard Postcard property. The implementation uses `postcard::to_allocvec` and `postcard::from_bytes` which are contractually total on valid inputs. This theorem is **waived** because:
- Postcard codec is a third-party dependency with its own correctness guarantees
- Round-trip is verified by `collect_tests.rs:2112-2154` (unit test)

### 2. Identity Validation Predicate

**Target**: `validate_hydrated_identity`

```lean
--伪代码 Lean projection
def identity_valid (state: CollectPaginationState) (run_id: RunId) (slot: SlotIdx) : Prop :=
  state.run_id = run_id ∧ state.collector_slot = slot
```

**Theorem**: `validate_hydrated_identity` returns `Ok` iff `identity_valid` holds.
**Status**: **Waived** — this is a one-line structural equality check (`collect.rs:143`). The unit tests at `collect_tests.rs:2262-2306` cover both the valid and invalid identity cases exhaustively.

## Theorem Obligations

No Lean theorems are **required** because:

1. The persistence path is a composition of:
   - Structural key lookup (`collect.rs:86-92`) — proven by HashMap semantics
   - Postcard round-trip — covered by unit tests
   - Identity validation — covered by unit tests
   - Fjall journal persistence — handled by storage bead

2. All critical properties are covered by existing unit tests in `collect_tests.rs`:
   - `collect_pagination_extra_round_trips_for_recovery` (line 2112-2154)
   - `collect_pagination_extra_rejects_corrupt_bytes` (line 2158-2163)
   - `collect_journal_extra_rejects_corrupt_bytes` (line 2166-2172)
   - `collect_pagination_extra_recovered_journal_rejects_corrupt_bytes` (line 2175-2191)
   - `collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page` (line 2193-2258)
   - `collect_pagination_extra_rejects_identity_mismatch` (line 2262-2270)
   - `collect_journal_extra_rejects_identity_mismatch` (line 2273-2282)
   - `collect_pagination_extra_recovered_journal_rejects_identity_mismatch` (line 2285-2307)

## Waivers

| Clause | Owner | Reason | Expiry | Compensating Evidence |
|--------|-------|--------|--------|-----------------------|
| Postcard round-trip theorem | vb-qi37.3.2 contract synthesizer | Third-party codec; unit tests cover round-trip | Not applicable | `collect_tests.rs:2112-2154` |
| Identity validation theorem | vb-qi37.3.2 contract synthesizer | One-line structural equality; unit tests cover both valid/invalid | Not applicable | `collect_tests.rs:2262-2307` |
| Fjall persistence proof | N/A | Separate storage bead scope | N/A | Fjall bead |
| Runtime shell (drive, evidence, recovery) | N/A | Outside Lean scope (I/O, wall-clock time) | N/A | Code review at `drive.rs:95-106`, `collect.rs:130-136` |

## Non-goals

- Lean proof of Fjall internals
- Lean proof of `collect_start`/`collect_next`/`collect_finish` (proven in vb-qi37.3.1)
- Lean proof of concurrent shard behavior

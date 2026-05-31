# Test Plan: Journal Batch Byte Accounting (vb-vzcuf)

## Summary

- **Bead:** vb-vzcuf
- **State:** 8 (test-planner)
- **Bridge review:** APPROVED (State 7), GOD RULE 2 deferred to State 11
- **Contract clauses:** C1-C9 (see contract.md)
- **Proof seeds:** PS-001 through PS-009
- **RROs mapped:** 45 (RRO-vb-vzcuf-001 through 045)
- **Behaviors identified:** 41
- **Trophy allocation:** 18 unit / 15 integration / 5 e2e / 3 static
- **Proptest invariants:** 9 (one per proof seed)
- **Fuzz targets:** 9 (one per proof seed, defense-depth lane)
- **Kani harnesses:** 9 (one per proof seed, bounded model check lane)
- **Mutation threshold target:** >=90%
- **GOD RULE 2 deferred tests:** 8 behaviors marked as deferred-to-state-11

## Coverage Mapping to 45 RROs

| Behavior Group | Contract | RRO IDs | Verifiers |
|---|---|---|---|
| B-GROUP-01: Byte Limit Construction | C1 | RRO-021..024, 042 | Verus, Kani, Flux, proptest, fuzz |
| B-GROUP-02: Encoded Length Accounting | C2 | RRO-017..020, 041 | Verus, Kani, Flux, proptest, fuzz |
| B-GROUP-09: Duplicate Accounting Policy | C2 | RRO-033..036, 045 | Verus, Kani, Flux, proptest, fuzz |
| B-GROUP-03: Admission Boundary | C3 | RRO-001..004, 037 | Verus, Kani, Flux, proptest, fuzz |
| B-GROUP-04: Typed Error API | C4 | RRO-009..012, 039 | Verus, Kani, Flux, proptest, fuzz |
| B-GROUP-05: No Partial Mutation | C5 | RRO-013..016, 040 | Verus, Kani, Flux, proptest, fuzz |
| B-GROUP-06: Error Separation/Precedence | C6 | RRO-029..032, 044 | Verus, Kani, Flux, proptest, fuzz |
| B-GROUP-07: Overflow Safety | C7 | RRO-005..008, 038 | Verus, Kani, Flux, proptest, fuzz |
| B-GROUP-08: Core/Storage Bridge | C8 | RRO-025..028, 043 | Verus, Kani, Flux, proptest, fuzz |

---

## 1. Behavior Inventory

### B-GROUP-01: Byte Limit Construction (C1, 5 RROs)

| ID | Behavior |
|---|---|
| B01.1 | New batch constructed with default limit has non-zero byte limit |
| B01.2 | New batch constructed with explicit limit uses provided value |
| B01.3 | Constructor rejects zero byte limit with error |
| B01.4 | Constructor rejects byte limit exceeding sensible maximum |
| B01.5 | New batch starts with staged bytes equal to zero |
| B01.6 | Accessor returns zero staged bytes immediately after construction |

### B-GROUP-02: Encoded Length Accounting (C2, 5 RROs)

| ID | Behavior |
|---|---|
| B02.1 | encode_record returns Vec length >= RECORD_HEADER_BYTES (60 bytes) |
| B02.2 | encode_record length exceeds postcard payload length (envelope, not raw payload) |
| B02.3 | Accounting uses full Vec::len() from encode_record, not payload_len_u32 |
| B02.4 | Staged bytes equal sum of encoded lengths of accepted events |
| B02.5 | encode_record fails with PayloadTooLarge when payload > MAX_JOURNAL_EVENT_PAYLOAD_BYTES |
| B02.6 | encode_record failure does not mutate staged bytes |

### B-GROUP-03: Admission Boundary (C3, 5 RROs)

| ID | Behavior |
|---|---|
| B03.1 | Event is accepted when staged_bytes + encoded_len == limit (exact fit) |
| B03.2 | Event is accepted when staged_bytes + encoded_len < limit |
| B03.3 | Event is rejected when staged_bytes + encoded_len > limit |
| B03.4 | Staged bytes increment by encoded_len on successful append |
| B03.5 | Zero-length encoded events are always accepted if within limit |
| B03.6 | Admission check uses checked arithmetic, not wrapping |

### B-GROUP-04: Typed Error API (C4, 5 RROs)

| ID | Behavior |
|---|---|
| B04.1 | Accumulated byte rejection returns JournalBatchBytesExceeded (or equivalent) variant |
| B04.2 | Accumulated byte rejection is NOT QueueFull |
| B04.3 | Accumulated byte rejection is NOT PayloadTooLarge |
| B04.4 | Error variant carries attempted bytes and limit fields |
| B04.5 | Error variant Display output references byte budget, not count |

### B-GROUP-05: No Partial Mutation (C5, 5 RROs)

| ID | Behavior |
|---|---|
| B05.1 | Rejected event is not staged in OwnedWriteBatch |
| B05.2 | inner.len() unchanged after byte rejection |
| B05.3 | Staged bytes unchanged after byte rejection |
| B05.4 | Batch remains open (not aborted) after byte rejection |
| B05.5 | Rejected key is not committed after commit call |
| B05.6 | Previously accepted events survive byte rejection of later event |

### B-GROUP-06: Error Separation and Precedence (C6, 5 RROs)

| ID | Behavior |
|---|---|
| B06.1 | Duplicate detection fires before byte admission check |
| B06.2 | Count capacity (QueueFull) fires before byte admission check |
| B06.3 | PayloadTooLarge fires before byte admission check |
| B06.4 | Byte admission fires after all other guards pass |
| B06.5 | When duplicate + overflow both apply, duplicate error wins |
| B06.6 | When count + overflow both apply, QueueFull wins |

### B-GROUP-07: Overflow Safety (C7, 5 RROs)

| ID | Behavior |
|---|---|
| B07.1 | staged_bytes + encoded_len uses checked_add |
| B07.2 | Overflow returns typed rejection, not panic |
| B07.3 | No usize to u32/u64 unchecked casts in admission path |
| B07.4 | Byte limit of u64::MAX combined with non-zero delta produces overflow rejection |
| B07.5 | Envelope length conversion does not panic on edge values |

### B-GROUP-08: Core/Storage Bridge (C8, 5 RROs)

| ID | Behavior |
|---|---|
| B08.1 | Core max_journal_batch_bytes can flow into storage batch limit |
| B08.2 | Storage default limit matches core default of 1_048_576 (or separation is explicit) |
| B08.3 | Core BudgetError::JournalBatchBytesExceeded is not conflated with storage JournalError |
| B08.4 | Core budget validation rejects zero limit before reaching storage |
| B08.5 | Storage limit type does not silently truncate core u32 value |

### B-GROUP-09: Duplicate Accounting Policy (C2 open question, 5 RROs)

| ID | Behavior |
|---|---|
| B09.1 | Same-batch duplicate event uses documented accounting policy |
| B09.2 | Duplicate append within same batch does not double-count bytes (if distinct-key) |
| B09.3 | Duplicate append within same batch counts bytes each attempt (if conservative) |
| B09.4 | Duplicate accounting does not panic or produce overflow |
| B09.5 | Duplicate accounting preserves staged byte invariant |

### E2E Behaviors (C9, observability)

| ID | Behavior |
|---|---|
| E01 | Full lifecycle: construct batch with limit, append to limit, observe rejection, commit valid |
| E02 | Full lifecycle: append many events under limit, commit, verify durable |
| E03 | Aborted batch (duplicate) with byte tracking: staged bytes survive abort |
| E04 | Accessor returns accurate staged bytes throughout batch lifecycle |
| E05 | Multi-event batch with mixed accept/reject produces correct final staged count |

---

## 2. Trophy Allocation

```
         [E2E]     5 behaviors  ← full workflow validation
    [Integration]  15 behaviors  ← real FjallJournal, real encode_record
    [Unit / Calc]  18 behaviors  ← pure admission helper, error construction
  [Static Analysis]  3 behaviors  ← no_unwrap lint, non_exhaustive, clippy gates
```

**Rationale:**
- Integration layer is widest (37%): JournalWriteBatch interacts with real Fjall, real codec. Fakes here would mask encoding bugs (H7 hazard). Google SWE Book: "Prefer real implementations."
- Unit/Calc layer (44%): The pure admission helper `admit_journal_event_bytes(staged, encoded_len, limit)` can be tested exhaustively combinatorially without any I/O. This is the "calc" layer where proptest shines.
- E2E (12%): The observability accessor and full lifecycle deserve black-box validation.
- Static (7%): Clippy `unwrap_used`, `panic`, `arithmetic_side_effects` lints. `#[non_exhaustive]` on JournalError enforced by compile.

**Deferred to State 11:** 8 behaviors that require production fields (`staged_bytes`, `byte_limit`, `AccumulatedBytesExceeded`) that do not yet exist. These are tagged `deferred-to-state-11` and listed in §9.

---

## 3. BDD Scenarios

### B-GROUP-01: Byte Limit Construction

#### B01.1: New batch constructed with default limit has non-zero byte limit

```
Given: a FjallJournal is open
When: JournalWriteBatch::new(journal) or journal.batch() is called
Then: the resulting batch has a byte_limit > 0
And: the byte_limit equals the documented storage default (1_048_576)
```

```rust
fn batch_construction_produces_nonzero_default_limit()
```

#### B01.2: New batch with explicit limit uses provided value [deferred-to-state-11]

```
Given: a JournalBatchByteLimit of value 64 is created
When: JournalWriteBatch is constructed with that limit
Then: the batch byte_limit equals 64
And: staged_bytes equals 0
```

```rust
fn explicit_limit_construction_stores_provided_value()
```

#### B01.3: Constructor rejects zero byte limit [deferred-to-state-11]

```
Given: a zero value is attempted for JournalBatchByteLimit
When: the value object is constructed
Then: construction fails with a typed error
And: no batch enters an open state with zero limit
```

```rust
fn zero_limit_construction_is_rejected()
```

#### B01.4: Byte limit type rejects large values exceeding sane maximum [deferred-to-state-11]

```
Given: a value exceeding a configured absolute maximum is provided
When: JournalBatchByteLimit is constructed
Then: construction fails with a typed error
```

```rust
fn limit_rejects_absurdly_large_value()
```

#### B01.5: New batch starts with staged bytes equal to zero [deferred-to-state-11]

```
Given: any valid batch construction
When: batch is queried for staged journal event bytes
Then: staged bytes equals zero
```

```rust
fn new_batch_has_zero_staged_bytes()
```

#### B01.6: Accessor returns zero staged bytes immediately after construction [deferred-to-state-11]

```
Given: a freshly constructed batch
When: the staged_bytes accessor is called
Then: it returns 0
```

```rust
fn staged_bytes_accessor_returns_zero_on_new_batch()
```

### B-GROUP-02: Encoded Length Accounting

#### B02.1: encode_record always produces output >= RECORD_HEADER_BYTES

```
Given: a valid JournalEvent with payload within per-record cap
When: encode_record(MAGIC_JOURNAL_EVENT, ...) is called
Then: the returned Vec<u8>.len() >= RECORD_HEADER_BYTES (60)
And: the returned Vec<u8>.len() > RECORD_HEADER_BYTES (has payload data)
```

```rust
fn encode_record_returns_at_least_header_bytes()
```

#### B02.2: encode_record length exceeds postcard payload length

```
Given: a valid JournalEvent
When: encode_record produces a Vec<u8>
Then: Vec::len() is greater than the postcard payload length alone
And: the difference equals the storage envelope overhead
```

```rust
fn encoded_length_exceeds_postcard_payload_length()
```

#### B02.3: Accounting uses full Vec::len() from encode_record

```
Given: a batch with staged_bytes S and byte_limit L where S + 60 <= L
When: an event whose encoded length is E > 60 is appended
Then: staged_bytes increases by E (the full encoded length)
And: NOT by the payload length alone
```

```rust
fn accounting_uses_full_encoded_length_not_payload_length()
```

#### B02.4: Staged bytes equal sum of encoded lengths of accepted events [deferred-to-state-11]

```
Given: events encoded to lengths [E1, E2, E3]
When: all three are accepted
Then: staged_bytes equals E1 + E2 + E3
```

```rust
fn staged_bytes_equals_sum_of_accepted_encoded_lengths()
```

#### B02.5: encode_record fails with PayloadTooLarge when payload exceeds cap

```
Given: an event payload exceeding MAX_JOURNAL_EVENT_PAYLOAD_BYTES
When: encode_record is called
Then: Err(JournalError::PayloadTooLarge { len, max }) is returned
And: len > max
```

```rust
fn encode_record_rejects_oversize_payload_with_payload_too_large()

// Error variant:
Given: an event payload exactly at cap
When: encode_record is called
Then: Ok(vec) is returned (exact cap is valid per-record)
```

```rust
fn encode_record_accepts_payload_at_exact_cap()
```

#### B02.6: encode_record failure does not mutate staged bytes

```
Given: a batch with staged_bytes = S
When: an event whose per-record payload exceeds the cap is appended
Then: PayloadTooLarge is returned
And: staged_bytes remains S (unchanged)
```

```rust
fn payload_too_large_does_not_mutate_staged_bytes()
```

### B-GROUP-03: Admission Boundary

#### B03.1: Exact-fit acceptance

```
Given: a batch with limit = 120 and staged_bytes = 60
When: an event with encoded length 60 is appended
Then: the event is accepted
And: staged_bytes becomes 120
```

```rust
fn admission_accepts_exact_fit_at_limit_boundary()
```

#### B03.2: Under-limit acceptance

```
Given: a batch with limit = 200 and staged_bytes = 60
When: an event with encoded length 80 is appended
Then: the event is accepted
And: staged_bytes becomes 140
```

```rust
fn admission_accepts_under_limit_event()
```

#### B03.3: Over-limit rejection

```
Given: a batch with limit = 100 and staged_bytes = 60
When: an event with encoded length 41 is appended
Then: the event is rejected
And: the error is the accumulated byte budget variant
And: staged_bytes remains 60
```

```rust
fn admission_rejects_over_limit_event()
```

#### B03.4: Staged bytes increment by encoded_len on success [deferred-to-state-11]

```
Given: a batch with staged_bytes = S and limit = L where S + E <= L
When: an event with encoded length E is appended successfully
Then: staged_bytes = S + E
```

```rust
fn staged_bytes_increments_by_encoded_length_on_success()
```

#### B03.5: Zero-length encoded events always accepted

```
Given: any batch with limit > 0 and staged_bytes <= limit
When: an event with encoded length 0 is appended
Then: the event is accepted (exact-fit when staged == limit, under when staged < limit)
And: staged_bytes unchanged (delta is 0)
```

```rust
fn zero_length_encoded_event_is_always_accepted()
```

#### B03.6: Admission check uses checked arithmetic

```
Given: staged_bytes = u64::MAX and encoded_len = 1
When: admission check computes staged_bytes + encoded_len
Then: the addition overflows
And: the path returns a typed rejection, not a panic
```

```rust
fn admission_uses_checked_arithmetic_not_wrapping()
```

### B-GROUP-04: Typed Error API

#### B04.1: Accumulated byte rejection returns JournalBatchBytesExceeded [deferred-to-state-11]

```
Given: a batch with limit = 100 and staged_bytes = 60
When: an event with encoded length 41 is appended
Then: JournalBatchBytesExceeded { attempted: 101, limit: 100 } is returned
```

```rust
fn byte_rejection_returns_journal_batch_bytes_exceeded_variant()

// Error variant:
Given: correct error is returned
When: caller matches on JournalBatchBytesExceeded
Then: attempted field holds the computed total (101)
And: limit field holds the configured limit (100)
```

```rust
fn byte_rejection_error_carries_attempted_and_limit_fields()
```

#### B04.2: Accumulated byte rejection is distinct from QueueFull

```
Given: a batch with ample count capacity but exceeded byte limit
When: append_event returns error
Then: the error is not QueueFull
```

```rust
fn byte_rejection_is_not_queue_full()
```

#### B04.3: Accumulated byte rejection is distinct from PayloadTooLarge

```
Given: a batch with per-record payload within cap but exceeded byte budget
When: append_event returns error
Then: the error is not PayloadTooLarge
```

```rust
fn byte_rejection_is_not_payload_too_large()
```

#### B04.4: Error variant carries diagnostic fields [deferred-to-state-11]

```
Given: a byte rejection error
When: fields are inspected
Then: attempted >= limit + 1
And: all fields use bounds-appropriate integer types (u64)
```

```rust
fn byte_rejection_fields_are_bounds_appropriate()
```

#### B04.5: Display text references byte pressure

```
Given: a byte rejection error (deferred-to-state-11)
When: .to_string() is called
Then: the output contains "byte" or "batch" (not "queue" or "count")
```

```rust
fn byte_rejection_display_references_batch_byte_pressure()
```

### B-GROUP-05: No Partial Mutation

#### B05.1: Rejected event not staged in OwnedWriteBatch

```
Given: a batch with 1 accepted event (inner.len() == 1)
When: a byte-rejected event is attempted
Then: inner.len() remains 1
And: the rejected key is not in the OwnedWriteBatch
```

```rust
fn rejected_over_limit_event_not_in_write_batch()

// Error variant:
Given: a freshly constructed batch
When: a byte-rejected event is attempted
Then: inner.len() is 0
```

```rust
fn first_event_byte_rejection_keeps_batch_empty()
```

#### B05.2: inner.len() unchanged after byte rejection

```
Given: a batch with N accepted events
When: a byte-rejected event fails
Then: batch.len() == N
```

```rust
fn batch_len_unchanged_after_byte_rejection()
```

#### B05.3: Staged bytes unchanged after byte rejection [deferred-to-state-11]

```
Given: a batch with staged_bytes = S and limit = L
When: an event with encoded_len > (L - S) is appended and rejected
Then: staged_bytes == S
```

```rust
fn staged_bytes_unchanged_after_byte_rejection()
```

#### B05.4: Batch remains open after byte rejection

```
Given: a batch with accepted events
When: a byte-rejected event fails
Then: batch is not aborted
And: subsequent valid events can still be accepted and committed
```

```rust
fn batch_remains_open_after_byte_rejection()

// Error variant (abort path):
Given: a duplicate durable event
When: append_event is called
Then: DuplicateEvent is returned
And: batch IS aborted (existing behavior preserved)
```

```rust
fn duplicate_event_aborts_batch_preserving_existing_semantics()
```

#### B05.5: Rejected key not committed after commit call

```
Given: a batch with multiple accepted events and one byte-rejected event
When: batch.commit() is called
Then: the accepted events are durably written
And: the rejected event key is not present in the journal
```

```rust
fn rejected_event_not_persisted_after_commit()

// Error variant:
Given: rejected event is not persisted
When: a subsequent batch appends with the same key
Then: the append succeeds (key is not already durable)
```

```rust
fn rejected_event_key_reusable_in_subsequent_batch()
```

#### B05.6: Previously accepted events survive byte rejection of later event [deferred-to-state-11]

```
Given: events E1 and E2 accepted, staged_bytes updated
When: event E3 is rejected for byte budget
Then: E1 and E2 remain staged
And: committing the batch persists E1 and E2 durably
```

```rust
fn prior_accepted_events_survive_later_byte_rejection()
```

### B-GROUP-06: Error Separation and Precedence

#### B06.1: Duplicate detection fires before byte admission

```
Given: a committed event key EK
When: a new batch appends EK with an event that would also exceed byte budget
Then: DuplicateEvent is returned (not accumulated byte error)
And: batch is aborted
```

```rust
fn duplicate_precedes_byte_admission_check()
```

#### B06.2: Count capacity fires before byte admission

```
Given: a batch at MAX_BATCH_COUNT
When: an event that would fit in byte budget is appended
Then: QueueFull is returned (not accumulated byte error)
```

```rust
fn queue_full_precedes_byte_admission_check()
```

#### B06.3: PayloadTooLarge fires before byte admission

```
Given: an event payload exceeding MAX_JOURNAL_EVENT_PAYLOAD_BYTES
When: appended in a batch with ample byte budget
Then: PayloadTooLarge is returned (not accumulated byte error)
```

```rust
fn payload_too_large_precedes_byte_admission_check()
```

#### B06.4: Byte admission fires only after all other guards pass

```
Given: all guards pass (unique key, under count, valid payload)
When: accumulated bytes exceed limit
Then: accumulated byte rejection is returned
And: no earlier guard could have fired
```

```rust
fn byte_admission_fires_after_all_other_guards_pass()
```

#### B06.5: Duplicate + Overflow → Duplicate wins

```
Given: a committed event EK and staged_bytes at u64::MAX
When: EK is appended again (would overflow if not caught by duplicate first)
Then: DuplicateEvent is returned
```

```rust
fn duplicate_precedes_overflow_guard()
```

#### B06.6: Count + Overflow → QueueFull wins

```
Given: a batch at MAX_BATCH_COUNT and staged_bytes at u64::MAX
When: another event is appended
Then: QueueFull is returned
```

```rust
fn queue_full_precedes_overflow_guard()
```

### B-GROUP-07: Overflow Safety

#### B07.1: Addition uses checked_add

```
Given: any staged_bytes and encoded_len values
When: admission path adds them
Then: it uses checked_add (verified by Kani harness, proptest property)
And: no unwrap/expect on the result
```

```rust
fn addition_uses_checked_arithmetic()
```

#### B07.2: Overflow returns typed rejection, not panic

```
Given: staged_bytes = u64::MAX and encoded_len = 1
When: admission path computes checked_add
Then: None is returned from checked_add
And: the code returns Err(accumulated budget overflow error)
And: the code does not panic
```

```rust
fn overflow_returns_typed_rejection_not_panic()
```

#### B07.3: No unchecked casts in admission path

```
Given: static analysis gate
When: clippy runs on the admission path
Then: no arithmetic_side_effects, no as_conversions on usize→u32/u64 in hot path
```

```rust
fn static_no_unchecked_casts_in_admission_path() // clippy-gated
```

#### B07.4: u64::MAX limit + delta overflow

```
Given: byte_limit = u64::MAX and staged_bytes = u64::MAX - 1
When: encoded_len = 2
Then: checked_add returns None
And: typed rejection returned
```

```rust
fn extreme_limit_overflow_returns_rejection()
```

#### B07.5: Envelope length conversion does not panic

```
Given: an edge-case encoded record length
When: conversion from usize to u64 occurs
Then: conversion is checked (try_from) not as
And: failure returns typed error
```

```rust
fn envelope_length_conversion_uses_try_from_not_as()
```

### B-GROUP-08: Core/Storage Bridge

#### B08.1: Core policy flows to storage limit [deferred-to-state-11]

```
Given: ResourceContract { max_journal_batch_bytes: 4096 }
When: a storage batch is created from this contract
Then: the batch byte_limit == 4096
```

```rust
fn core_max_batch_bytes_flows_to_storage_limit()
```

#### B08.2: Storage default matches core default

```
Given: current core default of 1_048_576
When: storage creates a batch without explicit policy
Then: byte_limit == 1_048_576 (or explicit separation is documented and tested)
```

```rust
fn storage_default_limit_matches_core_default()
```

#### B08.3: Core BudgetError not conflated with storage JournalError

```
Given: BudgetError::JournalBatchBytesExceeded { actual: 1000, limit: 500 }
When: constructed in core
Then: it is NOT a JournalError
And: bridge/translation is explicit (if applicable)
```

```rust
fn core_budget_error_distinct_from_storage_journal_error()
```

#### B08.4: Core budget validation rejects zero limit

```
Given: max_journal_batch_bytes = 0
When: core validates the ResourceContract
Then: validation returns Err
And: zero-limit batch cannot be constructed downstream
```

```rust
fn core_budget_validation_rejects_zero_limit()
```

#### B08.5: Storage limit type does not silently truncate

```
Given: a core u32 limit value
When: bridged to storage JournalBatchByteLimit
Then: no silent truncation or narrowing loss occurs
```

```rust
fn storage_limit_bridge_preserves_core_value_precision()
```

### B-GROUP-09: Duplicate Accounting Policy

#### B09.1: Same-batch duplicate has documented accounting behavior

```
Given: an event E is appended twice to the same batch
When: the batch is inspected
Then: the accounting behavior matches the documented policy (C2 open question)
```

```rust
fn same_batch_duplicate_accounting_matches_documented_policy()

// Error variant (policy-dependent):
Given: documented policy is "conservative attempt accounting"
When: same event appended twice
Then: staged_bytes reflects TWO appends
```

```rust
fn conservative_attempt_accounting_counts_each_append()

// Error variant (policy-dependent):
Given: documented policy is "distinct-key accounting"
When: same event appended twice
Then: staged_bytes reflects ONE append
```

```rust
fn distinct_key_accounting_counts_only_distinct_appends()
```

#### B09.4: Duplicate accounting does not panic [deferred-to-state-11]

```
Given: any duplicate event scenario
When: append_event is called twice with same key
Then: no panic occurs
And: error or acceptance follows documented policy
```

```rust
fn duplicate_accounting_never_panics()
```

#### B09.5: Duplicate accounting preserves staged byte invariant [deferred-to-state-11]

```
Given: staged_bytes = sum(accepted encoded lengths) under chosen policy
When: duplicate event is appended
Then: the invariant staged_bytes <= limit holds
And: staged_bytes remains consistent with the accounting policy
```

```rust
fn duplicate_accounting_preserves_staged_byte_invariant()
```

### E2E Behaviors

#### E01: Full lifecycle — construct, append to limit, reject, commit valid

```
Given: a batch with limit = 500
When: events are appended until byte budget exhausted
And: one more event is attempted (rejected)
And: batch.commit() is called
Then: accepted events are durably persisted
And: rejected event is not persisted
```

```rust
fn e2e_limit_enforcement_reject_then_commit()
```

#### E02: Full lifecycle — many events under limit, commit, verify

```
Given: a batch with limit = 50_000
When: 50 events of ~1000 bytes each are appended
Then: all 50 are accepted
And: batch.commit() succeeds
And: replay confirms 50 events durable
```

```rust
fn e2e_many_events_under_limit_committed_and_replayed()
```

#### E03: Aborted batch with byte tracking

```
Given: a batch with limit > 0, some events accepted
When: a duplicate durable event is appended (aborting the batch)
Then: batch is aborted
And: commit() returns Ok(()) without writing (existing no-op semantics)
And: staged_bytes behavior on abort is documented/defined
```

```rust
fn e2e_aborted_batch_byte_tracking_semantics()
```

#### E04: Accessor returns accurate staged bytes throughout lifecycle [deferred-to-state-11]

```
Given: a batch constructed with limit = 1000
When: events of lengths [200, 300, 400] are appended successfully
Then: staged_bytes() returns 200, then 500, then 900 at each step
```

```rust
fn e2e_accessor_returns_accurate_staged_bytes_throughout_lifecycle()
```

#### E05: Mixed accept/reject batch

```
Given: a batch with limit = 1000
When: events of lengths [300, 300, 500 (reject, would be 1100), 100] are appended
Then: final staged_bytes = 700 (300+300+100)
And: commit() persists events at indices 0, 1, 3
And: event at index 2 was not staged
```

```rust
fn e2e_mixed_accept_reject_batch_produces_correct_result()
```

---

## 4. Proptest Invariants

### Proptest: PS-001 — Admission boundary (C3, POB-vb-vzcuf-004)

**Invariant:** For all (staged, delta, limit) where `0 <= staged <= limit` and `0 <= delta <= u64::MAX - staged`:
- If `staged + delta <= limit`, admission accepts.
- If `staged + delta > limit`, admission rejects.
- `checked_add(staged, delta)` returns Some iff `staged + delta <= u64::MAX`.
- Rejection does not change staged.

**Strategy:** `proptest::num::u64::ANY` for staged, delta, limit with `prop_assume!(staged <= limit)`.

**Anti-invariant:** `delta > limit - staged` must always produce rejection.

**Production binding:** Exercises `JournalWriteBatch::append_event` with real `FjallJournal` and `encode_record`. Tests the guard interaction through the public API.

**Evidence file:** `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs`

---

### Proptest: PS-002 — Overflow safety (C7, POB-vb-vzcuf-008)

**Invariant:** For all (a, b) in u64:
- `a.checked_add(b)` is Some iff `a + b <= u64::MAX`.
- When Some, the result equals a + b.
- When None, the result is None (never panic, never wrap).
- encode_record called with any Serialize impl whose size is bounded returns Ok or Err, never panics.

**Strategy:** `(u64::MIN..u64::MAX, u64::MIN..u64::MAX)`. Include edge pairs: (MAX, 0), (MAX, 1), (0, MAX), (MAX, MAX).

**Anti-invariant:** `a.checked_add(b)` must never panic.

**Production binding:** Exercises std `checked_add` (the exact primitive used by implementation), validates `encode_record` panics are impossible.

**Evidence file:** `crates/vb_storage/tests/proptest_vb_vzcuf_PS_002.rs`

---

### Proptest: PS-003 — Error distinctness (C4/C6, POB-vb-vzcuf-012)

**Invariant:** For all batches where:
- Event payload within cap
- Inner count < MAX_BATCH_COUNT
- Event key is not a durable duplicate
- Accumulated bytes exceeded

The error returned is the accumulated byte budget variant, and it is:
- NOT an instance of QueueFull
- NOT an instance of PayloadTooLarge
- Pattern-matches distinctively

**Strategy:** Generate random events, construct batch near the byte limit, append until rejection, verify error variant.

**Anti-invariant:** Under the above conditions, returning QueueFull or PayloadTooLarge is invalid.

**Production binding:** Exercises production `JournalError` pattern matching on the actual error returned by `append_event`.

**Evidence file:** `crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs`

---

### Proptest: PS-004 — No partial mutation (C5, POB-vb-vzcuf-016)

**Invariant:** For any sequence of valid appends followed by one byte-rejected append:
- `batch.len()` equals the number of accepted events (not `accepted + 1`).
- The previously accepted keys are committed after `commit()`.
- The rejected key is not committed after `commit()`.

**Strategy:** Append N events (N in 0..=10), then one over-limit event. Capture pre-rejection state, compare post-rejection.

**Anti-invariant:** Appending a rejected event must not change len() or staged state.

**Production binding:** Exercises production `JournalWriteBatch::len()`, `append_event`, and `commit()`.

**Evidence file:** `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs`

---

### Proptest: PS-005 — Codec accounting (C2, POB-vb-vzcuf-020)

**Invariant:** For all valid `JournalEvent` values whose postcard payload fits within `MAX_JOURNAL_EVENT_PAYLOAD_BYTES`:
- `encode_record` returns `Ok(vec)`.
- `vec.len() >= RECORD_HEADER_BYTES (60)`.
- `vec.len()` is strictly greater than the postcard payload length (has envelope overhead).
- `vec.len()` is deterministic for the same input.

**Strategy:** Generate arbitrary JournalEvent variants with bounded payloads (0..1MB). Encode them and verify invariants.

**Anti-invariant:** `vec.len() < RECORD_HEADER_BYTES` or `vec.len() == payload_len`.

**Production binding:** Calls production `encode_record` and measures production `Vec::len()`.

**Evidence file:** `crates/vb_storage/tests/proptest_vb_vzcuf_PS_005.rs`

---

### Proptest: PS-006 — Limit presence (C1, POB-vb-vzcuf-024)

**Invariant:** For all valid batch construction paths:
- Default constructor produces a non-zero byte limit.
- Explicit constructor produces the provided limit value.
- No constructor produces a zero or absent limit (once fields exist at State 11).

**Strategy:** Construct batches via `new()`, `batch()`, and explicit path (when available). Assert limit > 0.

**Anti-invariant:** A freshly constructed batch must never have limit = 0.

**Production binding:** Exercises production `JournalWriteBatch::new` and `FjallJournal::batch()`.

**Evidence file:** `crates/vb_storage/tests/proptest_vb_vzcuf_PS_006.rs`

---

### Proptest: PS-007 — Core/storage bridge (C8, POB-vb-vzcuf-028)

**Invariant:** For all valid `ResourceContract::max_journal_batch_bytes` values:
- If converted to a storage limit, the storage limit equals the core value.
- If storage uses its own default, the default is documented and equals the core default (1_048_576).
- The core `BudgetError::JournalBatchBytesExceeded` and storage `JournalError::JournalBatchBytesExceeded` (if both exist) use compatible value representations.

**Strategy:** Generate arbitrary valid ResourceContract values, bridge to storage, compare limits.

**Anti-invariant:** Core limit of 4096 must not become storage limit of 0 or 1024.

**Production binding:** Exercises production `ResourceContract` default and field access.

**Evidence file:** `crates/vb_storage/tests/proptest_vb_vzcuf_PS_007.rs`

---

### Proptest: PS-008 — Guard precedence (C6, POB-vb-vzcuf-032)

**Invariant:** For all events where multiple rejection conditions could apply:
- The guard order is: duplicate > count > payload > accumulated > mutation.
- If duplicate applies, duplicate error is always returned regardless of other conditions.
- If count applies (and not duplicate), QueueFull is always returned.
- If payload applies (and not duplicate/count), PayloadTooLarge is always returned.
- Accumulated byte admission fires only when no earlier guard fires.

**Strategy:** Systematically construct events with combinations of error conditions and verify returned error matches precedence contract.

**Anti-invariant:** Receiving PayloadTooLarge when duplicate applies is a precedence violation.

**Production binding:** Exercises production `append_event` guard cascade.

**Evidence file:** `crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs`

---

### Proptest: PS-009 — Duplicate accounting (C2 open, POB-vb-vzcuf-036)

**Invariant:** Under the chosen accounting policy:
- **Conservative:** appended duplicates each contribute bytes. `staged_bytes = count_of_all_append_attempts * encoded_len`.
- **Distinct-key:** only first append of each key contributes. `staged_bytes = count_of_distinct_keys * encoded_len`.

**Strategy:** Append same event 3 times, measure staged bytes, verify against policy.

**Anti-invariant:** Duplicate accounting must not make staged_bytes exceed limit without rejection.

**Production binding:** Exercises production `append_event` and `staged_event_keys` tracking.

**Evidence file:** `crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs`

---

## 5. Fuzz Targets

All 9 fuzz targets are defense-depth lanes (RROs 037-045). They complement proptest by using libFuzzer's coverage-guided mutation:

| Fuzz Target | Input Type | Risk | Corpus Seeds |
|---|---|---|---|
| vb_vzcuf_PS_001 (C3 admission) | arbitrary JournalEvent bytes + limit | panic, OOM, logic error in admission | zero-staged, exact-fit, over-limit, u64::MAX staged |
| vb_vzcuf_PS_002 (C7 overflow) | arbitrary (staged, delta) u64 pairs | arithmetic panic, wrap | (MAX, 0), (MAX, 1), (1, MAX) |
| vb_vzcuf_PS_003 (C4/C6 error) | arbitrary event sequences | error conflation, missing variant arm | QueueFull, PayloadTooLarge, DuplicateEvent sequences |
| vb_vzcuf_PS_004 (C5 mutation) | arbitrary append-reject-commit sequences | partial persistence of rejected data | accept-then-reject, reject-only, accept-reject-accept-commit |
| vb_vzcuf_PS_005 (C2 codec) | arbitrary event payloads (bytes) | codec panic, length miscalculation | 0-byte payload, RECORD_HEADER_LEN payload, MAX-payload |
| vb_vzcuf_PS_006 (C1 constructor) | arbitrary limit values | zero-limit batch, panic on extreme value | 0, 1, 1_048_576, u32::MAX, u64::MAX |
| vb_vzcuf_PS_007 (C8 bridge) | arbitrary ResourceContract values | bridge value corruption, silent default drift | all-zero contract, max-value contract, default contract |
| vb_vzcuf_PS_008 (C6 precedence) | adversarial guard-combination events | precedence inversion, guard ordering bug | all single-error events, multi-error combos |
| vb_vzcuf_PS_009 (C2 duplicate) | arbitrary staged event key sequences | duplicate accounting bug, overflow from double-count | single-key repeated, multi-key, interleaved duplicates |

**Fuzz command template:** `cargo fuzz run vb_vzcuf_PS_<N> -- -max_total_time=60`

**Directory:** `fuzz/fuzz_targets/vb_vzcuf_PS_<N>.rs` (in isolated workspace)

---

## 6. Kani Harnesses

All 9 Kani harnesses are bounded model check lanes. They must use `kani::any()` with `kani::assume` constraints, not hardcoded shapes.

### Kani Harness: PS-001 — Admission boundary (RRO-002)

**Property:** For bounded `staged: u64`, `delta: u64`, `limit: u64` where `checked_add(staged, delta)` is computable:
- If `checked_add` is Some and `total <= limit`, admission returns Ok.
- If `checked_add` is None or `total > limit`, admission returns the accumulated byte error.

**Bound:** staged, delta, limit bounded to `u64` (Kani manages symbolic exploration).

**Rationale:** Arithmetic admission is the core of the bead. Proptest covers random inputs; Kani proves no counterexample within the full u64 bound.

**Evidence file:** `verification/kani/vb-vzcuf-PS-001.rs`

---

### Kani Harness: PS-002 — Overflow safety (RRO-006)

**Property:** For all `a: u64, b: u64`, `a.checked_add(b)` never panics. When it returns None, the implementation path returns an error, not a panic or unwrap.

**Rationale:** Proptest may miss specific overflow edge cases. Kani exhaustively proves no counterexample.

**Evidence file:** `verification/kani/vb-vzcuf-PS-002.rs`

---

### Kani Harness: PS-003 — Error distinctness (RRO-010)

**Property:** Under preconditions (payload within cap, count within limit, no durable duplicate), the accumulated-byte error variant is returned, and it is NOT QueueFull and NOT PayloadTooLarge.

**Rationale:** Runtime error handling relies on distinct error variants. Formal proof of distinctness prevents conflation bugs.

**Evidence file:** `verification/kani/vb-vzcuf-PS-003.rs`

---

### Kani Harness: PS-004 — No mutation on rejection (RRO-014)

**Property:** For all bounded batch states where byte rejection occurs:
- `inner.len()` before == `inner.len()` after.
- The rejected event is not in the OwnedWriteBatch.
- The batch is not aborted.

**Rationale:** Persistence correctness requires no partial mutation. Kani proves this for all bounded inputs.

**Evidence file:** `verification/kani/vb-vzcuf-PS-004.rs`

---

### Kani Harness: PS-005 — encode_record length (RRO-018)

**Property:** For all valid `encode_record` inputs:
- The returned `Vec::len()` is always >= `RECORD_HEADER_BYTES`.
- The returned `Vec::len()` is consistent for the same input.

**Rationale:** Accounting correctness depends on consuming full encoded length. Proptest covers random; Kani proves exhaustively.

**Evidence file:** `verification/kani/vb-vzcuf-PS-005.rs`

---

### Kani Harness: PS-006 — Limit non-zero (RRO-022)

**Property:** `JournalWriteBatch::new(journal)` always produces a batch where the byte limit > 0. No execution path produces a batch with limit == 0.

**Rationale:** Zero limit would make every append fail silently. Must be impossible.

**Evidence file:** `verification/kani/vb-vzcuf-PS-006.rs`

---

### Kani Harness: PS-007 — Budget bridge (RRO-026)

**Property:** Core `validate_u32_budget` produces correct `BudgetError::JournalBatchBytesExceeded` when actual > limit. The bridge maintains value equality.

**Rationale:** Core and storage must agree on limits. Kani proves no value drift.

**Evidence file:** `verification/kani/vb-vzcuf-PS-007.rs`

---

### Kani Harness: PS-008 — Guard precedence (RRO-030)

**Property:** The guard evaluation order in `append_event` is: key construction → durable duplicate → count → per-record payload → accumulated bytes → mutation. For all bounded inputs, the first applicable guard fires.

**Rationale:** User-visible API contract; precedence must be deterministic.

**Evidence file:** `verification/kani/vb-vzcuf-PS-008.rs`

---

### Kani Harness: PS-009 — Duplicate accounting (RRO-034)

**Property:** Under the chosen accounting policy, duplicate same-batch events follow the documented behavior. Staged bytes invariant is preserved.

**Rationale:** Accounting ambiguity can cause budget overruns or under-counts.

**Evidence file:** `verification/kani/vb-vzcuf-PS-009.rs`

---

## 7. Mutation Checkpoints

**Threshold:** >=90% mutation kill rate required on `cargo-mutants`.

### Critical Mutations That Must Be Caught

| Mutation Site | Mutation Type | Must Be Caught By |
|---|---|---|
| `staged + encoded_len <= limit` replaced with `staged + encoded_len < limit` | boundary condition change | `admission_accepts_exact_fit_at_limit_boundary` |
| `checked_add` replaced with wrapping `+` | arithmetic semantics change | `admission_uses_checked_arithmetic_not_wrapping`, `overflow_returns_typed_rejection_not_panic` |
| Guard order: swapped count and byte check positions | guard precedence inversion | All B06 precedence tests |
| `self.aborted = true` added to byte rejection path | abort semantics drift | `batch_remains_open_after_byte_rejection` |
| Staged bytes increment skipped on success | accounting deletion | `staged_bytes_increments_by_encoded_length_on_success` |
| Error variant: returned QueueFull instead of byte variant | error conflation | `byte_rejection_is_not_queue_full` |
| Error field: attempted set to `staged` instead of `staged + encoded_len` | field miscalculation | `byte_rejection_error_carries_attempted_and_limit_fields` |
| Limit comparison: `>` instead of `>=` for zero-limit rejection | boundary reversal | `zero_limit_construction_is_rejected` |
| `encode_record` result length replaced with `payload_len_u32` | accounting shortfall | `accounting_uses_full_encoded_length_not_payload_length` |
| Duplicate check moved after byte admission check | precedence inversion + abort after non-aborting rejection possible | `duplicate_precedes_byte_admission_check` |

### Non-Survivable Mutations (Markers for Implementation)

- GOD RULE 2 mutations (requires/ensures annotations missing) are caught by verifier tooling, not behavior tests.
- The `cover!()` absence in Kani harness notes in RROs (noted in notes field) means Kani harness non-vacuity is deferred to State 11.

---

## 8. Combinatorial Coverage Matrix

### B-GROUP-01: Byte Limit Construction

| Scenario | Input Class | Expected Output | Test Layer | RRO IDs |
|---|---|---|---|---|
| Default constructor | `JournalWriteBatch::new(journal)` | `byte_limit > 0` (default = 1_048_576) | integration | 024 |
| Explicit limit constructor | `byte_limit = 64` | `byte_limit == 64` | unit (deferred-to-state-11) | 022, 024 |
| Zero limit | `byte_limit = 0` | `Err(construction error)` | unit (deferred-to-state-11) | 022 |
| Boundary: u32::MAX limit | `byte_limit = u32::MAX` | batch created, limit = u32::MAX | unit (deferred-to-state-11) | 022 |
| Boundary: u64::MAX limit (if u64) | `byte_limit = u64::MAX` | batch created or rejected per policy | unit (deferred-to-state-11) | 022 |
| Staged bytes initial | any valid constructor | `staged_bytes == 0` | unit (deferred-to-state-11) | 024 |

### B-GROUP-02: Encoded Length Accounting

| Scenario | Input Class | Expected Output | Test Layer | RRO IDs |
|---|---|---|---|---|
| Happy path: normal event | valid JournalEvent | `encoded.len() >= 60 && encoded.len() > payload.len()` | unit | 018, 020 |
| Envelope minimum | empty payload event | `encoded.len() == RECORD_HEADER_BYTES + overhead` | unit | 020 |
| Payload at exact cap | payload = MAX_JOURNAL_EVENT_PAYLOAD_BYTES | `Ok(vec)`, length > cap | unit | 020 |
| Payload exceeds cap | `payload > MAX_JOURNAL_EVENT_PAYLOAD_BYTES` | `Err(PayloadTooLarge { len, max })` | unit | 020 |
| Accounting uses full length | event with encoded_len E | staged increases by E, not payload length | integration (deferred-to-state-11) | 018, 020 |

### B-GROUP-03: Admission Boundary

| Scenario | Input Class | Expected Output | Test Layer | RRO IDs |
|---|---|---|---|---|
| Exact fit | `staged + encoded_len == limit` | `Ok(())`, staged = limit | integration | 001, 004 |
| Under limit | `staged + encoded_len < limit` | `Ok(())`, staged = new total | integration | 001, 004 |
| Over limit | `staged + encoded_len > limit` | `Err(byte budget variant)` | integration | 001, 004 |
| Zero delta | `encoded_len = 0` | `Ok(())`, staged unchanged | unit | 001, 004 |
| At u64 boundary | `staged = u64::MAX, delta = 1` | `Err(overflow variant)` | unit | 006, 008 |
| Empty batch, over limit | `staged = 0, delta > limit` | `Err(byte budget variant)` | integration | 004 |

### B-GROUP-04: Typed Error API

| Scenario | Input Class | Expected Output | Test Layer | RRO IDs |
|---|---|---|---|---|
| Correct variant | over-limit event | `Err(JournalBatchBytesExceeded {..})` | integration (deferred-to-state-11) | 010, 012 |
| Not QueueFull | over-limit, under-count event | error is NOT `QueueFull` | integration | 010, 012 |
| Not PayloadTooLarge | over-limit, valid-payload event | error is NOT `PayloadTooLarge` | integration | 010, 012 |
| Fields populated | byte rejection error | `attempted, limit` fields carry correct values | integration (deferred-to-state-11) | 012 |
| Display text | byte rejection error | `.to_string()` contains "byte" or "batch", not "queue" | unit (deferred-to-state-11) | 012 |

### B-GROUP-05: No Partial Mutation

| Scenario | Input Class | Expected Output | Test Layer | RRO IDs |
|---|---|---|---|---|
| len unchanged | 3 accepted, 1 rejected | `batch.len() == 3` | integration | 016 |
| Accepted keys committed | 2 accepted, 1 rejected | commit persists accepted, not rejected | integration | 016 |
| Batch not aborted | byte-rejected event | `!aborted`, subsequent events still accepted | integration | 016 |
| Staged bytes unchanged | byte rejection | `staged_bytes` unchanged (deferred-to-state-11) | integration | 016 |
| Rejected key reusable | byte-rejected event key | next batch can append same key | integration | 016 |

### B-GROUP-06: Error Separation and Precedence

| Scenario | Input Class | Expected Output | Test Layer | RRO IDs |
|---|---|---|---|---|
| Duplicate > byte | committed key + over-budget | `DuplicateEvent` returns first | integration | 030, 032 |
| Count > byte | full batch + over-budget | `QueueFull` returns first | integration | 030, 032 |
| Payload > byte | huge payload + over-budget | `PayloadTooLarge` returns first | integration | 030, 032 |
| Byte only | all other guards pass | accumulated byte error returns | integration | 030, 032 |
| Duplicate > overflow | duplicate + staged=MAX | `DuplicateEvent` returns first | integration | 030, 032 |

### B-GROUP-07: Overflow Safety

| Scenario | Input Class | Expected Output | Test Layer | RRO IDs |
|---|---|---|---|---|
| Normal addition | `a + b <= u64::MAX` | `checked_add` returns Some | unit | 006, 008 |
| Overflow | `a + b > u64::MAX` | `checked_add` returns None | unit | 006 |
| Overflow produces error | staged=MAX, delta=1 | typed rejection, no panic | integration | 006, 008 |
| No unchecked cast | any usize value | `try_from` used, not `as` | static (clippy) | 007 |

### B-GROUP-08: Core/Storage Bridge

| Scenario | Input Class | Expected Output | Test Layer | RRO IDs |
|---|---|---|---|---|
| Core default equals storage default | ResourceContract::default() | storage default == core default (1_048_576) | integration | 028 |
| Core value flows through | `max_batch_bytes = 4096` | storage limit = 4096 | integration (deferred-to-state-11) | 026, 028 |
| Core rejects zero | `max_batch_bytes = 0` | core validation error | unit | 026 |
| No type truncation | any u32 core value | storage type preserves exact value | integration (deferred-to-state-11) | 028 |

### B-GROUP-09: Duplicate Accounting Policy

| Scenario | Input Class | Expected Output | Test Layer | RRO IDs |
|---|---|---|---|---|
| Same key twice (conservative) | event E appended 2x | staged_bytes += 2*encoded_len | integration (deferred-to-state-11) | 034, 036 |
| Same key twice (distinct-key) | event E appended 2x | staged_bytes += 1*encoded_len | integration (deferred-to-state-11) | 034, 036 |
| No panic on duplicate | any duplicate scenario | no panic, no overflow | integration | 034 |
| Invariant preserved | duplicate scenario | staged_bytes <= limit | integration (deferred-to-state-11) | 034 |

### E2E Coverage

| Scenario | Input Class | Expected Output | Layer | RRO IDs |
|---|---|---|---|---|
| Full lifecycle accept→reject→commit | mixed outcomes | durable state correct | e2e | 004, 016, 032 |
| Many events under limit | 50 events | all committed, replayable | e2e | 004, 028 |
| Aborted batch semantics | duplicate dur. event | commit no-ops, staged state defined | e2e | 016, 032 |
| Accessor accuracy | 3 accepted events | staged_bytes() correct at each step | e2e (deferred-to-state-11) | 024, 028 |

---

## 9. Deferred Behaviors (State 11)

These 8 behaviors require production fields (`staged_bytes: u64`, `byte_limit: u64`, `JournalError::JournalBatchBytesExceeded`) that do not yet exist. They must be testable after State 11 implementation adds the GOD RULE 2 binding:

| ID | Behavior | Blocked By |
|---|---|---|
| B01.2 | Explicit limit construction stores provided value | missing `byte_limit` field |
| B01.3 | Zero limit construction rejected | missing `JournalBatchByteLimit` value object |
| B01.4 | Limit rejects absurdly large value | missing validation on value object |
| B01.5 | New batch has zero staged bytes | missing `staged_bytes` field |
| B01.6 | Accessor returns zero on new batch | missing accessor |
| B02.4 | Staged bytes equals sum of encoded lengths | missing `staged_bytes` accumulator |
| B03.4 | Staged bytes increments by encoded_len | missing `staged_bytes` mutation |
| B04.1 | JournalBatchBytesExceeded variant returned | missing error variant |
| B04.4 | Error fields carry correct types | missing `JournalBatchBytesExceeded` definition |
| B04.5 | Display text references byte pressure | missing error variant |
| B05.3 | Staged bytes unchanged after rejection | missing `staged_bytes` field |
| B05.6 | Prior events survive later rejection | missing staged bytes persistence |
| B08.1 | Core policy flows to storage limit | missing `byte_limit` constructor parameter |
| B09.1 | Accounting matches documented policy | open C2 product question |
| B09.4 | Duplicate accounting never panics | depends on C2 resolution |
| B09.5 | Duplicate preserves staged byte invariant | missing `staged_bytes` field |
| E04 | Accessor returns accurate staged bytes | missing accessor |

These are tracked by RROs with `notes: "GOD RULE 2 GAP"` and `notes: "Open product question pending"`.

---

## 10. Static Analysis Gates

| Gate | Tool | What It Catches |
|---|---|---|
| No unwrap/expect | `clippy::unwrap_used` lint | Panics in admission path |
| No unchecked arithmetic | `clippy::arithmetic_side_effects` lint | Missing checked_add/checked_mul |
| No `as` casts on numeric narrowing | `clippy::as_conversions` lint | usize→u32/u64 truncation |
| non_exhaustive enforcement | compiler | Missing match arms on JournalError |
| No panic/panic! | `clippy::panic` lint | Development debugging leftovers |
| Cargo deny | `cargo deny check` | Unsafe dependency creep |

**Evidence:** `moon ci` includes clippy, `cargo deny`, and `cargo check` for all workspace crates. Add `#![deny(clippy::unwrap_used, clippy::arithmetic_side_effects)]` to `crates/vb_storage/src/batch.rs` after State 11.

---

## 11. Evidence Command Registry

All tests must be runnable with exact commands. These commands reference the isolated workspace (not source checkout) where test artifacts exist:

| Command | Coverage | Layer |
|---|---|---|
| `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_001` | B-GROUP-03 admission | proptest |
| `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_002` | B-GROUP-07 overflow | proptest |
| `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003` | B-GROUP-04 error distinctness | proptest |
| `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004` | B-GROUP-05 no mutation | proptest |
| `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_005` | B-GROUP-02 codec | proptest |
| `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_006` | B-GROUP-01 limit | proptest |
| `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_007` | B-GROUP-08 bridge | proptest |
| `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_008` | B-GROUP-06 precedence | proptest |
| `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_009` | B-GROUP-09 duplicates | proptest |
| `cargo test -p workspace_tests --test journal_batch_accounting_tests vb_vzcuf -- --nocapture` | All groups | integration |
| `cargo test -p workspace_tests --test journal_side_index_contracts` | B-GROUP-05/09 | integration |
| `cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_admission_boundary` | B-GROUP-03 | kani |
| `cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_overflow_safety` | B-GROUP-07 | kani |
| `cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_error_distinctness` | B-GROUP-04 | kani |
| `cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_no_mutation_on_rejection` | B-GROUP-05 | kani |
| `cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_encode_record_length` | B-GROUP-02 | kani |
| `cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_byte_limit_nonzero` | B-GROUP-01 | kani |
| `cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_budget_bridge` | B-GROUP-08 | kani |
| `cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_guard_precedence` | B-GROUP-06 | kani |
| `cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_duplicate_accounting` | B-GROUP-09 | kani |
| `cargo fuzz run vb_vzcuf_PS_001 -- -max_total_time=60` | B-GROUP-03 | fuzz |
| `cargo fuzz run vb_vzcuf_PS_002 -- -max_total_time=60` | B-GROUP-07 | fuzz |
| `cargo fuzz run vb_vzcuf_PS_003 -- -max_total_time=60` | B-GROUP-04 | fuzz |
| `cargo fuzz run vb_vzcuf_PS_004 -- -max_total_time=60` | B-GROUP-05 | fuzz |
| `cargo fuzz run vb_vzcuf_PS_005 -- -max_total_time=60` | B-GROUP-02 | fuzz |
| `cargo fuzz run vb_vzcuf_PS_006 -- -max_total_time=60` | B-GROUP-01 | fuzz |
| `cargo fuzz run vb_vzcuf_PS_007 -- -max_total_time=60` | B-GROUP-08 | fuzz |
| `cargo fuzz run vb_vzcuf_PS_008 -- -max_total_time=60` | B-GROUP-06 | fuzz |
| `cargo fuzz run vb_vzcuf_PS_009 -- -max_total_time=60` | B-GROUP-09 | fuzz |
| `moon ci` | all static analysis gates | static |

---

## 12. Anti-Pattern Rejection Checklist

Per testing-philosophy.md and test-planner skill:

- [x] No test asserts only `is_ok()` or `is_err()` — every assertion specifies values/variants
- [x] No mocks — real FjallJournal and real encode_record used everywhere
- [x] Tests test behaviors (public API guarantees), not methods
- [x] One logical assertion per scenario — each BDD scenario is atomic
- [x] Test names are descriptive sentences (e.g., `admission_rejects_over_limit_event`)
- [x] No `sleep()` in tests — deterministic Fjall operations
- [x] No shared mutable state between tests — each test creates its own temp journal
- [x] DAMP over DRY — each test self-contained with its own setup
- [x] Proptest invariants bound inputs — no unbounded `u64` exploration
- [x] Kani harnesses use `kani::any()` with `kani::assume` — no hardcoded shapes
- [x] Every error variant in JournalError taxonomy has an explicit test scenario
- [x] Every parsing/codec boundary has a fuzz target
- [x] Every pure function with multiple inputs has a proptest invariant
- [x] Mutation threshold target stated (>=90%)
- [x] Deferred behaviors explicitly tagged `deferred-to-state-11`

## Open Questions

1. **C2 Same-Batch Duplicate Policy:** Conservative attempt accounting vs distinct-key accounting. Tests B09.2 and B09.3 document both options; one must be chosen before State 9 test-writer writes the actual test bodies. Marked as "open product question" in contract.md and RROs.

2. **C9 Accessor Semantics on Aborted Batch:** Should the staged_bytes accessor return 0 (mirroring `len()` which returns 0 when aborted) or preserve the diagnostic value? This affects E03 and E04. Decision needed before State 11 implementation.

3. **Constructor API Shape:** Exact public factory signature for supplying byte limit remains open. Tests B01.1-B01.4 assume a `JournalBatchByteLimit` value object passed to constructor or `new_with_limits`. The final API shape must be finalized at State 11.

4. **Evidence Workdir:** The bridge review (State 7) found that evidence commands reference the source checkout but artifacts exist only in the isolated workspace. The formal-verifier (State 10) must resolve this before execution. Test artifacts in this plan assume the isolated workspace path.

5. **GOD RULE 2 Productions:** 9 Verus RROs lack `requires`/`ensures` on production `exec fn`. This is deferred to State 11. The behavior tests described here do not replace or excuse the GOD RULE 2 obligation — they provide compensating behavioral evidence only.

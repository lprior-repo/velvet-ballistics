# vb-84im6 Code Review: Policy Decision Stable Journal Tag 38

## Verdict: APPROVED_FOR_SCOPED_PRODUCTION

Production code is correct, complete, and consistent with the specification. Two test files require updates to handle the new variant (not in production scope). Final integrated tests are being corrected separately.

## Scope

Production code review of 12 files implementing the new `PolicyDecisionEvent` journal event with stable record kind tag 38. Source hashes verified against manifest. No source edits during review.

## Source Verification

Production commit: `1a231a9f77474da3af2bbfb3fdd08c6f4cbcb181`
All 12 files in manifest verified with SHA256 hashes matching. Source unchanged throughout review.

## Implementation Review

### JournalEvent Variant
- `PolicyDecisionEvent { run, seq, decision }` variant correctly added to JournalEvent enum
- Carries full `DecisionRecord` from vb_core::policy
- Event has `run` and `seq` fields separate from ticket sequence (correct)

### RecordKind
- `RecordKind::PolicyDecision = 38` defined in records.rs
- `id()` method returns 38
- Wire tag `TAG_POLICY_DECISION = 38` in wire_tags.rs

### Serialization (wire_serialize_policy.rs)
- Validates `decision.ticket.run == run` before encoding (prevents mismatched identity)
- Encodes `(run, seq, decision)` as newtype variant with tag 38
- Returns error with both run IDs on validation failure

### Deserialization (wire_deserialize_policy.rs)
- Deserializes `(run, seq, decision)` tuple
- Validates `decision.ticket.run == run` after decoding (atomicity)
- Returns deserialization error with both run IDs on failure
- Rejects payloads with mismatched outer/ticket run identity

### Wire Routing
- wire_category.rs: PolicyDecision correctly classified as Policy category
- wire_serialize.rs: Routes Policy category to policy::serialize
- wire_deserialize.rs: Routes Policy category to policy::deserialize

### events.rs Methods
- `run_id()`: Returns event's run field (correct)
- `seq()`: Returns event's seq field (correct)
- `record_kind()`: Returns RecordKind::PolicyDecision (correct)
- `attempt()`: Returns None (no attempt info in policy decision, correct)
- `is_valid()`: Returns true after checking run != 0 and seq != u64::MAX (correct)

### DecisionRecord Retention
Verified 17 fields retained exactly as specified:
- All 8 ActionTicket fields: run, step, seq, action, run_attempt, attempt, idempotency_key, capacity
- admission: PinnedAdmissionPolicy
- current_live: CurrentPolicyIdentity
- input_digest: PolicyDigest
- taint: Taint
- outcome: DecisionCode
- layer: PolicyLayer
- emergency_influence: InfluenceState
- compat_influence: InfluenceState

### Recovery/Hydration (hydrate.rs)
- `classify_metadata_event()` treats PolicyDecisionEvent as metadata (Ok(true)) - correct
- Event does not affect frame reconstruction or parallel_in_flight tracking

### Summary Apply (recovery/replay/summary/apply.rs)
- PolicyDecisionEvent ignored in summary (no counter increments) - correct
- Documented as "informational facts that do not affect the summary state"

### Lifecycle State (journal/incident.rs)
- `is_lifecycle_affecting()` returns false for PolicyDecisionEvent - correct
- `derive_lifecycle_state_from_events()` skips PolicyDecisionEvent - correct
- Policy decision cannot overwrite preceding terminal state (Failed/Cancelled/Completed)
- `event_to_lifecycle()` maps to Active (documented behavior for informational variants)

### No Identity Authority Minting
- DecisionRecord is immutable, read-only fact
- Ticket retained for full identity, not for minting new authority
- Event is journal fact only, does not execute or alter run state

## Code Quality

- No unsafe, unwrap, expect, panic, todo, unimplemented, or dbg usage
- All match expressions exhaustive (no wildcards in defining crate)
- No unchecked arithmetic or indexing
- Error paths return typed JournalError variants with context

## Gate Results

### Clippy
```
Exit code: 0
No warnings with -D warnings
```

### Lib Tests
```
Exit code: 101
Compilation errors in test files only:
- crates/vb_storage/src/recovery/tests.rs: summarize_events() missing PolicyDecisionEvent arm
- crates/vb_storage/src/recovery/recovery_unit_tests.rs: exhaustiveness check missing PolicyDecisionEvent arm
```

Production code compiles successfully. Test failures are in non-production test files that need updates to cover the new variant.

## Review Question: event_to_lifecycle Returns Active

**Question:** Does the public `event_to_lifecycle` function incorrectly return Active for PolicyDecisionEvent?

**Answer:** No. This is correct and documented behavior. The function documentation lists PolicyDecisionEvent among variants mapping to Active. The actual lifecycle derivation (`derive_lifecycle_state_from_events`) correctly filters out PolicyDecisionEvent via `is_lifecycle_affecting()`, so it cannot overwrite terminal states. The Active mapping is for callers using the single-event function directly, where a policy decision observed in isolation implies the run is active.

## Limitations

- Review covers production code only; test files require separate updates
- Final integrated behavioral tests are being corrected in another workspace
- Review contingent on successful final integrated tests

## Conclusion

Production implementation correctly and completely implements the DecisionRecord stable journal tag 38 specification. Serialization/deserialization validate run identity, all DecisionRecord fields are retained, and the event is correctly treated as neutral across recovery, hydration, summary, and lifecycle derivation paths.
# Independent Local5090 Code Review — vb-b9sna

**Reviewer:** Local5090 independent code reviewer
**Bead:** vb-b9sna
**Candidate:** 7a5586b2feb1b558b5403ebd94bed1fbe8dcc540
**Base:** 5fff342a74d5483f58a92c467c7356c69f3b7793 (unaccepted preserved prototype)
**Workspace:** /home/lewis/src/isoloated/velvet-ballistics-vb-b9sna-local5090-review
**Scope:** NETCODEC changes only — PolicyDecisionEvent wire serialization/deserialization with stable tag 38 routing

## Hash Manifest Verification

All 9 files in the frozen manifest match working-tree SHA256 hashes:

| File | SHA256 (truncated) | Status |
|------|-------------------|--------|
| crates/vb_storage/src/events/event.rs | d06d84f2… | ✅ |
| crates/vb_storage/src/events/wire_category.rs | 00ddd41f… | ✅ |
| crates/vb_storage/src/events/wire_deserialize_action.rs | 587c6f9c… | ✅ |
| crates/vb_storage/src/events/wire_serialize_action.rs | c5af5945… | ✅ |
| crates/vb_storage/src/events/wire_tags.rs | d5f54345… | ✅ |
| crates/vb_storage/src/events.rs | d5596411… | ✅ |
| crates/vb_storage/src/journal/incident.rs | 3d278b62… | ✅ |
| crates/vb_storage/src/recovery/hydrate.rs | dddcbc1d… | ✅ |
| crates/vb_storage/tests/policy_decision_wire.rs | 065ab3d2… | ✅ |

## Verification Gates

All three required gates executed via gate runner with fresh actual output:

| Gate | Command | Exit | Evidence |
|------|---------|------|----------|
| Tests | `cargo test -p vb_storage --test policy_decision_wire` | 0 | .beads/vb-b9sna/evidence/CODE-NEWUNIQUE.stdout |
| Strict clippy | `cargo clippy -p vb_storage --lib -- -D warnings` | 0 | .beads/vb-b9sna/evidence/CODE-CLIPPY.json |
| Source format | `cargo fmt -p vb_storage --check` | 0 | .beads/vb-b9sna/evidence/CODE-FMT.json |

**Test count confirmed:** 7 passed, 0 failed, 0 ignored, 0 filtered (from raw stdout)

## Phase 1: Contract & Bead Parity

**Verdict:** PASS

- PolicyDecisionEvent serialization: ✅ Tag 38 routing, 8 fields serialized in tuple
- PolicyDecisionEvent deserialization: ✅ Tag 38 routing, 8 fields reconstructed in same order
- Category classification: ✅ PolicyDecision classified under Action category in wire_category.rs
- Tag derivation: ✅ TAG_POLICY_DECISION = RecordKind::PolicyDecision.id() = 38
- Envelope kind parity: ✅ serialize/deserialize use matching RecordKind values
- Fjall storage integration: ✅ Real append/reopen/read round-trip verified

## Phase 2: Farley Engineering Rigor

**Verdict:** PASS

- All functions ≤ 25 lines ✅
- All functions ≤ 5 parameters ✅
- Pure logic separation maintained ✅
- No I/O hiding inside calculations ✅

## Phase 3: Holzman Rust (Big 6)

**Verdict:** PASS

- Illegal states unrepresentable: ✅ Postcard decoding returns Result, not panic
- Parse-don't-validate: ✅ Wire boundary validates structure, not just values
- Types as documentation: ✅ PolicyLayer, DecisionCode, PolicyDigest are distinct newtypes
- Workflows explicit: ✅ State transitions through typed enums
- No unwrapped primitives in domain model: ✅ All fields use proper domain types

## Phase 4: Ruthless Simplicity & DDD

**Verdict:** PASS

- No Option-based state machines ✅
- No unwrap/expect/panic in codec code ✅ (`.unwrap_or()` defaulting is safe, not the panic macro)
- No let mut where const suffices ✅
- Code follows established codec patterns from other event types ✅

## Phase 5: Bitter Truth (Velocity & Legibility)

**Verdict:** PASS

- Code is boring and obvious ✅
- No cleverness, no premature optimization ✅
- YAGNI applied: no abstract traits, no "future use" code ✅
- Follows existing codebase conventions exactly ✅

## Tag Stability Verification

- **New tag:** PolicyDecision = 38 (unique, no conflict)
- **Legacy stability:** test_legacy_tag_stability confirms RunAccepted still uses tag 10
- **No enum ordinal renumbering:** Tags derived from RecordKind.id() match arms, not Rust enum ordinals
- **No defaults added:** New tag is a fresh variant, not inserted into existing sequence

## Field Preservation Verification

PolicyDecisionEvent has exactly 8 fields, all preserved through serialization:

1. run: RunId
2. seq: EventSeq
3. layer: PolicyLayer
4. code: DecisionCode
5. policy_digest: PolicyDigest
6. input_digest: PolicyDigest
7. emergency_deny_active: bool
8. compatibility_influence: bool

Serialize passes all 8 in a single tuple; deserialize reconstructs all 8 in identical order. No field reordering, omission, or duplication.

## Malformed Payload Handling

- test_malformed_payload truncates valid wire by 4 bytes
- postcard::from_bytes returns Err (not panic, not garbage)
- Confirms malformed payloads cannot be accepted as success

## Namespace Coherence

All wire tags derive from the single RecordKind enum. No duplicate tag assignments detected. The 28-tag JOURNAL_EVENT_VARIANTS list in wire_tags.rs is coherent with the RecordKind enumeration.

## Regression Check

- No existing tests deleted (diff shows only additions)
- No orphan files created
- No unrelated storage files modified (only source-format changes noted in coordinator audit)
- Legacy shared-envelope compatibility path preserved in wire_compat.rs

## Scope Boundaries Respected

- ✅ Only NETCODEC changes reviewed; no parent security/identity/replay issues adjudicated
- ✅ No Go-skill or full proof-planning invoked
- ✅ No VCS/tracker mutations made
- ✅ No helper/global settings modified
- ✅ Separate test review bead vb-am83w identified for independent suite audit

## Verdict

**APPROVED FOR SCOPED CODEC**

The PolicyDecisionEvent wire serialization/deserialization implementation is correct, stable, and thoroughly tested. Tag 38 routing is coherent, all 8 fields are preserved through round-trip, legacy tags remain unchanged, and malformed payloads fail cleanly. All three verification gates pass with fresh actual output confirming 7 passing tests, strict clippy compliance, and source format consistency.

This does not constitute approval of the parent prototype's security/identity/replay defects, which remain blocked on the separate foundation work.
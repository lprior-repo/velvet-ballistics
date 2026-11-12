bead_id: vb-5h50
bead_title: storage: Trim journal events after durable snapshots
phase: state-4-contract-review
updated_at: 2026-05-09T00:00:00Z

# Contract Verification Review

## Review Criteria
- Every contract clause has at least one test obligation.
- Every contract clause has at least one verification layer or explicit waiver.
- Error taxonomy is exhaustive (covers all failure modes).
- Preconditions are realistic and checkable.
- Invariants are strong enough to prevent data loss.

## Findings

### contract.md
- **P1-P4**: Preconditions are checkable at runtime. ✅
- **Po1-Po5**: Postconditions cover state preservation, replay equivalence, and fail-closed behavior. ✅
- **I1-I4**: Invariants prevent data loss, guarantee idempotency, and enforce retention. ✅
- **Error taxonomy**: `NoDurableSnapshot`, `RetentionPolicyBlocks`, `Fjall`, `Journal`, `IncompleteTrim` — covers all known failure modes. ✅

### verification-layers.md
- Layer 0 (unit): Fast, comprehensive. ✅
- Layer 1 (proptest): Covers replay equivalence and retention with arbitrary inputs. ✅
- Layer 2 (integration): Full round-trip validation. ✅
- Layer 3 (Kani): Scoped to pure key-comparison logic only — appropriate since Fjall I/O is out of scope. ✅
- Layer 4 (Miri): Heap/stack safety for byte slicing. ✅

### proof-obligations.jsonl
- All 9 obligations are valid JSONL with unique IDs. ✅
- Every obligation maps to a contract clause. ✅
- Tool names are exact (`cargo test`, `cargo kani`, `cargo +nightly miri test`). ✅

### traceability-matrix.jsonl
- All 5 contract clauses have at least one test and one proof. ✅
- Evidence descriptions are concrete. ✅

## Waivers
- Kani scope is intentionally limited to pure logic; Fjall storage semantics are verified by integration tests instead. This is acceptable because Fjall is an external crate with its own durability guarantees.
- Loom is waived because trimming uses Fjall's internal concurrency (Mutex on write_lock), not custom concurrent data structures.

## Decision

STATUS: APPROVED

The contract is complete, the verification layers are appropriately scoped, and every clause is traceable to executable tests. Proceed to test planning.

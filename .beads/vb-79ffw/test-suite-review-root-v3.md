# Behavior test suite review

STATUS: REJECTED

Reviewed source checkpoint3292293143bf2808e3803e59fe614fc8ea43dc88 with the test-reviewer skill. No proof artifacts or proof plans reviewed.

## Findings

1. **blocker — legacy fixture is not a valid record from the stated old production.** `crates/vb_storage/tests/legacy_wire_decoder.rs` serializes a bare payload struct; actual old JournalEvent serialization includes the event discriminant/newtype variant. A current SerdeDeCustom error on that malformed payload does not establish rejection of a valid old ActionTicket record. Validating the envelope CRC/digest is insufficient. Required repair: generate exact records through old423d8ae8 production and require exact old public decoder acceptance before current-decoder tests.
2. **blocker — mutation oracle permits mutation.** `crates/vb_runtime/tests/vb_79ffw_ticket_authority_probe.rs` asserts `events_after_stale.len() >= events_before.len()`. Appending corrupt events or changing every existing payload can still pass. Required repair: compare complete serialized event vectors exactly and assert the pending ticket/frame state available through public APIs.
3. **blocker — stale tick error is logged, not asserted.** The match accepts either Ok or Err. Required repair: assert the contract's exact rejection result/fields and retain current-ticket completion through a terminal event afterward.
4. **blocker — new tests violate repository safety rules.** The wire test introduces unchecked indexing/cast, expect, and explicit panic. Required repair: fallible helpers, safe access, checked conversion, concrete assertions. No lint allowance.
5. **blocker — legacy representativeness/provenance missing.** One arbitrary noncanonical old key is used. Required repair: validated golden cases including canonical old keys and boundary fields; report only actually tested cases and migration limits.

## Actual evidence and limits

The v3 wire test really exited0 and the runtime probe really reached RunFinished, but these facts do not discharge the findings above. Prior report claims of full serialized equality and exact tick-error assertions are unsupported. No advancement to main approval until every finding is fixed with actual evidence. The existing repository baseline or a previous weak test does not justify these gaps. No source/test edits performed by this reviewer.

# Trusted Base Plan — vb-y9d3v

Every assumed, trusted, stubbed, bounded, or reduced surface used across planned obligations. Each row maps to a planned `trusted-base-ledger/v1` row to be materialized by proof-writer (State 5) and reviewed by proof-reviewer (State 6).

## Trusted Base Items

| ID | Obligation(s) | Surface | Location | Kind | Reason | Behavior-Affecting | Compensating Evidence |
|---|---|---|---|---|---|---|---|
| TBP-001 | All (PO-001 through PO-028) | Shard state initialization (RunState, runs map, workflow, action_attempts) | crates/vb_runtime/src/shard/types.rs | assume | Harnesses construct valid RunState for the unit under test; the shard state is trusted as a well-formed input boundary for pure function verification. | true | Integration tests (State 8-10) and proptest properties exercise full shard lifecycle; Kani harness uses Arbitrary generation not hardcoded shapes. |
| TBP-002 | PO-001, PO-002, PO-004, PO-005, PO-009, PO-011, PO-013, PO-017, PO-018, PO-020, PO-022, PO-024, PO-026 | ActionTicket DTO and vb_core public types (RunId, StepIdx, SeqNo, ActionId, Workflow) | crates/vb_core/src/action.rs, crates/vb_core/src/ids.rs, crates/vb_core/src/workflow.rs | external_body | vb_core types are outside vb_runtime's proof scope; they are consumed as trusted public API types. | true | vb_core has its own verification lanes; type-contracts.md defines invariant contracts; proptest strategies validate field-level invariants. |
| TBP-003 | PO-002, PO-018 | Future-attempt domain decision: attempt > current is rejected | crates/vb_runtime/src/shard/helpers.rs:72-94 | model_bound | Fresh-main code currently accepts future attempts within capacity (implementation gap). Until implementation is corrected, proof obligations target the contract semantics. The assumption that code will be fixed is a model bound. | true | Contract ACT-006 explicitly requires this; acceptance invariant #2 demands coverage; State 11 (implementation) must close the gap before State 12 formal execution. |
| TBP-004 | PO-003, PO-007, PO-010, PO-012, PO-019, PO-023, PO-027 | Retry metadata and retry policy extraction | crates/vb_runtime/src/shard/helpers.rs:224-294 | model_bound | Retry metadata slot values are trusted to be valid I64 when present. Extracted values are bounded to u16 via checked conversion. | true | Kani harnesses verify the Err paths for non-I64/zero/out-of-range values; proptest exercises randomized slot values. |
| TBP-005 | PO-004, PO-009, PO-013, PO-020, PO-024, PO-026 | Workflow compilation and action contract resolution | crates/vb_core/src/workflow.rs, crates/vb_runtime/src/engine/ | external_body | Workflow compilation occurs at host time, outside runtime fence scope. Action contract lookup is trusted to return valid contracts for registered actions. | true | Workflow compilation has separate verification; invalid action id produces InvalidActionCompletion at the preflight gate independently. |
| TBP-006 | PO-008, PO-015, PO-016, PO-021 | Timer wheel data structure (deadline queue, run-indexed generation map) | crates/vb_runtime/src/shard/timer_wheel.rs | model_bound | Timer wheel internals (BinaryHeap ordering, generation map) are abstracted; proofs target the generation comparison logic at fire_expired boundary. | true | Unit tests for timer_wheel.rs cover deadline ordering and generation monotonicity; TLA+ model (PO-028) models the full wheel lifecycle. |
| TBP-007 | PO-011 | Verus model extraction: pure helper functions extracted from production code | crates/vb_runtime/src/verification/verus/vb_y9d3v_action_fence.rs | model_bound | Verus proofs operate on extracted pure helper functions that mirror production logic. The bridge (proof-to-implementation) must name production source refs and verify behavioral equivalence. | true | Extracted helpers are verbatim copies of production pure functions; bridge verification (State 7) confirms no semantic drift. |
| TBP-008 | PO-028 | TLA+ model bounded to MAX_U64 (GOD RULE 3) | verification/tla/vb_y9d3v_ActionAuthority.tla | model_bound | TLA+ uses bounded integers (MAX_U64) per GOD RULE 3. The model is temporal design evidence, not Rust implementation proof. | false (model-only evidence) | Rust implementation proofs (PO-001 through PO-027) close the Rust-local invariants; TLA+ adds temporal ordering evidence. |

## Trusted Base Debt Tracking

All 8 trusted base items are `planned` with `owner_state: 5` (proof-writer). The proof-writer must:
1. Materialize exact source locations for each trusted surface
2. Record `trusted-base-ledger/v1` rows with SHA-256 hashes of the trusted artifacts
3. Mark any `assume`, `axiom`, `admit`, `external_body`, `trusted`, `ignore`, stub, disabled check, or model reduction

The proof-reviewer (State 6) must independently verify that:
- No behavior-affecting trusted base item lacks compensating evidence
- TBP-003 (future-attempt gap) is accompanied by an explicit implementation change plan
- TBP-007 (Verus model extraction) has a bridge row confirming production equivalence
- TBP-008 (TLA+ model) is not cited as Rust implementation closure

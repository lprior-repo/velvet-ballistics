# Theorem Kernel Projection: vb-qi37.4

## Boundary
- TLA+-owned temporal model: admission failure/rejection/ack lifecycle and persistence-before-ack ordering.
- Verus-owned Rust core: digest equality model, gate-count/proof-flag validation, exact capability matching, and fail-closed pure decision functions.
- Theorem-owned kernel: none required at State 3.
- Rust/runtime shell: Fjall I/O, postcard codec execution, shard mutation, async/CLI/API/IPC envelopes, and wall-clock fields.
- External systems excluded from theorem proof: filesystem, database flush behavior, process crash/restart, and operator CLI rendering.

## Theorem-Owned Clauses
- None.

## Rationale
- The critical local properties are finite field/refinement checks and set/cardinality predicates expressible in Verus.
- The critical workflow property is temporal and already mapped to TLA+.
- No algebraic kernel currently needs Lean/Aeneas/Hax beyond Verus; adding one would be proof-surface inflation unless proof review finds a specific Verus limitation.

## Waivers
- LEAN-WAIVE-001: Lean/Aeneas/Hax not required for `vb-qi37.4` State 3. Owner: contract/proof-planning. Reason: Verus and TLA+ own all identified high-assurance clauses. Expiry: before State 5 proof-review if Verus cannot express digest/capability/gate-count invariants. Compensating evidence: Verus obligations plus Kani/proptest/fuzz/integration gates in `proof-obligations.jsonl`.

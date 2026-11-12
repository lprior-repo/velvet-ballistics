# Test Plan: Persist run header before acknowledgement

## Summary
- Behaviors identified: 5
- Trophy allocation: 1 unit / 4 integration / 0 e2e; durability is cross-crate integration-heavy.
- Proptest invariants: 1
- Fuzz targets: 0; no parser/deserializer boundary in bead scope.
- Kani harnesses: 0; no arithmetic/index kernel in bead scope.

## 1. Behavior Inventory
- Runtime rejects duplicate run ids before allocating a second active state.
- Runtime rejects failed admission before state allocation.
- Runtime acknowledges submit only after run header/admission persistence.
- Runtime returns typed durability error when header persistence fails before ack.
- Recovery finds persisted run header/admission by exact run id and digest.

## 2. Trophy Allocation
| Behavior | Layer | Rationale |
|---|---|---|
| duplicate run id | unit | existing shard unit boundary |
| admission reject before allocation | integration | artifact store + shard |
| before-ack persistence | integration | runtime + journal |
| failure injection | integration | storage failure seam |
| recovery lookup | integration | persisted journal replay |

## 3. BDD Scenarios
### submit_rejects_duplicate_run_id
Given: run id R already exists.
When: submit R again.
Then: returns `Err(RuntimeError::RunAlreadyExists)` and active run count is unchanged.

### admission_rejection_does_not_insert_run_state
Given: admission policy rejects workflow digest D.
When: submit run R with D.
Then: returns exact admission `RuntimeError` variant and R is absent from active runs.

### storage_failure_before_header_prevents_ack
Given: admission succeeds but journal append for header/admission fails.
When: submit run R.
Then: returns exact durability `RuntimeError` and no active run R exists.

### restart_lookup_finds_persisted_header
Given: submit succeeds with durable journal.
When: runtime restarts and replays journal.
Then: recovered header contains exact R, workflow digest, policy, and capabilities.

## 4. Proptest Invariants
- For any valid workflow digest and run id, persisted header digest equals workflow digest after successful submit.

## 5. Fuzz Targets
- None in scope.

## 6. Kani Harnesses
- None in scope.

## 7. Mutation Checkpoints
- Deleting persistence-before-ack branch must be killed by `storage_failure_before_header_prevents_ack`.
- Returning default digest must be killed by `restart_lookup_finds_persisted_header`.
- Threshold: 90% mutation kill rate minimum for touched paths.

## 8. Combinatorial Coverage Matrix
| Scenario | Input Class | Expected Output | Test Layer |
|---|---|---|---|
| duplicate | existing run id | `Err(RuntimeError::RunAlreadyExists)` | unit |
| rejected admission | absent/invalid artifact | exact admission error | integration |
| append failure | storage fails before header | exact durability error, no active run | integration |
| success | valid artifact/storage | recovered exact header | integration |

## Open Questions
- Exact test-support failure-injection seam must be selected by test-writer without weakening production durability.

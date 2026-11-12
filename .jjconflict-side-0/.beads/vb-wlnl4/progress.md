# Integration Progress: vb-wlnl4 (accurate)

Integration engineer: DeepSeek v4 Pro via OMP. Candidate: `codex/vb-wlnl4-local-integration-candidate`.
Published checkpoints (fast-forward children of 1e7aeee2): `013189c5` → `685a70fc`.

## Done (verified with true-exit runner)

1. **Digest repair (c789b1b3) merged** over 1e7aeee2 (clean, disjoint file sets).
   F-4 fail-closed, F-5 at_once/pages/items resolve, F-6 justification, T-2 oracles.
2. **C6 retained-input recovery** fixed (YAML parse + journaled policy). Green end-to-end.
3. **F-1 executor guard** wired: `validate_registered_recovery` called during recovery
   (dead-code warning gone); legacy recovery sets `ReconcileOnly` for pending-action.
4. **Capability grammar fixture** fixed (hyphens invalid). 10/13 fjall tests recovered.
5. **AskTicket enforcement** verified intact (1934 vb_runtime lib tests, 0 failures).

## Remaining (precise, not "hash pending")

- **F-2 durable retry end-to-end**: `cmd_retry` stub; no `RunCheckpointV1::capture`
  producer; no `RetryCheckpoint` emission. Red-phase file documents gap (vb-qi37.16.3).
- **3 admission ABI tests**: Gate-12 vs ABI-recording semantics, execution_binding
  error ordering, pending-ticket rehydration. See integration-manifest.md §Remaining.

## Evidence (true-exit runner logs)

- c6_retained_input_recovery.json (exit 0)
- compile_check.json (exit 0, 496 passed)
- digest_collision.json (exit 0, 9 passed)
- runtime_full_lib.json (exit 0, 1934 passed)
- fjall_pending_action.json (exit 101, 3 remaining failures documented)

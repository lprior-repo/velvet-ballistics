# Proof Repair Guide: vb-engine-yaml

## Required Outcome

Return to proof review only when every required State 5 proof obligation has executable raw evidence or a valid replanned/waived obligation with owner, expiry, and compensating evidence. Summary-only proof-writer claims are not acceptable.

## Required Repairs

1. Repair `PO-013` Loom evidence.
   - Fix the `Arc` compile failures in `crates/vb_runtime/src/models/loom/timer_fired_cancel.rs` and `crates/vb_runtime/src/models/loom/shutdown_drain.rs` in an allowed source/model repair state.
   - Use the synchronization type intended by the Loom model semantics, then rerun `RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime bounded_queue`.
   - Record raw PASS output showing the command compiled and executed, not only that the import error disappeared.

2. Repair or replan `PO-011` and `PO-012` Kani evidence.
   - The current planned harness names do not exist. Either create the required harnesses or update the plan to existing exact harness names that cover the same obligation surface.
   - For `PO-011`, cover accessor, bytecode, constants, slots, node deduplication, idempotency, budget, expression-stack bounds, and divide-by-zero/resource-limit behavior.
   - For `PO-012`, cover raw IR rejection, dummy proof rejection, digest mismatch rejection, and missing capability gate rejection.
   - Record each harness command, unwind bound, exit status, and raw PASS output.

3. Strengthen `PO-005` ingress TLA obligation coverage.
   - Add protocol-kind cases for YAML, JSON, HTTP, text command, direct API, and binary IPC attempts.
   - Add typed outcome/diagnostic classes for unsupported protocol, artifact not accepted, and backpressure.
   - Keep bounded queue rejection evidence, but also prove unsupported runtime protocols cannot become accepted ingress.
   - Rerun `tlc -config verification/tla/EngineYamlIngress.cfg verification/tla/EngineYamlIngress.tla` and record raw output.

4. Repair the State 6 contract verification input.
   - Rerun contract-verification-review after the repaired contract, traceability matrix, and proof plan are on disk.
   - State 6 cannot approve while `.beads/vb-engine-yaml/contract-verification-review.md` remains a rejection artifact.

## Rerun Targets

- `tlc -config verification/tla/EngineYamlIngress.cfg verification/tla/EngineYamlIngress.tla`
- `RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime bounded_queue`
- Focused `cargo kani` commands for all `PO-011` and `PO-012` harnesses, with documented unwind bounds.
- `verus verification/verus/resource_budget.rs`
- `verus verification/verus/step_state_machine.rs`
- `verus verification/verus/recovery_verification.rs`
- `verus verification/verus/capability_artifact_model.rs`

## Approval Gate

Proof review may approve only after `proof-findings.jsonl` findings are resolved or explicitly replanned, `contract-verification-review.md` is clean, and all required proof-owner obligations have raw executable evidence.

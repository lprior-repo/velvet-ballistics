# Proof Plan Review Input: vb-core-ipc-loom-property

## Review Request
proof-reviewer must evaluate whether the proof strategy and obligation matrix adequately cover the concurrency risks for the 3 new IPC/orchestrator seams.

---

## Review Checklist

### Completeness
- [ ] All 4 new loom models have explicit obligation rows with commands and expected evidence
- [ ] All 5 existing loom models are confirmed and not silently dropped
- [ ] All 3 TLA+ obligations have model + config + invariant + command
- [ ] VERUS-FP-001 is scoped to FramePool capacity_invariant only (not full FramePool proof)
- [ ] INV-004 (write_buffer byte conservation) has both loom (LOOM-IPC-002) and TLA+ (TLA-IPC-002)

### Correctness
- [ ] Loom model for MemoryIngress models bounded channel behavior (not unbounded)
- [ ] FramePool loom model uses Arc<Mutex<FramePool>> (thread-safe variant)
- [ ] IPC server client-map loom model covers insert + remove + get (not just insert)
- [ ] Write buffer loom model covers fill + drain + WouldBlock (not just happy path)
- [ ] TLA+ state constraints are bounded (queued <= 4, Cardinality(active) <= 4, Len(buffer) <= 64)

### Blockers identified
- [ ] `crates/vb_ipc/src/models/loom/` is MISSING — proof-writer must create before running loom
- [ ] `vb_ipc` Cargo.toml lacks `loom = "0.7"` dev-dependency
- [ ] Are 3 new loom models sufficient, or does FramePool loom model also need to be created?

### Traceability
- [ ] Every obligation maps to a contract clause (INV-001 through INV-005, VB-CONC-001..005)
- [ ] Every contract clause with concurrency risk has a loom obligation
- [ ] Unmapped obligations: NONE expected

### Risk classification
- [ ] concurrency risk → loom (correct)
- [ ] backpressure risk → loom (correct)
- [ ] temporal risk → already covered by existing models (VB-CONC-002, VB-CONC-003); no new temporal modeling needed
- [ ] No unsafe UB risk → Miri not triggered (correct)
- [ ] Not fuzz/proptest territory → structural concurrency proofs not input-space tests (correct)

### Waivers
- [ ] No waivers claimed (all seams have verifier lanes) — confirm this is acceptable

---

## Obligation Matrix Summary

| ID | Clause | Verifier | Target | Required | Status |
|---|---|---|---|---|---|
| LOOM-MI-001 | INV-001 | loom | vb_ipc/models/loom/memory_ingress.rs | yes | planned |
| LOOM-FP-001 | INV-002 | loom | vb_runtime/models/loom/frame_pool.rs | yes | planned |
| LOOM-IPC-001 | INV-003 | loom | vb_ipc/models/loom/ipc_server_clients.rs | yes | planned |
| LOOM-IPC-002 | INV-004 | loom | vb_ipc/models/loom/write_buffer.rs | yes | planned |
| TLA-MI-001 | INV-001 | tla-plus | specs/MemoryIngressChannel.tla | no | planned |
| TLA-IPC-001 | INV-003 | tla-plus | specs/IpcServerClientMap.tla | no | planned |
| TLA-IPC-002 | INV-004 | tla-plus | specs/WriteBuffer.tla | no | planned |
| VERUS-FP-001 | INV-002 | verus | vb_runtime/frame_pool.rs | no | planned |
| EXISTING-001 | VB-CONC-001 | loom | vb_runtime/models/loom/journal_writer_queue.rs | yes | planned |
| EXISTING-002 | VB-CONC-002 | loom | vb_runtime/models/loom/action_completion_cancel.rs | yes | planned |
| EXISTING-003 | VB-CONC-003 | loom | vb_runtime/models/loom/timer_fired_cancel.rs | yes | planned |
| EXISTING-004 | VB-CONC-004 | loom | vb_runtime/models/loom/shutdown_drain.rs | yes | planned |
| EXISTING-005 | VB-CONC-005 | loom | vb_runtime/models/loom/bounded_queue.rs | yes | planned |

**Total: 13 obligations — 5 required (loom required), 8 optional**

---

## Key Open Questions for Reviewer

1. Is the `vb_ipc` loom directory creation + dev-dependency addition in scope for proof-writer, or does it require a separate bead?
2. Should the 5 existing loom models be re-run to confirm they still pass after the new models are added?
3. Does TLA-IPC-001 (IpcServerClientMap.tla) already exist, or does proof-writer need to create it?
4. Does TLA-IPC-002 (WriteBuffer.tla) already exist, or does proof-writer need to create it?
5. Is INV-004 (write_buffer) properly covered by LOOM-IPC-002 + TLA-IPC-002, or is there a gap?

---

## Files to Review
- `.beads/vb-core-ipc-loom-property/contract.md` — contract clauses
- `.beads/vb-core-ipc-loom-property/traceability-matrix.jsonl` — requirement mapping
- `.beads/vb-core-ipc-loom-property/proof-obligations.jsonl` — raw obligations
- `.beads/vb-core-ipc-loom-property/delivery-scope.jsonl` — scope with risk tags

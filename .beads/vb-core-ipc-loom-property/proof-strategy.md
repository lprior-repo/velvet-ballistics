# Proof Strategy: vb-core-ipc-loom-property

## State
State: 4 (Proof Planning)
Generated: by proof-planner skill

---

## Scope Summary

| Category | Count | Details |
|---|---|---|
| New loom models | 4 | MemoryIngress, FramePool (thread-safe), IPC client-map, write_buffer |
| Existing loom models (pass) | 5 | JournalWriterQueue, ActionTicket, TimerWheel, Shutdown, BoundedQueue |
| New TLA+ models | 1 | MemoryIngressChannel.tla (INV-001) |
| Existing TLA+ models | 2 | IpcServerClientMap.tla, WriteBuffer.tla |
| Verus obligations | 1 | FramePool capacity_invariant |

**Total proof obligations: 13 rows (4 new loom + 5 existing loom + 3 TLA+ + 1 Verus)**

---

## Risk Classification

| Risk Tag | Applies To | Primary Verifier | Rationale |
|---|---|---|---|
| concurrency | All 4 new loom models | loom | Interleaving exploration of concurrent take/submit/receive/insert/remove/drain |
| backpressure | MemoryIngress, FramePool | loom | Bounded channel capacity invariants under concurrent load |
| temporal | (covered by existing VB-CONC-002/003) | loom (existing) | Completion vs cancel race already modeled |

Concurrency is the dominant risk. Temporal risks for INV-002/INV-003 are already covered by existing loom models (VB-CONC-002, VB-CONC-003) — no new temporal modeling needed.

---

## Verifier Selection

### loom (primary — 4 new + 5 existing)
- **Why**: Concurrent interleavings for take/release, try_submit/try_recv, HashMap insert/remove, fill/drain
- **Bounded exploration**: Each model explores bounded permutations (max 3–6 concurrent operations)
- **Not unsafe UB**: No raw pointer/aliasing risk in these seams; Miri not triggered

### TLA+ (supplementary — 1 new + 2 existing)
- **TLA-MI-001**: Bounded channel formal invariant (queued <= CAPACITY) with TLC model checking
- **TLA-IPC-001**: IPC client-map cardinality bound + token uniqueness
- **TLA-IPC-002**: Write buffer byte conservation (Len(buffer) = written - drained)
- **Rationale**: TLA+ gives exhaustive state-space coverage for the formal invariant; complements loom's permutation exploration with a declarative spec

### Verus (specialized — 1)
- **VERUS-FP-001**: FramePool capacity_invariant proven via loop invariant in proof fn
- **Rationale**: Cheaper than exhaustive loom for a pure capacity bound; Rust-native

### Not applicable
- **Kani**: Bounded state space is already covered by loom; no unbounded arithmetic
- **Miri**: No unsafe code in these seams
- **Flux**: Verus covers the same refinement-type invariants at lower cost
- **fuzz/proptest**: These are structural concurrency proofs, not input-space tests

---

## New Loom Model Inventory

| # | Model File | Crate | Obligation | Invariant | Exploration Bound |
|---|---|---|---|---|---|
| 1 | `crates/vb_ipc/src/models/loom/memory_ingress.rs` | vb_ipc | LOOM-MI-001 | available <= capacity | 3 producers × 3 consumers × 3 rounds |
| 2 | `crates/vb_runtime/src/models/loom/frame_pool.rs` | vb_runtime | LOOM-FP-001 | available <= capacity | 3 takers × 3 releasers × 3 rounds |
| 3 | `crates/vb_ipc/src/models/loom/ipc_server_clients.rs` | vb_ipc | LOOM-IPC-001 | token uniqueness, active <= MAX_CLIENTS | 3 accepts × 3 removes × 3 rounds |
| 4 | `crates/vb_ipc/src/models/loom/write_buffer.rs` | vb_ipc | LOOM-IPC-002 | byte conservation | 3 fills × 3 drains × 3 rounds |

---

## Dependency Discovery Findings

### CRITICAL BLOCKERS (must fix before proof execution)
1. `crates/vb_ipc/src/models/loom/` directory is **MISSING** — all 3 new vb_ipc loom models need this directory
2. `vb_ipc` Cargo.toml lacks `loom = "0.7"` dev-dependency — loom tests in vb_ipc will not compile

### Blocking evidence
```
MISSING: crates/vb_ipc/src/models/loom/
vb_ipc/Cargo.toml: loom dev-dependency ABSENT (grep returned no match)
```

### Resolution plan
- proof-writer must create `crates/vb_ipc/src/models/loom/` with `mod.rs` + 3 model files
- proof-writer must add `loom = "0.7"` to `crates/vb_ipc/Cargo.toml` [dev-dependencies]

---

## Proof Execution Order

### Phase A — Unblock compilation (prerequisite)
1. Create `crates/vb_ipc/src/models/loom/` directory
2. Add `loom = "0.7"` to `crates/vb_ipc/Cargo.toml` [dev-dependencies]
3. Write `crates/vb_ipc/src/models/loom/mod.rs` with pub mod declarations
4. Write skeleton `memory_ingress.rs`, `ipc_server_clients.rs`, `write_buffer.rs` (compile-scaffold only; full model written by proof-writer)

### Phase B — New loom models (4)
1. LOOM-MI-001: `memory_ingress.rs` — bounded mpsc submit/receive
2. LOOM-FP-001: `frame_pool.rs` — take/release under Arc<Mutex<FramePool>>
3. LOOM-IPC-001: `ipc_server_clients.rs` — HashMap insert/remove token uniqueness
4. LOOM-IPC-002: `write_buffer.rs` — fill/drain byte conservation

### Phase C — Existing loom models (5)
5–9. EXISTING-001..005: Already pass; confirm still passing after changes

### Phase D — TLA+ (3)
10. TLA-MI-001: `specs/MemoryIngressChannel.tla`
11. TLA-IPC-001: `specs/IpcServerClientMap.tla` (may already exist)
12. TLA-IPC-002: `specs/WriteBuffer.tla` (may already exist)

### Phase E — Verus (1)
13. VERUS-FP-001: `crates/vb_runtime/src/frame_pool.rs` capacity invariant proof

---

## Evidence Criteria

| Obligation | Evidence Required | Pass Threshold |
|---|---|---|
| LOOM-MI-001 | loom model completes; 0 failures; invariant preserved | 0loom failures |
| LOOM-FP-001 | loom model completes; 0 failures; capacity invariant | 0loom failures |
| LOOM-IPC-001 | loom model completes; 0 failures; token uniqueness | 0loom failures |
| LOOM-IPC-002 | loom model completes; 0 failures; byte conservation | 0loom failures |
| EXISTING-001..005 | Confirmed pass from prior bead; re-run to confirm | prior evidence |
| TLA-MI-001 | TLC: no invariant violations, no deadlocks | 0violations |
| TLA-IPC-001 | TLC: no invariant violations, no deadlocks | 0violations |
| TLA-IPC-002 | TLC: no invariant violations, no deadlocks | 0violations |
| VERUS-FP-001 | Verus: 0 verifier errors | 0errors |

---

## Waiver Claims

None. All risk-tagged seams have a corresponding verifier lane.

---

## Assumptions

1. `crossbeam_channel::bounded` provides the concurrency boundary; loom models our usage surface only
2. FramePool loom model uses `Arc<Mutex<FramePool>>` to explore the intended thread-safe variant (production code uses `&mut self` which is not thread-safe)
3. IPC server client-map mutations happen only inside `poll_once` critical section
4. TLA+ state constraints (queued <= 4, Cardinality(active) <= 4, Len(buffer) <= 64) bound the exploration to small finite models
5. loom max explores 3×3×3 permutations per model to keep runtime tractable

---

## Follow-up Triggers

| Trigger | Condition | Follow-up |
|---|---|---|
| FramePool production code changes | Arc<Mutex<>> variant replaces `&mut self` | Re-run LOOM-FP-001 |
| IPC server dispatch changes | poll_once critical section expands | Re-run LOOM-IPC-001 |
| crossbeam-channel upgrade | Version bump of crossbeam-channel | Re-run LOOM-MI-001, TLA-MI-001 |

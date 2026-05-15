# Assurance Bundle: vb-core-ipc-loom-property

bead_id: vb-core-ipc-loom-property
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /tmp/vb-ws/vb-core-ipc-loom-property
commit_or_change: HEAD (main, origin/main)

---

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|-------------|-----------------|---------------------|-----------------|--------|
| MemoryIngress backpressure envelope | INV-001 | LOOM-MI-001 (11 tests PASS) | proof-review.md APPROVED, test-suite-review.md APPROVED | PASS |
| FramePool capacity bound | INV-002 | LOOM-FP-001 (4 tests PASS) | proof-review.md APPROVED, test-suite-review.md APPROVED | PASS |
| IPC client map token uniqueness | INV-003 | LOOM-IPC-001 (4 tests PASS) | proof-review.md APPROVED, test-suite-review.md APPROVED | PASS |
| write_buffer byte conservation | INV-004 | LOOM-IPC-002 (4 tests PASS) | proof-review.md APPROVED, test-suite-review.md APPROVED | PASS |
| VB-CONC-001 journal writer queue | VB-CONC-001 | EXISTING-001 PASS | proof-review.md APPROVED | PASS |
| VB-CONC-002 action completion/cancel | VB-CONC-002 | EXISTING-002 PASS | proof-review.md APPROVED | PASS |
| VB-CONC-003 timer fired/cancel | VB-CONC-003 | EXISTING-003 PASS | proof-review.md APPROVED | PASS |
| VB-CONC-004 shutdown drain | VB-CONC-004 | EXISTING-004 PASS | proof-review.md APPROVED | PASS |
| VB-CONC-005 bounded queue | VB-CONC-005 | EXISTING-005 PASS | proof-review.md APPROVED | PASS |

---

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|------------|------|---------|----------|--------|--------|
| LOOM-MI-001 | loom | `RUSTFLAGS="--cfg loom" cargo test -p vb_ipc memory_ingress -- --nocapture` | `crates/vb_ipc/src/models/loom/memory_ingress.rs` | PASS | — |
| LOOM-FP-001 | loom | `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime models::loom::frame_pool -- --test-threads=1` | `crates/vb_runtime/src/models/loom/frame_pool.rs` | PASS | — |
| LOOM-IPC-001 | loom | `RUSTFLAGS="--cfg loom" cargo test -p vb_ipc ipc_server_clients -- --nocapture` | `crates/vb_ipc/src/models/loom/ipc_server_clients.rs` | PASS | — |
| LOOM-IPC-002 | loom | `RUSTFLAGS="--cfg loom" cargo test -p vb_ipc write_buffer -- --nocapture` | `crates/vb_ipc/src/models/loom/write_buffer.rs` | PASS | — |
| EXISTING-001 | loom | `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime models::loom::journal_writer_queue -- --test-threads=1` | `crates/vb_runtime/src/models/loom/journal_writer_queue.rs` | PASS | — |
| EXISTING-002 | loom | `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime models::loom::action_completion_cancel -- --test-threads=1` | `crates/vb_runtime/src/models/loom/action_completion_cancel.rs` | PASS | — |
| EXISTING-003 | loom | `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime models::loom::timer_fired_cancel -- --test-threads=1` | `crates/vb_runtime/src/models/loom/timer_fired_cancel.rs` | PASS | — |
| EXISTING-004 | loom | `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime models::loom::shutdown_drain -- --test-threads=1` | `crates/vb_runtime/src/models/loom/shutdown_drain.rs` | PASS | — |
| EXISTING-005 | loom | `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime models::loom::bounded_queue -- --test-threads=1` | `crates/vb_runtime/src/models/loom/bounded_queue.rs` | PASS | — |
| TLA-MI-001 | tla-plus | `cd specs && tlc -config MemoryIngressChannel.cfg MemoryIngressChannel.tla` | `specs/MemoryIngressChannel.tla` | DEFERRED_GLOBAL | Out of scope per contract non-goals |
| TLA-IPC-001 | tla-plus | `cd specs && tlc -config IpcServerClientMap.cfg IpcServerClientMap.tla` | `specs/IpcServerClientMap.tla` | DEFERRED_GLOBAL | Out of scope per contract non-goals |
| TLA-IPC-002 | tla-plus | `cd specs && tlc -config WriteBuffer.cfg WriteBuffer.tla` | `specs/WriteBuffer.tla` | DEFERRED_GLOBAL | Out of scope per contract non-goals |
| VERUS-FP-001 | verus | `moon run :verify-proof 2>&1 | grep -E '(FRAME|frame_pool|verified|error)'` | `crates/vb_runtime/src/models/loom/frame_pool.rs` | DEFERRED_GLOBAL | Out of scope per contract non-goals |

---

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|-----------|---------|----------|--------|
| `memory_ingress_invariants` | `RUSTFLAGS="--cfg loom" cargo test -p vb_ipc memory_ingress -- --nocapture` | `crates/vb_ipc/src/models/loom/memory_ingress.rs` | PASS |
| `frame_pool_basic`, `frame_pool_capacity_boundary` | `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime models::loom::frame_pool -- --test-threads=1` | `crates/vb_runtime/src/models/loom/frame_pool.rs` | PASS |
| `ipc_server_clients_basic/concurrent_accepts/capacity_preserved/rapid_cycles` | `RUSTFLAGS="--cfg loom" cargo test -p vb_ipc ipc_server_clients -- --nocapture` | `crates/vb_ipc/src/models/loom/ipc_server_clients.rs` | PASS |
| `write_buffer_basic/concurrent/would_block/capacity_respected` | `RUSTFLAGS="--cfg loom" cargo test -p vb_ipc write_buffer -- --nocapture` | `crates/vb_ipc/src/models/loom/write_buffer.rs` | PASS |
| `journal_writer_queue_invariants` | loom test | `crates/vb_runtime/src/models/loom/journal_writer_queue.rs` | PASS |
| `action_completion_cancel_concurrent/race` | loom test | `crates/vb_runtime/src/models/loom/action_completion_cancel.rs` | PASS |
| `timer_fired_cancel_ordering` | loom test | `crates/vb_runtime/src/models/loom/timer_fired_cancel.rs` | PASS |
| `shutdown_drain_ordering` | loom test | `crates/vb_runtime/src/models/loom/shutdown_drain.rs` | PASS |
| `bounded_queue_invariants/multiple_operations` | loom test | `crates/vb_runtime/src/models/loom/bounded_queue.rs` | PASS |
| Formal verification gate | `moon ci` or canonical CI | `verification-ledger.jsonl` | PASS (9 PASS, 4 DEFERRED_GLOBAL) |

---

## Review Evidence

| Review | Artifact | Status | Findings |
|--------|----------|--------|----------|
| Proof review | `proof-review.md` | APPROVED | CAS retry verified, thread counts correct, loom model structure sound |
| Contract verification | `contract-verification-review.md` | APPROVED | All 5 invariants covered, contract clauses match tests |
| Test suite review | `test-suite-review.md` | APPROVED | 418 tests pass, CAS retry pattern verified, thread counts 2+2 (not 3+3, not a defect) |
| Black-hat review | `black-hat-review.md` | APPROVED | No defects found, 9 loom obligations PASS, 3 producers exercised |
| Formal verification | `formal-verification-report.md` | APPROVED | 9 required PASS, 4 DEFERRED_GLOBAL non-blocking |

---

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|------|--------|-------|------------------|-----------------------|
| TLA-MI-001 | TLA+ spec exists; TLC execution out of scope per contract non-goals | vb-core-ipc-sync-evidence bead | Follow-up bead | Loom model covers INV-001; TLA+ deferred to sync-evidence bead |
| TLA-IPC-001 | TLC execution out of scope per contract non-goals | vb-core-ipc-sync-evidence bead | Follow-up bead | Loom model covers INV-003; TLA+ deferred to sync-evidence bead |
| TLA-IPC-002 | TLC execution out of scope per contract non-goals | vb-core-ipc-sync-evidence bead | Follow-up bead | Loom model covers INV-004; TLA+ deferred to sync-evidence bead |
| VERUS-FP-001 | Verus proof not executed in this bead | Future proof bead | Follow-up bead | Loom model covers INV-002; Verus deferred to future proof bead |

---

## Truth Serum Audit

- report: `.beads/vb-core-ipc-loom-property/truth-serum-report.md`
- status: APPROVED (see final-evidence-decision.md)

# Round 4 Agent A1 — Section 50 ArrayQueue LETHAL Sweep

**Reviewer:** black-hat-reviewer
**Bead:** N/A (adversarial re-review of Round 3 finding)
**State:** Round 3 follow-up
**Source checkout:** /home/lewis/src/velvet-ballistics
**Attempt:** 4 (re-verification)

## Gate Result: STATUS: REJECTED

Both LETHALs are CONFIRMED. A third LETHAL was found. The scanner does not catch any of them. Bead tracking is absent.

---

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|-------------|--------|----------|
| Section 50: ArrayQueue required for IPC | FAIL | crates/vb_ipc/src/ingress.rs:5,77,117 uses crossbeam_channel |
| Section 50/20: lock-free bounded queue for hot-path completion queues | FAIL | crates/vb_runtime/src/action_queue/queue.rs:19,38 uses std::sync::Mutex<VecDeque> |
| Section 50/20: ArrayQueue/rtrb for queues | FAIL | crates/vb_runtime/src/action_queue/queue.rs:35 uses std::sync::mpsc::sync_channel |
| MAJOR-1 bead exists and tracks migration | FAIL | bd search MAJOR-1 returns "No issues found"; no open or in-progress bead |
| Tests assert ArrayQueue backend | FAIL | crates/vb_ipc/src/queue/tests/array_queue_tests.rs tests the public API only; no backend introspection. A migration to nothing-but-ArrayQueue would NOT be caught by the test suite. |
| Scanner catches crossbeam_channel::bounded( | FAIL | scripts/check-hot-cold-forbidden-apis.rs:87 only matches crossbeam_channel::unbounded(; bounded slips through |
| Scanner catches std::sync::mpsc::sync_channel( | FAIL | Same regex set; sync_channel is not in the deny list |
| Scanner catches std::sync::Mutex<VecDeque> | FAIL | No rule for this pattern at all |

## PHASE 5: The Bitter Truth

Two patterns of carelessness:

1. **The "test the public surface" cop-out.** array_queue_tests.rs is 913 lines of public-API tests that prove the behavior of MemoryIngress is correct. They prove nothing about the backend. Someone could delete the ArrayQueue reference comment, keep crossbeam_channel forever, and the test suite would still pass. This is a behavior test masquerading as a contract test.

2. **The scanner that the team trusts is theater.** check-hot-cold-forbidden-apis.rs:83-88 includes a CHANNEL-UNBOUNDED-001 rule. The name implies "we catch unbounded channel usage." It does not. It catches the only one of the four forbidden patterns that is so obviously broken that no production code uses it.

## Findings

### FINDING-1: LETHAL — crossbeam_channel::bounded in IPC MemoryIngress (CRITICAL)
- File:line: crates/vb_ipc/src/ingress.rs:5,77,117
- Section 50 violation. The exact implementation the spec says to remove.
- User-visible impact: Lock contention under 256 concurrent Unix-socket producers; consumer thread blocks on mutex.
- Required fix: Replace Sender/Receiver field pair in MemoryIngress with ArrayQueue (MPMC) or rtrb::RingBuffer (SPSC). Drop crossbeam-channel from vb_ipc/Cargo.toml:11.

### FINDING-2: LETHAL — Mutex<VecDeque> in BoundedActionCompletionQueue (CRITICAL)
- File:line: crates/vb_runtime/src/action_queue/queue.rs:3,19,38, types.rs:66,73
- Section 20/50 backend violation.
- The module's own doc-comment (action_queue.rs:9-10) advertises this as the LETHAL-5 fix for Section 4's bounded action-completion-queue requirement.
- User-visible impact: Lock contention on the action completion hot path; shard tick stalls waiting for action workers.
- Required fix: Mirror the ShardCommandQueue pattern at shard/queue.rs:34-120.

### FINDING-3: LETHAL — std::sync::mpsc::sync_channel for backpressure (HIGH)
- File:line: crates/vb_runtime/src/action_queue/queue.rs:35
- The backpressure channel is std::sync::mpsc, not ArrayQueue/rtrb.
- User-visible impact: Backpressure warnings block enqueue path.

### FINDING-4: Scanner gap: bounded channel variants and Mutex<VecDeque> not in deny list (HIGH)
- File:line: scripts/check-hot-cold-forbidden-apis.rs:83-88
- Only matches unbounded variants. The actual production violations slip through.

### FINDING-5: Test suite has no backend-introspection assertion (HIGH)
- File:line: crates/vb_ipc/src/queue/tests/array_queue_tests.rs
- Tests prove behavior, not backend identity. A future "migration PR" that keeps crossbeam_channel would land cleanly.

### FINDING-6: No bead tracks MAJOR-1 (MEDIUM)
- bd search MAJOR-1: no matches. The 8 BIG-ASS-TESTING-TO-FIX.md Section 50 callouts have no routed work.

### FINDING-7: Documentation debt codified the violation rather than routing it (MEDIUM)
- BIG-ASS-TESTING-TO-FIX.md:37,76,104,118,137,179,205,223 — 8 individual callouts
- Every agent that reads the repo to understand what to fix will read the same memo. Without a bead, the broken thing still ships.

## Verdict: SHIP-BLOCKER

**Severity: 100/100.**

### Required Repair Actions

1. **CRITICAL FINDING-1**: Replace crossbeam_channel in crates/vb_ipc/src/ingress.rs with ArrayQueue (MPMC) or rtrb::RingBuffer (SPSC). Drop crossbeam-channel from vb_ipc/Cargo.toml:11.

2. **CRITICAL FINDING-2**: Replace std::sync::Mutex<VecDeque<ActionTicket>> in crates/vb_runtime/src/action_queue/ with ArrayQueue<ActionTicket>. Re-run kani_action_queue_* harnesses.

3. **HIGH FINDING-3**: Replace std::sync::mpsc::sync_channel in BoundedActionCompletionQueue::with_backpressure with ArrayQueue<BackpressureWarning> (or delete the API).

4. **HIGH FINDING-4**: Extend scripts/check-hot-cold-forbidden-apis.rs:83-88 to deny crossbeam_channel::bounded(, std::sync::mpsc::sync_channel(, and the Mutex<VecDeque< / Mutex<Vec< pattern.

5. **HIGH FINDING-5**: Add a memory_ingress_uses_arrayqueue_backend contract test that fails if the backend is not ArrayQueue.

6. **MEDIUM FINDING-6**: File three beads: vb-ipc-arrayqueue-migration, vb-actionqueue-arrayqueue-migration, vb-hot-cold-scanner-coverage. Claim them.

7. **MEDIUM FINDING-7**: Once the three beads are filed, prune the eight BIG-ASS-TESTING-TO-FIX.md Section 50 callouts.

## Final Verdict: SHIP-BLOCKER. This is not acceptable as debt.

# Proof Strategy — vb-hn4sc

## State: 4 (Proof Planning)
## Bead: vb-hn4sc — Storage: enforce byte-budget limits in queued group commits (P1)

---

## 1. Scope

**Bead:** vb-hn4sc — Enforce byte-budget at `JournalWriterQueue::flush_batch`.
**Verifier lanes:** `rust-local`, `persistence`, `kani`, `proptest` (length roundtrip).
**Primary crate:** `vb_storage` (`crates/vb_storage/src/queue/writer.rs`,
`crates/vb_storage/src/queue/writer/stage.rs`, `crates/vb_storage/src/types.rs`,
`crates/vb_storage/src/error/{mod,codes}.rs`).
**Sister type (parity target):** `JournalWriteBatch::append_event` at
`crates/vb_storage/src/batch/append_event.rs:86-102`.

This bead fills a documented P1 gap: `_limits: StorageLimits` at
`writer.rs:54` is currently ignored. After this change, `flush_batch`
enforces a per-flush encoded-byte budget BEFORE `owned_batch.commit()`,
returning `Err(JournalError::JournalBatchBytesExceeded { attempted, limit })`
(reused — no new variant) when the next staged event would push
accumulated bytes over the configured budget.

---

## 2. Hard Constraints (from contract.md §1 AC-1.6 + T-HN4SC-7)

- **No new `JournalError` variant.** Reuse `JournalBatchBytesExceeded { attempted: u64, limit: u64 }`.
  Diagnostic code `0x4022` (`JOURNAL_BATCH_BYTES_EXCEEDED`) and display
  string `"journal batch byte budget exceeded: attempted {attempted} > limit {limit}"`
  are preserved verbatim.
- **No widening of `RuntimeError`.** The contract asserts that the typed
  `JournalError` is the wire signal regardless of how `RuntimeError`
  classifies it (deferred to `proof-to-implementation`, OI-1).
- **Compile-time const assertion required.** A `const _: () = { ... }`
  block must bind `StorageLimits::DEFAULT.max_journal_batch_bytes ==
  DEFAULT_JOURNAL_BATCH_BYTES_INCLUSIVE_OF_HEADER (1_048_636)` and
  `== DEFAULT_JOURNAL_BATCH_BYTE_LIMIT + RECORD_HEADER_BYTES`.
- **No `unsafe`, no `unwrap`, no `expect`, no `panic`, no `todo`, no `unimplemented`, no `dbg!`.**
- **Kani harness must use `kani::any()`.** No hardcoded shapes
  (GOD RULE 1 — see `proof-writer` and the `kani` skill).

---

## 3. Verifier Lane Strategy

Four verifier lanes are mandated by the bead. Each lane owns specific
proof obligations, the production code it binds to, and a documented
evidence command. Default-profile verifiers (Verus, Flux, Loom, Miri,
cargo-fuzz, TLA+) are explicitly marked `not_applicable` per seed with
concrete evidence references — no silent omission.

### 3.1 Lane: `rust-local`

**Scope:** Rust-local invariants that compile and code-review can prove:
- `StorageLimits::DEFAULT.max_journal_batch_bytes` const-equals `1_048_636`
  (compile-time const assertion in `types.rs`).
- Parity: `JournalWriteBatch::append_event` and
  `JournalWriterQueue::flush_batch` emit the SAME error variant, the
  SAME diagnostic code (`0x4022`), and the SAME symbolic code
  (`JOURNAL_BATCH_BYTES_EXCEEDED`) for the SAME oversize event.
- Enqueue path does NOT enforce byte budget (negative-space claim).
- DuplicateStagedKey and DuplicateEvent guard precedence over the byte
  gate (existing tests must continue to pass).
- `drain_all` short-circuits on first `JournalBatchBytesExceeded`.
- Default budget accommodates at least one max-size event
  (`1_048_636 == RECORD_HEADER_BYTES + MAX_JOURNAL_EVENT_PAYLOAD_BYTES`).

**Production binding:** `crates/vb_storage/src/types.rs`,
`crates/vb_storage/src/queue/writer.rs`,
`crates/vb_storage/src/batch/append_event.rs`.

### 3.2 Lane: `persistence`

**Scope:** Persistence-layer invariants involving `FjallJournal` /
`OwnedWriteBatch` / commit ordering:
- The gate fires AFTER `staged_keys_unique` and `durable_key_unique`
  checks and BEFORE `owned_batch.insert` and `owned_batch.commit()`
  (atomicity — partial prefix is forbidden per master §49).
- The byte accumulator is a stack-local `u64` reset to `0` at every
  `flush_batch` entry; NOT a field on `JournalWriterQueueState`.
- `drain_all` calls `flush_batch` in a bounded loop and propagates the
  first byte-budget error.

**Production binding:** `crates/vb_storage/src/queue/writer.rs:152-231`
(`flush_batch` body), `crates/vb_storage/src/queue/writer/stage.rs`
(`stage_queued_event`).

### 3.3 Lane: `kani`

**Scope:** Bounded model checking of the pure `gate_decision` predicate
in `crates/vb_storage/src/kani_vb_vzcuf_ps010.rs` (NEW).

The harness MUST use `kani::any()` with explicit `kani::assume(...)` bounds
on `accumulator`, `next`, and `limit`, and MUST assert:
1. `accumulator.checked_add(next) == None ⟹ GateDecision::Reject { attempted: u64::MAX, limit }`.
2. `accumulator + next <= limit ⟹ GateDecision::Accept { new_accumulated: accumulator + next }`.
3. `accumulator + next > limit ⟹ GateDecision::Reject { attempted: accumulator + next, limit }`.
4. Exact fit (`accumulator + next == limit`) ⟹ `GateDecision::Accept`.
5. Default budget accommodates at least one max-size encoded event
   (`1_048_636 == 60 + 1_048_576`).

**Production binding:** the harness exercises the `checked_add`-safe
arithmetic that production `gate_decision` will replicate, with the
same overflow sentinel (`u64::MAX`) used by
`JournalWriteBatch::append_event:86-102`. The `GateDecision` enum and
`EncodedRecordLength` / `AccumulatedFlushBytes` newtypes bind
structurally to the type contracts in `type-contracts.md §1-4`.

**Wiring:** registered behind the `kani-vb-vzcuf` feature gate in
`crates/vb_storage/src/lib.rs:76-94` (PS-001..PS-009 slot) — the
proof-writer adds `kani_vb_vzcuf_ps010` next to those.

### 3.4 Lane: `proptest` (length roundtrip)

**Scope:** Property test that the `encode_record` length roundtrips
through the queued path's byte accounting. Specifically:
- For any `JournalEvent` accepted by `encode_record`, the encoded
  `Vec<u8>.len()` (`value.len()` at `stage.rs:61-67`) equals the byte
  count the gate consumes (`EncodedRecordLength::new(value.len() as u64)`).
- The byte basis is the FULL encoded length (60-byte header + payload),
  matching `JournalWriteBatch::append_event:89` (parity basis).

This is a **length roundtrip** property, not a fuzz target. Inputs are
generated by `proptest::any::<JournalEvent>()` filtered to events whose
encoded length is `<= MAX_ENCODED_RECORD_BYTES`.

**Production binding:** `crates/vb_storage/src/codec.rs` (`encode_record`)
and the gate's `EncodedRecordLength::new` smart constructor at the
production site in `crates/vb_storage/src/queue/writer.rs`.

---

## 4. Default-Profile Verifiers: Explicit Non-Applicability

Per the proof-planner skill, every demanded verifier lane must receive
an explicit decision. The following default-profile verifiers are
explicitly marked `not_applicable` for this bead with concrete
evidence:

| Verifier | Reason | Evidence |
|---|---|---|
| `verus` | The queued-path gate is implemented as a pure newtype predicate over `u64`. Verus would require `#[spec]`/`#[proof]` annotations on the production source; the contract explicitly defers Verus work for the queued path (`codebase-map.md §116`). Existing Verus coverage of `JournalWriteBatch` (`vb-vzcuf` PS-006/PS-007) is preserved. | `.beads/vb-vzcuf/contract.md` notes Verus is reserved for the direct-batch path; `codebase-map.md §116` "writer_contract.rs Verus route — only touched if proof-plan includes a Verus spec for the new byte-budget gate (currently no Verus spec covers the queued path)". |
| `flux-rs` | The `gate_decision` predicate is a 3-line pure function over `u64` with no refinement types; Flux RS has no production binding target here. Existing Flux RS coverage of `vb-vzcuf` is preserved (PS-005/PS-008). | `codebase-map.md §116` confirms no Flux spec covers the queued path; `vb-hn4sc` does not introduce refinement types beyond newtypes over `u64`. |
| `loom` | The byte accumulator is a stack-local `u64` reset to `0` at every `flush_batch` entry; no new mutex is introduced (W-HN4SC-5). The existing `vb_runtime::models::loom::journal_writer_queue.rs` mock (lines 14-130) is unrelated to `vb_storage::queue::JournalWriterQueue`. Loom model for the queued-byte path is explicitly out of scope per contract.md OI-3. | `codebase-map.md §144` "A real Loom schedule-exploration harness for the new byte accumulator is OUT OF SCOPE for this bead"; `type-contracts.md §2.2` "byte_budget is immutable post-construction; no public setter exists". |
| `miri` | No `unsafe` is introduced (T-HN4SC-10). The touched files use `#![forbid(unsafe_code)]` at the module level. | `crates/vb_storage/src/error/mod.rs:1` and `crates/vb_storage/src/batch/types.rs:1` both have `#![forbid(unsafe_code)]`; `type-contracts.md §180` "No unsafe anywhere in the new types or gate". |
| `cargo-fuzz` | The gate's pure predicate has a bounded input space (`u64 × u64 × u64`) that Kani covers exactly (PS-010). The queued path consumes `JournalEvent` from a typed constructor, not from untrusted bytes. Fuzz is reserved for parser/codec hostile-byte boundaries (PS-005 fuzz target). | `codebase-map.md §84` "no kani/proptest harness covers the queued path byte budget"; `vb-vzcuf` fuzz lane covers parser/codec per existing evidence. |
| `tla-plus` | The proof-planner skill explicitly removes TLA+; temporal workflows are covered by loom + proptest. This bead has no temporal workflow surface — `flush_batch` is a synchronous call. | proof-planner skill SKILL.md "TLA+ removed. The temporal-workflow shape uses loom + proptest. There is no `tla-plus` verifier lane." |

---

## 5. Risk Stratification

| Risk tag | Lanes | Notes |
|---|---|---|
| `persistence` | `persistence` (POB-vb-hn4sc-005) | Atomicity of `OwnedWriteBatch.commit` ordering. |
| `public-api` | `rust-local` (POB-vb-hn4sc-003, POB-vb-hn4sc-004) | `StorageLimits::DEFAULT` field addition + reused error variant. |
| `contract-parity` | `rust-local` (POB-vb-hn4sc-004), `proptest` (POB-vb-hn4sc-002) | Direct batch path vs. queued path emit same error variant + same byte basis. |
| `arithmetic` | `kani` (POB-vb-hn4sc-001) | `checked_add` overflow and exact-fit boundary. |
| `concurrency` | `persistence` (POB-vb-hn4sc-005) | Byte accumulator must NOT be a shared field; existing mutex covers all state mutation. |
| `error-classification` | `rust-local` (POB-vb-hn4sc-004) | Diagnostic code `0x4022` reused; no new variant. |
| `migration` | `rust-local` (POB-vb-hn4sc-003) | Default `1_048_636` accommodates at least one max-size event (matching `kani_vb_vzcuf_ps007::check_bridge_accommodates_single_event`). |
| `release-critical` | `rust-local` (POB-vb-hn4sc-003), `persistence` (POB-vb-hn4sc-005) | P1 bug — production callers must keep working. |

---

## 6. Kani Harness Wiring (MANDATORY)

The new harness is `crates/vb_storage/src/kani_vb_vzcuf_ps010.rs`. It is
wired into `crates/vb_storage/src/lib.rs` behind the existing
`#[cfg(all(kani, feature = "kani-vb-vzcuf"))]` block (lines 76-94), and
registered in `crates/vb_storage/Cargo.toml` via the existing
`kani-vb-vzcuf = []` feature flag (line 29).

Run command (after proof-writer authors the harness):
```bash
cargo kani -p vb_storage --features kani-vb-vzcuf \
  --harness 'kani_vb_vzcuf_ps010::check_queued_byte_budget_invariants'
```

Harness ID convention: `kani_vb_vzcuf_ps010::check_queued_byte_budget_invariants`.

---

## 7. Compile-Time Const Assertion Plan (rust-local)

The const block at the bottom of `crates/vb_storage/src/types.rs`:

```text
const STORAGE_LIMITS_DEFAULT_BATCH_BYTES_BOUND: () = {
    assert!(
        StorageLimits::DEFAULT.max_journal_batch_bytes
            == crate::storage_constants::DEFAULT_JOURNAL_BATCH_BYTES_INCLUSIVE_OF_HEADER
    );
    assert!(
        crate::storage_constants::DEFAULT_JOURNAL_BATCH_BYTES_INCLUSIVE_OF_HEADER
            == crate::batch::types::DEFAULT_JOURNAL_BATCH_BYTE_LIMIT
                + crate::constants::RECORD_HEADER_BYTES
    );
    assert!(
        StorageLimits::DEFAULT.max_journal_event_payload_bytes
            == crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES
    );
};
```

This binds the storage default to the existing `batch/types.rs`
constant (`DEFAULT_JOURNAL_BATCH_BYTE_LIMIT = 1_048_576`) and the new
encoded-basis constant (`DEFAULT_JOURNAL_BATCH_BYTES_INCLUSIVE_OF_HEADER
= 1_048_636`), so a future drift is caught at compile time.

---

## 8. Obligation Set Summary

Six obligations (within the 5-6 range mandated by the bead) cover the
four lanes and the contract clauses that demand proof evidence:

| Obligation ID | Lane | Seeds | Contract clauses |
|---|---|---|---|
| POB-vb-hn4sc-001 | kani | ps-001, ps-003, ps-010, ps-013 | R-HN4SC-1, W-HN4SC-4, GROUP-COMMIT-BYTE-GATE-1/4 |
| POB-vb-hn4sc-002 | proptest | ps-002, ps-013 | R-HN4SC-1, GROUP-COMMIT-BYTE-GATE-2 |
| POB-vb-hn4sc-003 | rust-local | ps-005, ps-015, ps-014 | R-HN4SC-1, T-HN4SC-7, AC-1.4, GROUP-COMMIT-BYTE-GATE-7 |
| POB-vb-hn4sc-004 | rust-local | ps-004, ps-012, ps-014 | R-HN4SC-1, E-HN4SC-1..7, AC-1.3, AC-1.6 |
| POB-vb-hn4sc-005 | persistence | ps-001, ps-006, ps-011, ps-007 | R-HN4SC-1, W-HN4SC-1/2/3/5/6/8/9 |
| POB-vb-hn4sc-006 | rust-local | ps-008 | R-HN4SC-1, W-HN4SC-5, GROUP-COMMIT-BYTE-GATE-5 |

---

## 9. Handoff

- **State 4b** — `proof-plan-reviewer` dispositions each lane decision.
- **State 5** — `proof-writer` authors the Kani harness at
  `crates/vb_storage/src/kani_vb_vzcuf_ps010.rs`, registers it in
  `crates/vb_storage/src/lib.rs` and `.moon/tasks/kani.yml`.
- **State 7** — `proof-to-implementation` produces the bridge map.
- **State 12** — `formal-verifier` executes and closes the ledger.

Reviewer owns disposition; formal verifier owns closure. This plan
identifies obligations only.
# Theorem Kernel Projection — vb-qi37.16.4

## Boundary
- **TLA+-owned temporal model:** `AskAnswerLifecycle` (lifecycle state transitions, journal replay determinism, no duplicate answers, monotonic seqno) — see `tla-spec.md`
- **Verus-owned Rust core:** All pure/core Rust invariants for the answer command — see below
- **Theorem-owned kernel:** None required — Verus is sufficient for all Rust-local pure critical behavior
- **Rust/runtime shell:** File I/O, Fjall storage writes, IPC frame encoding — handled by integration tests and Kani
- **External systems excluded from formal proof:** Fjall persistence layer, CLI argument parsing

---

## Verus-Owned Clauses (No Theorem Kernel Needed)

All Rust-local pure/core critical behavior for this bead is expressible in Verus:

| Contract Clause | Verus Target | Property |
|----------------|--------------|----------|
| INV-002 | `Shard::handle_ask_answer` | `SlotWritten` value taint is checked before write; `Secret`-tainted values are rejected unless workflow allows it |
| PRE-004 | `AskTicket::eq` | All fields of `AskTicket` (run, step, seq, action, attempt, idempotency_key) are compared for equality deterministically |
| PRE-005 | `duplicate_check` spec fn | No `(run_id, step, seq)` ticket exists in answered set before journal append |
| PRE-003 | `check_payload_size` spec fn | `value_file` bytes length <= `max_ipc_payload_bytes` from `ResourceContract` |

**Rationale for no Lean/Aeneas/Hax:** The taint lattice (`Clean < DerivedFromSecret < Secret`) is a three-element partial order that is directly expressible as a Verus `spec fn` with explicit lattice ordering lemmas. The `AskTicket` equality proof is straightforward structural equality on `u64`/`u16` fields. No algebraic protocol lattice, no extraction to a proof assistant, no arithmetic bounds beyond what Kani covers — therefore no theorem kernel projection to Lean/Aeneas/Hax is warranted.

---

## Lean/Aeneas/Hax Obligations
None — waived in favor of Verus for all Rust-local pure clauses.

### Waiver Record
- **Clause:** All Rust-local pure clauses (INV-002, PRE-003, PRE-004, PRE-005)
- **Reason:** Taint lattice, ticket equality, size bounds, and duplicate detection are all expressible as Verus `spec fn`, `proof fn`, and `requires/ensures` contracts. No algebraic extraction or theorem-prover advantage justifies the Lean/Aeneas/Hax overhead.
- **Owner:** State 3 (contract synthesis)
- **Expiry:** Never — revisited if future contract clauses introduce a protocol lattice or algebraic state machine that exceeds Verus expressiveness
- **Compensating evidence:** Kani for bounded model checking of arithmetic bounds; integration tests for runtime behavior; TLA+ for lifecycle state machine

---

## Non-goals
- Taint propagation across multi-step workflows (separate bead scope)
- Algebraic semantics of the full `Taint` lattice beyond the three-level lattice used by ask-answer (separate bead scope)
- Fjall storage correctness (storage integration tests)

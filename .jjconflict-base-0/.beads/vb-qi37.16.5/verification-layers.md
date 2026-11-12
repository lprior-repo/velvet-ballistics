# Verification Layers

## Boundary

- **Verus-owned kernel**: Rust-local pure typestate invariants in `lifecycle.rs`, command validation preconditions in `storage.rs`, journal event construction in `journal.rs`
- **TLA+ temporal model**: Lifecycle state machine transitions, append-only journal semantics, replay correctness, invalid/duplicate/stale rejection
- **Theorem projection**: None (see `lean-contract.md`)
- **Runtime shell**: CLI args.rs dispatch, storage I/O (vb_storage), async runtime — excluded from formal proof
- **External systems excluded**: Storage backend (abstracted as journal interface), OS I/O

## Layer Assignment

| Clause | Primary Layer | Secondary/Defense Layer |
|--------|---------------|------------------------|
| INV-001 (single canonical state) | verus | tla-plus |
| INV-002 (append-only journal) | tla-plus | verus postconditions on append_event |
| INV-003 (valid transitions) | tla-plus | verus preconditions on transition fn |
| INV-004 (replay bit-identical) | tla-plus | integration + replay test |
| PRE-001 (storage connected) | integration | manual-qa |
| PRE-002 (command validation) | verus | tla-plus |
| PRE-003 (clean snapshot) | integration | tla-plus |
| POST-001 (exactly-one event) | verus | tla-plus + integration |
| POST-002 (replay fidelity) | tla-plus | integration + replay test |
| POST-003 (invalid-transition error) | verus + tla-plus | integration |
| POST-004 (duplicate rejection) | verus + tla-plus | integration |
| POST-005 (stale rejection) | verus + tla-plus | integration |
| ERR::InvalidTransition | integration + tla-plus | Fowler scenario |
| ERR::DuplicateRequest | integration + tla-plus | Fowler scenario |
| ERR::StaleRequest | integration + tla-plus | Fowler scenario |
| ERR::JournalWriteFailure | integration | manual-qa |
| ERR::ReplayCorruption | integration | tla-plus |
| ERR::StorageUnavailable | integration | manual-qa |

## Verus Scope

- **Rust targets**:
  - `contracts/verus/vb_qi37_16_5_lifecycle_journal_storage.rs` — State12 executable standalone Verus harness for the six Rust-local lifecycle/journal/storage obligations; replaces non-executable standalone production-source commands while preserving the original production files as `source_target` in `proof-obligations.jsonl`
  - `crates/vb_runtime/src/shard/lifecycle.rs` — typestate `enum LifecycleState`, transition functions
  - `crates/vb_runtime/src/journal.rs` — `append_event`, `replay`
  - `crates/vb_storage/src/journal.rs` — `write_event`, `read_journal`
  - `crates/velvet_ballistics/src/storage.rs` — command validation

- **Spec/proof surface**:
  - `spec_transition(bead_id, cmd, state_before) -> state_after` — pure state transition function
  - `proof_transition_valid(bead_id, cmd, state_before)` — proves cmd is valid from state_before
  - `proof_append_event_injective(event, journal)` — proves exactly-one-event property
  - `proof_replay_reconstructs_state(journal)` — proves replay fidelity

- **Invariants**:
  - `LifecycleState::invariant(s) == valid_state(s)`
  - `transition_invariant(s, cmd) == valid_transition(s, cmd)`
  - `journal_append_invariant(journal, event) == no_overwrite(journal, event)`

- **Trusted boundary**: Validated `LifecycleState` constructors, `RuntimeJournalEvent` validated builders; State12 harness is a minimal mathematical model derived from `contract.md` clauses and excludes production crate dependency wiring/storage I/O. No `assume`, `external_body`, `external`, or axioms are used.
- **Shell exclusions**: CLI parsing (args.rs), storage I/O (vb_storage), wall-clock time

## TLA+ Scope

- **Module/Model path**: `specs/LifecycleJournal.tla`
- **Variables**: `bead_state`, `journal`, `commands`, `crashed`
- **Actions**: `Init`, `Cancel`, `Resume`, `Retry`, `Answer`, `Crash`, `Replay`
- **Safety invariants**: `NoOverwrite`, `SingleCanonicalState`, `InvalidTransitionBlocked`
- **Temporal properties**: `EventuallyTerminalOrCancelled`, `JournalGrowth`
- **Fairness/deadlock stance**: Weak fairness on lifecycle actions; deadlock freedom confirmed
- **Refinement boundary**: TLA+ `bead_state` refines Rust `lifecycle.rs` typestate; TLA+ `journal` refines Rust `RuntimeJournalEvent` vector; TLA+ `Replay` refines Rust `journal.rs::replay()`
- **Evidence command**: `tlc -config specs/LifecycleJournal.cfg specs/LifecycleJournal.tla`

## Integration Test Scope

- **Happy path**: cancel/resume/retry/answer each fires successfully from valid prior states
- **Invalid transition**: each command fires against all invalid prior states, expects `E_INVALID_TRANSITION`
- **Duplicate request**: same command issued twice in same state, expects `E_DUPLICATE_REQUEST`
- **Stale request**: command issued after state already advanced, expects `E_STALE_REQUEST`
- **Restart/replay**: crash simulation mid-journal, replay reconstructs exact state
- **Structured diagnostics**: all error variants include `{code, context, timestamp, bead_id, command}`

## Replay Test Scope

- **Clean replay**: empty journal or clean snapshot → valid initial state
- **Full replay**: journal with N events → bit-identical bead_state reconstruction
- **Partial replay**: replay from snapshot + incremental journal → identical result
- **Corruption detection**: malformed event → `E_REPLAY_CORRUPTION`

## Waivers

- **Lean/Aeneas/Hax**: Not required — all critical behavior expressible in Verus or TLA+ (see `lean-contract.md`)
- **Kani model checking**: Not separately required — integration tests + TLA+ model cover the state space; Kani may be used as defense-in-depth if implementation reveals numeric/indexing gaps
- **cargo-mutants**: Not required for this bead — integration test coverage is the primary evidence for lifecycle correctness; mutation testing may be applied as defense-in-depth in later beads

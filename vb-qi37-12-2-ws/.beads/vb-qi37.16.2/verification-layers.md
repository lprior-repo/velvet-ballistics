# Verification Layers

## Boundary
- **Verus-owned kernel**: Pure state transition predicates (RuntimeState::is_resumable), journal data structure invariants (append-only, hydration completeness), typestate field presence (ResumeResult), Shard::handle_resume transition logic
- **TLA+ temporal model**: Runtime lifecycle state machine, resume state transition ordering, journal immutability, fail-closed error behavior, eventual liveness
- **Theorem projection**: None (Verus handles all Rust-local proof obligations)
- **Runtime shell**: CLI argument parsing (Command::Resume), structured output formatting, storage backend I/O (FJALL), async scheduling, wall-clock time
- **External systems excluded from formal proof**: FJALL/LSM-tree storage backend, terminal I/O, file system paths

---

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layers | Notes |
|-----------------|---------------|------------------|-------|
| PRE-001 | verus | proptest | run_id existence check predicate |
| PRE-002 | verus | unit | is_resumable predicate on RuntimeState |
| PRE-003 | verus | integration | is_hydration_complete check |
| POST-001 | tla-plus | verus + replay | journal append-before-success temporal ordering |
| POST-002 | unit | integration | structured output field presence |
| POST-003 | tla-plus | unit | fail-closed on invalid resume |
| POST-004 | verus | replay | journal durability evidence append |
| INV-001 | tla-plus | verus | valid state machine transitions |
| INV-002 | verus | tla-plus | journal append-only invariant |
| INV-003 | verus | unit | ResumeResult field presence |
| INV-004 | tla-plus | verus | Failed state not resumable |

---

## Verus Scope

### Rust Target: `vb_runtime::shard::lifecycle::Shard::handle_resume`
- **Spec function**: `spec_handle_resume(run: RunId) -> Result<ResumeResult, ResumeError>`
- **Proof function**: `proof_handle_resume_preserves_invariants`
- **Invariants**: RuntimeState valid, journal immutable, resume only from Resumable
- **Trusted boundary**: RuntimeState constructors validate state validity
- **Shell exclusions**: I/O, async scheduling, storage wall-clock time, CLI output

### Rust Target: `vb_runtime::shard::types::RuntimeState`
- **Spec predicates**: `is_initial`, `is_running`, `is_resumable`, `is_resuming`, `is_failed`
- **Proof invariants**: exhaustive enum variants, state transition validity

### Rust Target: `vb_runtime::journal::RuntimeJournal`
- **Spec functions**: `append`, `get_state`, `is_hydration_complete`
- **Proof invariants**: append-only SEQ, hydration completeness predicate
- **Trusted boundary**: Validated journal event constructors

### Evidence Command
```bash
verus .beads/vb-qi37.16.2/verus_resume_harness.rs
```
Expected: Verus verifies the dedicated pure resume harness for all five Verus-owned obligations with 0 errors. The harness is the executable Verus context; production-to-harness refinement remains an explicit trusted boundary.

---

## TLA+ Scope

### Module/Model Path
`specs/ResumeStateMachine.tla`

### Variables
`RuntimeState`, `Journal`, `PendingResume`, `ResumedSet`

### Actions
`StartRun`, `Suspend`, `Resume`, `CompleteResume`, `FailResume`, `FailRun`

### Safety Invariants
`NoDoubleRunning`, `ValidTransition`, `JournalImmutable`, `FailedNotResumable`, `ResumeIdempotent`

### Temporal Properties
`EventuallyResumed`, `EventuallyTerminalOrFailed`, `NoStarvation`, `JournalAppendBeforeSuccess`

### Fairness/Deadlock Stance
Weak fairness on `CompleteResume` and `FailResume` under enabled actions. Deadlock freedom guaranteed by terminal state reachability.

### Refinement Boundary
TLA+ `RuntimeState` refines Rust `RuntimeState` enum. TLA+ `Journal` refines Rust `RuntimeJournal` append log. TLA+ actions refine Rust `Shard::handle_resume` success/failure paths.

### Evidence Command
```bash
tlc -config specs/ResumeStateMachine.cfg specs/ResumeStateMachine.tla
```
Expected: TLC reports no invariant violations, no deadlock, temporal properties satisfied.

---

## Second-Ring Verification

| Layer | Target | Claim | Command |
|-------|--------|-------|---------|
| replay | vb_storage journal | Resume survives journal replay | `cargo test --package vb_storage --test replay_resume` |
| integration | CLI-runtime boundary | Command::Resume routes correctly | `cargo test --package velvet_ballastics --test cli_integration` |
| unit | ShardCommand::Resume | Valid transitions only | `cargo test --package vb_runtime --lib -- shard::lifecycle` |
| proptest | RuntimeState predicates | Exhaustive state coverage | `cargo test --package vb_runtime --lib -- properties` |

---

## Waivers
None. All contract clauses have at least one verification layer or explicit waiver rationale recorded above.

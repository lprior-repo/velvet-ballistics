# Proof Strategy: vb-core-lower-control-primitives

## Bead Overview
- **Bead**: vb-core-lower-control-primitives
- **State**: 4 (Proof Planning)
- **Artifact root**: `.beads/vb-core-lower-control-primitives/`
- **Scope**: Lowering functions in `crates/vb_compile/src/lib.rs` (`lower_for_each`, `lower_together`, `lower_collect`, `lower_reduce`, `lower_repeat`, `lower_wait`, `lower_ask`)

## Risk Classification

| Risk | Trigger | Lane | Justification |
|------|---------|------|---------------|
| id+1 overflow (u16::MAX) | Arithmetic wrap in `lower_repeat`/`lower_ask` | Verus + Kani | High consequence; Rust-local invariant; Verus cheapest for pre-condition proof; Kani for bounded exhaustive BMC |
| WaitKind invalid combos | Type-safe enum replacement | Verus | Enum exhaustiveness; Rust-local; no concurrency |
| Wrong node count (POST-001–007) | vec![] length wrong | Verus | Rust-local postcondition; proptest insufficient for exact length; Verus spec_fn cheapest |
| Step chain well-formedness | Temporal property across lowering actions | TLA+ / TLC | State machine over steps + slots; interleaving of Lower* actions; discrete math property |
| CompileError exhaustiveness | match missing arms | Clippy | Fast static scan; no formal method needed |

## Verifier Lane Selection

### Verus (primary — Rust-local invariants)
**Artifact**: `crates/vb_compile/src/lib.rs` (verus annotations)
**Command**: `verus crates/vb_compile/src/lib.rs`

| Obligation | Target | What is proven |
|------------|--------|----------------|
| VERUS-INV-001 | `lower_repeat` | `id.checked_add(1)` for `attempt_slot` never wraps; `StepIdx::new` called only after `is_some()` |
| VERUS-INV-002 | `lower_ask` | `id.checked_add(1)` for `resume` never wraps; `StepIdx::new` called only after `is_some()` |
| VERUS-POST-001 | `lower_for_each` | Returns exactly 2 nodes: ForEachStart + ForEachNext |
| VERUS-POST-002 | `lower_together` | Returns exactly 2 nodes: TogetherStart + TogetherJoin |
| VERUS-POST-003 | `lower_collect` | Returns exactly 3 nodes: CollectStart + CollectPage + CollectFinish |
| VERUS-POST-004 | `lower_reduce` | Returns exactly 3 nodes: ReduceStart + ReduceNext + ReduceFinish |
| VERUS-POST-005 | `lower_repeat` | Returns exactly 3 nodes + `attempt_slot = id + 1` |
| VERUS-POST-007 | `lower_ask` | Returns exactly 2 nodes + `resume.id = id + 1` |
| VERUS-WAITKIND | `WaitKind` enum | Enum is dataless with exactly 2 variants; no invalid combos constructible |

**Assumptions**: Verus has access to `crates/vb_compile/src/lib.rs`. Trusted: `CompiledNode` constructors, `StepIdx::new`, `SlotIdx::new`, `slot_idx_for_step`.

### Kani (secondary — bounded BMC for overflow)
**Artifact**: `crates/vb_compile/src/kani_idempotency_parity.rs` (new harness)
**Command**: `cargo kani --harness kani_lower_control --force-mc-flags`
**Coverage**: `id ∈ [0, u16::MAX−1]`; verifies no counterexample for `id+1` overflow in `lower_repeat` and `lower_ask`

**Assumptions**: Kani available in toolchain. Scope limited to `lower_repeat` + `lower_ask` overflow path.

### TLA+ / TLC (structural well-formedness)
**Artifact**: `specs/ControlLowering.tla` + `specs/ControlLowering.cfg` (new)
**Command**: `tlc -config specs/ControlLowering.cfg specs/ControlLowering.tla`
**Model**: One TLA+ action per `lower_*` function; `steps` and `slots` variables; `MaxSteps=10`, `MaxSlots=20`

| Invariant | Formal statement |
|-----------|-----------------|
| NoDuplicateStepIds | `∀i, j ∈ DOMAIN steps : i ≠ j ⇒ steps[i].id ≠ steps[j].id` |
| ValidOffsets | `∀n ∈ steps : n.body > n.id ∧ n.done > n.id` (when present) |
| AskResumeIdCorrect | `∀n ∈ steps : n.kind = Ask ⇒ ∃r ∈ steps : r.id = n.id + 1 ∧ r.kind = AskResume` |
| SlotsRecorded | `∀n ∈ steps : ∀s ∈ referenced_slots(n) : s ∈ DOMAIN slots` |

**Assumptions**: TLA+ toolbox available. Bounded model; real deployment uses u16.

### Clippy (compile-time exhaustiveness)
**Artifact**: `crates/vb_compile/src/lib.rs`
**Command**: `cargo clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings`
**Coverage**: All `CompileError` variants covered in `match`/`if let` expressions.

## Blocker
- **vb-f04l must land before implementation**: The TLA spec and Verus annotations require the actual `lib.rs` implementation to be stable. Proof writing begins after vb-f04l lands.

## Waiver / Not-Applicable Rationale

| Lane | Status | Reason |
|------|--------|--------|
| Flux | not_applicable | No refinement types in scope; type-state enforced by Rust ADTs |
| Loom | not_applicable | No concurrency in lowering (single-threaded, no spawn/Mutex/atomics) |
| Miri | not_applicable | No unsafe code in `vb_compile` (`#![forbid(unsafe_code)]` on all modules) |
| Fuzz | not_applicable | No parser/deserialization in lowering; input is validated `StepIdx`/`SlotIdx` |
| Proptest (as primary) | not_applicable | Overflow and exhaustiveness require formal proof; proptest supplements only |

## Verification Order
1. Clippy (fast, catch match exhaustiveness early)
2. Verus (proof obligations 1-9, 12)
3. Kani (obligation 10 — overflow BMC)
4. TLA+ (obligation 11 — structural model)
5. All gates must pass before state 5 (Proof Writing)

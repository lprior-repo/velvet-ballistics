# Proof Plan Review Input: vb-core-lower-control-primitives

## Reviewer Action Required
Approve or reject the proof plan. Focus on: missing obligations, wrong verifier lanes, unrealistic bounds, missing artifact paths.

---

## Scope
Lowering of YAML control primitives to CompiledNode IR in `crates/vb_compile/src/lib.rs`:
- `lower_for_each` → [ForEachStart, ForEachNext]
- `lower_together` → [TogetherStart, TogetherJoin]
- `lower_collect` → [CollectStart, CollectPage, CollectFinish]
- `lower_reduce` → [ReduceStart, ReduceNext, ReduceFinish]
- `lower_repeat` → [RepeatStart, RepeatAttempt, RepeatFinish] + `attempt_slot = id + 1`
- `lower_wait` → [WaitUntil | WaitEvent]
- `lower_ask` → [Ask, AskResume] + `resume.id = id + 1`

## Key Risks

### CRITICAL: id+1 overflow
- `lower_repeat` line 556-558: `id.checked_add(1).ok_or(...)` for `attempt_slot`
- `lower_ask` line 654-661: `id.checked_add(1).ok_or(...)` for `resume`
- At `id = u16::MAX`, `id + 1` wraps
- **Mitigation**: Verus `spec_repeat_no_overflow` / `spec_ask_no_overflow` + Kani BMC harness

### HIGH: WaitKind invalid combos
- Previously `is_event: bool` allowed invalid combos (e.g., `is_event=false` + `timeout_slot`)
- Now replaced by `WaitKind::Until { deadline }` and `WaitKind::Event { event, timeout: Option }`
- **Mitigation**: Verus exhaustiveness proof on `WaitKind` enum

### MEDIUM: Wrong node counts
- Each `lower_*` must return exact node count (POST-001 through POST-007)
- **Mitigation**: Verus `spec_fn` + `proof_fn` for each function

## Obligations Planned

| ID | Verifier | Artifact | Command | Expected Evidence |
|----|----------|----------|---------|-------------------|
| VERUS-INV-001 | verus | crates/vb_compile/src/lib.rs | verus crates/vb_compile/src/lib.rs | 0 errors; `attempt_slot` overflow proof |
| VERUS-INV-002 | verus | crates/vb_compile/src/lib.rs | verus crates/vb_compile/src/lib.rs | 0 errors; `resume` overflow proof |
| VERUS-POST-001 | verus | crates/vb_compile/src/lib.rs | verus crates/vb_compile/src/lib.rs | node count = 2 for `lower_for_each` |
| VERUS-POST-002 | verus | crates/vb_compile/src/lib.rs | verus crates/vb_compile/src/lib.rs | node count = 2 for `lower_together` |
| VERUS-POST-003 | verus | crates/vb_compile/src/lib.rs | verus crates/vb_compile/src/lib.rs | node count = 3 for `lower_collect` |
| VERUS-POST-004 | verus | crates/vb_compile/src/lib.rs | verus crates/vb_compile/src/lib.rs | node count = 3 for `lower_reduce` |
| VERUS-POST-005 | verus | crates/vb_compile/src/lib.rs | verus crates/vb_compile/src/lib.rs | node count = 3 + attempt_slot = id+1 for `lower_repeat` |
| VERUS-POST-007 | verus | crates/vb_compile/src/lib.rs | verus crates/vb_compile/src/lib.rs | node count = 2 + resume.id = id+1 for `lower_ask` |
| VERUS-WAITKIND | verus | crates/vb_compile/src/lib.rs | verus crates/vb_compile/src/lib.rs | WaitKind dataless enum, 2 variants |
| KANI-OVERFLOW | cargo kani | crates/vb_compile/src/kani_idempotency_parity.rs | cargo kani --harness kani_lower_control --force-mc-flags | No counterexamples for id ∈ [0, u16::MAX-1] |
| TLA-WF-001 | tlc | specs/ControlLowering.tla + specs/ControlLowering.cfg | tlc -config specs/ControlLowering.cfg specs/ControlLowering.tla | All invariants pass; no violations |
| CLIPPY-ERR | cargo clippy | crates/vb_compile/src/lib.rs | cargo clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings | 0 warnings |

## Waived Lanes
- **Flux**: No refinement types in scope; type-state enforced by Rust ADTs
- **Loom**: Lowering is single-threaded; no spawn/Mutex/atomics
- **Miri**: All modules have `#![forbid(unsafe_code)]`
- **Fuzz**: No parser or deserialization in lowering path
- **Proptest (as primary)**: Cannot exhaustively prove node-count equality or overflow absence

## Blocker
- **vb-f04l must land before proof writing begins** — Verus specs and TLA model need stable implementation

## Questions for Reviewer
1. Is Verus the right tool for the node-count postconditions, or should these be tested via proptest only?
2. Are the TLA model bounds (MaxSteps=10, MaxSlots=20) sufficient for structural proof?
3. Should Kani run on both overflow sites or just the highest-risk one (`lower_repeat`)?
4. Is there a preferred artifact location for `kani_lower_control` harness — existing `kani_idempotency_parity.rs` or new file?

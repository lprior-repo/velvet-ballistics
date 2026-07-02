# RE-010 (vb-vbdco) Closure Evidence

**Bead:** vb-vbdco
**Parent:** vb-8muyy (wave-15 P3 bug-hunt follow-up)
**Original RE-010 finding:** `bug-hunt-2026-06-21/findings/runtime-engine/RE-010-evidence-collector-silent-drop.md`
**Closure date:** 2026-06-24
**Resolver:** Lewis (orchestrator)

## Duplicate Status

`vb-vbdco` is a **duplicate** of closed bead `vb-y71ef`
(`bead vb-y71ef: RE-010 surface EvidenceCollector drops as typed errors`,
commit `d8221505b`, merged into `main` by commit `5f101f82b`).

`vb-y71ef` already completed the exact scope described by `vb-vbdco`:

1. Added `EngineError::EvidenceCapacityExceeded` (vb_core/src/errors.rs:422).
2. Added diagnostic code `EVIDENCE_CAPACITY_EXCEEDED_CODE = 0x140E` and
   `engine_error_static_code` mapping in
   `vb_core/src/engine/error_routing.rs:99`.
3. Made `EvidenceCollector::push_step_started`,
   `push_step_succeeded`, `push_slot_written`,
   `push_slot_written_with_taint` return `Result<(), EngineError>`.
4. Removed `dropped` counter from `EvidenceCollector`; capacity
   overflow is now surfaced as a typed error.
5. Wired drive-step call sites
   (`engine/drive.rs:107,127-129,160-162`) to propagate the error via
   `RuntimeEngineError::Core`.
6. Updated 18 test sites across `engine/types.rs`,
   `engine/property_tests.rs`, and `engine/tests.rs` to assert the
   new typed-error contract.
7. The subsequent commit `cd2de4c41` (fix(test): update re_011 capacity
   assertion to capacity=1) and `3bbfa264d` (fix(drive.rs): swap
   mark_running/push_step_started order to match y71ef design) further
   refined the RE-011 transactional ordering so the collector overflow
   path matches the y71ef regression test contract.

Two follow-up beads (`vb-295cc` for RE-011 transactional ordering and
the `cd2de4c41` capacity-assertion tweak) were already merged before
`vb-vbdco` was opened. `vb-vbdco` is therefore an accidental
re-triage of the same finding rather than new work.

## Source-Of-Truth State On `main`

- HEAD on `main`: `cd2de4c4185cd1626c9b5d5dfb373ce837a92e26`
- Branch tip for this closure: `bead/vb-vbdco` = `cd2de4c41`
  (fast-forward from `main`, no new commits).

### Public surface in production code (already on `main`)

`crates/vb_core/src/errors.rs` lines 420-433:

```rust
/// Evidence capacity was exceeded during a non-collect push.
#[error("evidence capacity exceeded: step {step:?} slot {slot:?} capacity {capacity}")]
EvidenceCapacityExceeded {
    step: StepIdx,
    slot: SlotIdx,
    capacity: usize,
    len: usize,
    required: &'static str,
},
```

`crates/vb_runtime/src/engine/types.rs` — `EvidenceCollector` API:

```rust
pub fn push_step_started(&mut self, step: StepIdx) -> Result<(), EngineError>
pub fn push_step_succeeded(&mut self, step: StepIdx, output: Option<SlotIdx>) -> Result<(), EngineError>
pub fn push_slot_written(&mut self, slot: SlotIdx, value: SlotValue) -> Result<(), EngineError>
pub fn push_slot_written_with_taint(&mut self, slot: SlotIdx, value: SlotValue, taint: Taint) -> Result<(), EngineError>
```

All four return `EngineError::EvidenceCapacityExceeded` when the
collector is at capacity; the event is **not** pushed in that case.

`crates/vb_runtime/src/engine/drive.rs` call sites propagate via
`.map_err(RuntimeEngineError::Core)?`:

- line 107: `evidence.push_step_started(pc).map_err(RuntimeEngineError::Core)?;`
- lines 127-129: `evidence.push_step_succeeded(...).map_err(...)?;`
- lines 160-162: `evidence.push_slot_written_with_taint(...).map_err(...)?;`

## Verification Evidence

The remainder of this directory contains raw command output captured
during the closure pass:

- `01-git-state.log` — current git state and relevant commits
- `02-types-rs.txt` — EvidenceCollector public API dump
- `03-errors-rs.txt` — EngineError::EvidenceCapacityExceeded dump
- `04-drive-rs.txt` — drive-step call-site propagation dump
- `05-cargo-check-vb_core.log` — `cargo check -p vb_core`
- `06-cargo-check-vb_runtime.log` — `cargo check -p vb_runtime`
- `07-cargo-test-vb_runtime-evidence.log` — targeted evidence tests
- `08-cargo-test-vb_runtime-blackhat.log` — blackhat engine tests
- `09-cargo-test-vb_runtime-property.log` — property tests
- `10-cargo-test-vb_runtime-all.log` — full vb_runtime unit tests
- `11-cargo-test-vb_runtime-re_011.log` — RE-011 transactional ordering test
- `12-cargo-test-vb_core-all.log` — full vb_core unit tests

All targeted RE-010 evidence tests and the RE-011 transactional
ordering test pass on `main`. No new tests, contracts, or proofs
were authored in this worktree because the contract, behavior tests,
and the production implementation were already merged by `vb-y71ef`.

## Closure Decision

This bead is closed as a duplicate of `vb-y71ef`. No new commits are
required. `bead/vb-vbdco` is fast-forward-equivalent to `main` so no
merge is needed.

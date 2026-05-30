# ARCH-DRIFT REPORT: `crates/vb_runtime/src/engine/drive.rs`

## VERDICT: GUILTY — MUST SPLIT

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | **1383** | 300 | **VIOLATION (461%)** |
| Core logic | ~165 | 300 | OK |
| Test code | **1195** | 0 | VIOLATION |

**Root cause:** 1195 lines of tests are embedded in the same file as production code.
A dedicated `tests.rs` already exists in `engine/` (86KB) — these tests are DUPLICATED.

---

## 2. RESPONSIBILITY MAP

### Drive/Scheduling Responsibilities (lines 19–163)

| Function | Responsibility | Status |
|----------|----------------|--------|
| `compute_max_parallel_in_flight` | Parallelism budget calculation | OK — single responsibility |
| `drive_deterministic_full` | Main drive loop orchestration | OK |
| `initialize_drive` | Drive initialization | OK |
| `begin_drive_step` | Step acquisition | OK |
| `finish_drive_step` | Step completion + evidence | OK — dual responsibility, borderline |
| `signal_is_success` | Signal classification | OK |
| `emit_slot_evidence` | Evidence emission | OK |
| `collect_written_slot` | Evidence slot resolution | OK |

### Test Responsibilities (lines 189–1383)

- 1195 lines of test helpers and test cases
- **Duplicates content in `engine/tests.rs`**

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### VIOLATION 1: Raw `u16` for branch counts

```rust
// Line 20-21
let mut max_branches: u16 = 0;
for i in 0..plan.node_count() {
    let step = StepIdx::new(i);
```

`i` iterates as raw `usize`, then converted to `StepIdx`. The iteration index is raw.

### VIOLATION 2: `usize` error fields (lines 27-31)

```rust
RuntimeEngineError::BranchLimitExceeded {
    max: u16::MAX.into(),        // usize from u16
    requested: branches.len(),    // usize from usize
}
```

`max` and `requested` are `usize` but represent bounded values (u16 MAX). Should be
a `BranchCount` newtype with `TryFrom<usize>` validation.

### VIOLATION 3: Test helpers use raw `u16` (lines 208-331)

```rust
fn cn(id: u16, output: Option<u16>, next: Option<u16>, kind: CompiledNodeKind) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: output.map(SlotIdx::new),
        next: next.map(StepIdx::new),
```

All indices passed as raw `u16`. Should use `StepIdx::new()` directly in tests,
or dedicated test-only constructors.

### VIOLATION 4: `u16::try_from(branches.len())` (line 26)

```rust
let branch_count = u16::try_from(branches.len()).map_err(|_| {
```

`branches.len()` is `usize`. The conversion is lossy but not validated until error.

---

## 4. DDD VIOLATIONS

### Violation: Embedded Tests Violate Single Responsibility Principle

The `drive.rs` module has TWO responsibilities:
1. **Drive loop scheduling** (the actual module purpose)
2. **Test infrastructure** (should be in `engine/tests.rs`)

Per Scott Wlaschin DDD: "One concept per module." The drive module is about **scheduling
workflow execution** — tests are a separate concept about **verification**.

### Violation: `finish_drive_step` Has Dual Responsibility (line 111-124)

```rust
fn finish_drive_step(
    run: &mut RunFrame,
    evidence: &mut EvidenceCollector,
    collect_states: &CollectStates,
    step: DriveStep<'_>,
    signal: &RuntimeSignal,
) -> RuntimeEngineResult<()> {
    mark_step_after_signal(run, step.pc, signal).map_err(RuntimeEngineError::Core)?;
    emit_slot_evidence(run, evidence, collect_states, step.node)?;  // <-- Evidence is observability, not drive
    if signal_is_success(signal) {
        evidence.push_step_succeeded(step.pc, step.node.output);
    }
    Ok(())
}
```

This function mixes **step progression** (mark_step_after_signal) with **evidence emission**.
Evidence emission should be a separate step or handled by the caller.

### Violation: `DriveStep` Struct Leaks Raw Indices

```rust
struct DriveStep<'a> {
    pc: StepIdx,
    node: &'a CompiledNode,
}
```

This is actually OK (StepIdx is a proper newtype). The issue is how it's constructed
in `begin_drive_step` — the `pc` comes from `run.pc()` which returns `StepIdx`.

---

## 5. REQUIRED REFACTORING

### Split 1: Extract Tests to `engine/tests.rs`

All test code (lines 189–1383) should move to `engine/tests.rs`.

**Before:** `drive.rs` = 1383 lines  
**After:** `drive.rs` = 188 lines

### Split 2: Create `drive/types.rs` for NewTypes

Create `crates/vb_runtime/src/engine/drive/types.rs`:

```rust
use crate::engine::types::RuntimeEngineResult;

/// Branch count for TogetherStart nodes.
/// Bounded to u16::MAX as a defense-in-depth measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchCount(u16);

impl BranchCount {
    pub fn new(count: usize) -> Result<Self, BranchCountError> {
        u16::try_from(count)
            .map(BranchCount)
            .map_err(|_| BranchCountError::Exceeded {
                max: u16::MAX as usize,
                requested: count,
            })
    }
}
```

### Split 3: Extract Evidence Logic

`emit_slot_evidence` and `collect_written_slot` should be in `engine/evidence.rs` or
the evidence collector module, not the drive module.

---

## 6. ARCHITECTURAL DRIFT SUMMARY

| Issue | Severity | Fix |
|-------|----------|-----|
| 1383 lines (limit 300) | CRITICAL | Extract embedded tests |
| 1195 lines of duplicated tests | CRITICAL | Remove from drive.rs |
| `finish_drive_step` dual responsibility | HIGH | Split evidence handling |
| `usize` for bounded branch counts | MEDIUM | Newtype `BranchCount` |
| Raw `u16` in test helpers | LOW | Use proper constructors |

---

## 7. RECOMMENDED FILE SPLIT

```
crates/vb_runtime/src/engine/
├── drive.rs          # 188 lines — core drive logic only
├── drive/
│   ├── mod.rs        # Re-exports
│   ├── types.rs      # BranchCount newtype (NEW)
│   └── evidence.rs   # emit_slot_evidence, collect_written_slot (NEW)
└── tests.rs         # Existing 86KB tests + drive tests merged
```

---

## 8. COMPLIANCE GATE

**Before this file can be marked COMPLIANT:**
- [ ] `drive.rs` ≤ 300 lines
- [ ] All tests moved to `engine/tests.rs`
- [ ] Evidence functions extracted to `drive/evidence.rs`
- [ ] `BranchCount` newtype added in `drive/types.rs`
- [ ] No raw `u16` for indices in public API
- [ ] `finish_drive_step` handles only step completion

# Round 4 Agent A2 — Section 65 SideEffect/RetrySafety LETHAL

**Reviewer:** black-hat-reviewer
**Date:** 2026-06-07
**Target:** crates/vb_core/src/action.rs:96-120, crates/vb_compile/src/mod_compile_core.rs:146-215, crates/vb_compile/src/enums/
**Bead:** NONE (drift is untracked)
**Severity:** 95 / 100 — SHIP-BLOCKER

## 1. Exact Gap Between Production and Master

### Production (crates/vb_core/src/action.rs:96-120)

```rust
pub enum SideEffect {            // 5 variants
    None = 0,                    // line 98
    Writes = 1,                  // line 100
    Sends = 2,                   // line 102
    Creates = 3,                 // line 104
    Destroys = 4,                // line 106
}

pub enum RetrySafety {           // 3 variants
    Safe = 0,                    // line 115
    KeyRequired = 1,             // line 117
    Unsafe = 2,                  // line 119
}
```

### Master (velvet-ballistics-MASTER.md:3265-3280)

```rust
pub enum SideEffect {            // 7 variants — REQUIRED
    Pure,                        // master 3266
    LocalRead,                   // master 3267
    LocalWrite,                  // master 3268
    ExternalRead,                // master 3269
    ExternalWrite,               // master 3270
    Process,                     // master 3271
    UnsafeShell,                 // master 3272
}

pub enum RetrySafety {           // 4 variants — REQUIRED
    Idempotent,                  // master 3276
    RequiresIdempotencyKey,      // master 3277
    NotRetrySafe,                // master 3278
    Unknown,                     // master 3279
}
```

**Variance: 2 missing SideEffect variants, 1 missing RetrySafety variant, 4 variants renamed with material semantic drift.**

## 2. What the Test Files Assert

crates/vb_compile/src/enums/tests/side_effect_tests.rs:117-125 and crates/vb_compile/src/enums/tests/retry_safety_tests.rs:89-94 construct arrays of the master variant set:

```rust
// side_effect_tests.rs:117
let variants = [
    SideEffect::Pure, SideEffect::LocalRead, SideEffect::LocalWrite,
    SideEffect::ExternalRead, SideEffect::ExternalWrite,
    SideEffect::Process, SideEffect::UnsafeShell,
];

// retry_safety_tests.rs:89
let variants = [
    RetrySafety::Idempotent, RetrySafety::RequiresIdempotencyKey,
    RetrySafety::NotRetrySafe, RetrySafety::Unknown,
];
```

## 3. The Malformed `use` Import IS a Compile-Time Syntax Error

Both test files contain the identical malformed import (lines 12-13 of each):

```rust
use vb_core::{                                       // line 12
use vb_core::action::ActionName;                    // line 13 — INVALID
    action::verify_idempotency,
    ActionContract, ActionId, Idempotency, RetrySafety, RunFrame, RunId,
    SideEffect, SlotIdx, SlotValue, StepIdx, Taint,
};
```

`rustc --edition 2024 --crate-type=lib` returns:
```
error: expected identifier, found keyword `use`
  --> tests/side_effect_tests.rs:13:1
13 | use vb_core::action::ActionName;
   | ^^^ expected identifier, found keyword
```

## 4. Test Files Are 100% Dead Code

crates/vb_compile/src/lib.rs:14-26 declares 13 modules. There is NO `mod enums;`. The directory crates/vb_compile/src/enums/ is not in the module tree.

## 5. Three Workflows That Pass the Broken Gate but Should Be Rejected

### 5a. Process-spawn action
- Production: author cannot declare SideEffect::Process. Coerced to SideEffect::Writes, RetrySafety::KeyRequired, Idempotency::IdempotentExternal.
- **Broken gate: ACCEPTED**
- **Master gate: REJECTED** (Section 65 rule: "Process → Retry rejected by default")
- **Real-world impact**: a process-spawning action with a key ingredient derived from $run.id:$step.id will silently retry on suspension, double-spawning the child process.

### 5b. Arbitrary shell-execution action
- Same coercion, same broken gate accept, master gate reject.
- **Real-world impact**: A flaky network partition causes the action to re-execute, doubling the side-effect of an arbitrary shell command.

### 5c. Local-state-write action with policy-overridden retry
- **Broken gate: ACCEPTED** (matches the (_, Safe, IdempotentExternal) arm; no idempotency key is even checked)
- **Master gate: REJECTED without explicit policy override**

## 6. YAML Behavior

grep -rn "side_effect\|retry_safety\|SideEffect\|RetrySafety" against crates/vb_compile/src/ast/ and crates/vb_compile/src/schema/ returns zero matches. The YAML compiler does not lift these fields from workflow source.

## 7. Bead Tracking Status: NO BEAD EXISTS

bd list --all | grep -iE "MAJOR-6|taxonomy|enum mismatch|side_effect" returns no hits.

## Top 3 Worst Findings

1. **The gate cannot express the master's most-dangerous categories.** SideEffect::Process and SideEffect::UnsafeShell are not in the production enum. Any workflow author who wants to declare these must coerce to a 4-bucket broken variant and pass a more permissive gate.

2. **The drift is structurally locked into CI.** crates/vb_compile/tests/idempotency_parity.rs:36-296 iterates over the broken 5×3 cardinality. crates/vb_compile/src/kani_idempotency_parity.rs:20-46 declares the broken cardinality. Migrating to master breaks the green build.

3. **The drift is untracked, unobservable, and unfix-able-in-isolation.** No MAJOR-6 bead exists. The FIXME(MAJOR-6) lives in enums/mod.rs:7 (dead code). The two test files are uncompilable AND orphaned.

## Verdict: SHIP-BLOCKER

Recommended action: open a P0 bead for MAJOR-6 taxonomy migration. Block landing of any new workflow with shell-execution or process-spawning actions until the migration completes. Add a Kani harness that asserts enum SideEffect has at least 7 variants.

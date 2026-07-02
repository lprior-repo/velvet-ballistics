# Moon :test Gate Report for vb-fb52

**Date:** Sat May 09 2026
**Task:** `moon run :test`
**Exit Code:** 101
**Status:** FAILED

## Summary

The `:test` gate failed during the `velvet-ballistics:check` task.

## Failed Tasks

- `velvet-ballistics:check` - exit code 101

## Error Categories

### 1. Missing Crate `serde_yaml` (xtask/src/evidence.rs)
```
error[E0433]: cannot find module or crate `serde_yaml` in this scope
  --> xtask/src/evidence.rs:328:20
  --> xtask/src/evidence.rs:359:20
  --> xtask/src/evidence.rs:360:36
  --> xtask/src/evidence.rs:388:20
```

### 2. Missing Functions in xtask/src/main.rs
```
error[E0425]: cannot find function `cmd_ai_fast` in this scope
error[E0425]: cannot find function `cmd_ai_deep` in this scope
error[E0425]: cannot find function `cmd_ai_release` in this scope
```

### 3. Struct Field Mismatch (evidence::Error::GateTimeout)
```
error[E0026]: variant `evidence::Error::GateTimeout` does not have a field named `gate_name`
error[E0027]: pattern does not mention field `gate`
```

### 4. Private Constructor (vb_storage/src/trimming.rs)
```
error[E0532]: cannot match against a tuple struct which contains private fields
  --> crates/vb_storage/src/trimming.rs:1336:33
note: constructor is not visible here due to private fields
pub struct EventSeq(pub u64);
```

### 5. Missing `attempt` Field on JournalEvent Variants
Over 40 errors in:
- `crates/vb_storage/src/recovery/replay/summary.rs`
- `crates/vb_storage/src/recovery/tests.rs`

```
error[E0559]: variant `events::JournalEvent::AskAnsweredEvent` has no field named `attempt`
error[E0559]: variant `events::JournalEvent::ActionFailedEvent` has no field named `attempt`
error[E0559]: variant `events::JournalEvent::SlotWrittenEvent` has no field named `attempt`
... (many more)
```

## Affected Files

- `xtask/src/evidence.rs` - serde_yaml usage + GateTimeout variant
- `xtask/src/main.rs` - missing AI command functions
- `crates/vb_storage/src/trimming.rs` - EventSeq constructor visibility
- `crates/vb_storage/src/recovery/replay/summary.rs` - missing attempt fields
- `crates/vb_storage/src/recovery/tests.rs` - missing attempt fields (extensive)
- `crates/vb_core/src/engine/tests/integration_*.rs` - unused imports (warnings only)
- `crates/vb_validate/src/type_taint_tests.rs` - unused import (warning only)

## Root Cause Analysis

The failures indicate schema/API drift between:
1. The `JournalEvent` enum definition (which apparently no longer has `attempt` fields)
2. The test code that constructs `JournalEvent` variants (still expecting `attempt` fields)
3. The `EventSeq` struct having private fields that `trimming.rs` tries to construct publicly
4. Missing `serde_yaml` dependency in xtask
5. Missing `cmd_ai_*` command handlers in xtask

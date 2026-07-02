# Domain Model Review: Enum Types vs String

## Summary

A module split/restore cycle left stale String-literal code in `crates/vb_ipc/src/server/handlers.rs` where strongly-typed enums were required. This document reviews the five affected enum types and why enum usage is mandatory for type safety at the IPC boundary.

## Affected Enum Types

All enums are defined in `crates/vb_ipc/src/payloads.rs` and are `Serialize` + `Deserialize` with explicit wire-format naming.

### 1. `GateKind`
- **Purpose:** Identifies which validation gate a workflow verification result corresponds to.
- **Variants:** `Gate07ExpressionStackDepth` through `Gate15DeterminismProof`.
- **Wire format:** `snake_case` via `serde(rename_all)` and per-variant `serde(rename)`.
- **Risk of String:** A misspelled gate string would silently produce wrong verification results. The enum makes invalid gate names unrepresentable at the type level.

### 2. `PassFail`
- **Purpose:** Binary pass/fail status for verification results.
- **Variants:** `Pass`, `Fail`.
- **Wire format:** `PascalCase` via `serde(rename_all)`.
- **Risk of String:** Any string other than `"Pass"`/`"Fail"` would deserialize to an invalid state. Using the enum ensures exhaustiveness in match arms.

### 3. `TaintPathStatus`
- **Purpose:** Severity classification for taint propagation paths.
- **Variants:** `Dangerous`, `Warning`.
- **Wire format:** `lowercase` via `serde(rename_all)`.
- **Risk of String:** Same as above -- only two valid states exist, and the enum enforces this.

### 4. `NodeKind`
- **Purpose:** Classification of nodes in a compiled workflow graph (`Nop`, `SetConst`, `EvalExpr`, `Choose`, `ForEachStart`, etc.).
- **Variants:** 31 distinct workflow node kinds.
- **Wire format:** `PascalCase` via `serde(rename_all)`.
- **Risk of String:** With 31 variants, String-based dispatch is error-prone and loses exhaustiveness checking. The enum enables the compiler to verify all variants are handled in `match` expressions.

### 5. `EdgeType`
- **Purpose:** Classification of control-flow edges in a workflow graph.
- **Variants:** `Branch`, `LoopBody`, `LoopExit`, `ParallelBranch`, `ParallelJoin`, `Fallthrough`, `ErrorHandler`, `Jump`.
- **Wire format:** `snake_case` via `serde(rename_all)`.
- **Risk of String:** Edge types drive graph traversal logic; a typo in a string would corrupt graph analysis (e.g., taint tracking).

## Type Safety Boundary

```
Wire bytes  --(serde)-->  Enum variant  --(Rust code)-->  Handler logic
     ^                        ^
     |                        |
  String                Typed match
  (untrusted)           (exhaustive, safe)
```

The handler code must operate on the **enum variant** side of this boundary. Stale String-literal code that bypassed the enum violated this boundary and produced E0308 errors because the surrounding structs expected the enum types.

## Fix Pattern

Before (broken):
```rust
edge_type: "branch".to_string(),  // E0308: expected EdgeType, found String
```

After (correct):
```rust
edge_type: crate::EdgeType::Branch,  // Typed variant matches struct field
```

## Recommendations for Downstream Work

1. **Keep enums exhaustive** -- Any future addition of a variant to these enums must update all `match` arms in `handlers.rs`.
2. **Avoid `From<&str>` for unknown inputs** -- The current `From<&str>` implementations have a default fallback (e.g., `NodeKind::Nop` for unknown strings). This is acceptable for backward-compatible deserialization but should not be used in new handler code.
3. **Orphaned files** -- The four files in `crates/vb_ipc/src/server/handlers/` (`command.rs`, `event.rs`, `query.rs`, `session.rs`) may contain similar String/enum mismatches if they are ever re-integrated. A future bead should audit them before wiring them into the module tree.

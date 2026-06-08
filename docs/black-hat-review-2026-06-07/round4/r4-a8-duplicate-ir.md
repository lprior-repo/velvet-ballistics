# Round 4 Agent A8 — Duplicate IR Types (CRITICAL)

**Reviewer:** black-hat-reviewer · **STATUS: SHIP-BLOCKER · 82/100**

The duplicates are not the worst-kept-secret in the repo. The worst kept secret is: **the canonical is not the only one a reader of the master will reach for, and the dead twin is silently divergent, intentionally-deferred-debt that no live build is allowed to catch.**

## Per-File Pair Diffs (ground truth)

### Pair 1: `nodes.rs` vs `workflow/types.rs::CompiledNode` + `CompiledNodeKind`

| Item | Dead `nodes.rs:9-18` | Canonical `workflow/types.rs:522-538` |
|------|----------------------|----------------------------------------|
| `CompiledNode.id` | `StepIdx` | `StepIdx` ✓ |
| `CompiledNode.output` | `Option<SlotIdx>` | `Option<SlotIdx>` ✓ |
| `CompiledNode.next` | `Option<StepIdx>` | `Option<StepIdx>` ✓ |
| `CompiledNode.on_error` | **MISSING** | `Option<StepIdx>` |
| `CompiledNode.error_slot` | **MISSING** | `Option<SlotIdx>` |
| `CompiledNode.kind` | `CompiledNodeKind` | `CompiledNodeKind` ✓ |

**Result: DIVERGED** (semantic loss of two fields: `on_error` and `error_slot`). Severity: 9/10.

### Pair 2: `expressions.rs` vs `workflow/types.rs::ExprOp`

**Result: BYTE-IDENTICAL** (modulo 3 dead comment lines). Severity: 2/10.

### Pair 3: `accessors.rs` vs `workflow/types.rs::AccessorProgram` + `PathSegment`

**Result: BYTE-IDENTICAL**. Severity: 1/10.

### Pair 4: `compiled_workflow.rs` vs `compiled_workflow.rs.removed`

**NOT byte-identical** (different SHA-256). The diff is in `ResourceContract::DEFAULT`:
- live: `max_steps = 1_000`, `max_constants = 8_192`
- .removed: `max_steps = 10_000`, `max_constants = u16::MAX`

The `.removed` file is a more permissive stale snapshot. Both are dead. Severity: 6/10.

### Pair 5: `validation.rs` + `validation/{graph,nodes,resource,targets}.rs` (884 lines) vs `workflow/validation.rs` (1046 lines)

The dead set is **NOT** a copy of the canonical. The dead `validation/resource.rs:13-15` uses `use crate::accessors::AccessorProgram;`, `use crate::expressions::ExprProgram;`. **Neither `crate::accessors` nor `crate::expressions` exists in `lib.rs`.** Activate the module and the build fails.

The dead `validation/graph.rs:159` uses `&[crate::nodes::Branch]`. **There is no `Branch` type in the dead `nodes.rs`.** Activate the module: compile error.

Severity: 7/10.

## Master Contract Audit

### Master line 572: `CompiledNode` = `id, output, next, kind`

**WRONG** — the canonical has 6 fields (`id, output, next, on_error, error_slot, kind`). The dead `nodes.rs` matches the master's wrong contract. **The dead file is more faithful to the master than the canonical is.** This is a contract-drift bomb.

### Master line 578: `ResourceContract` = "16 fields"

**WRONG (and stale in the master)**. The canonical is 18 fields. The dead `compiled_workflow.rs` has 16 fields (matches the master's count, but is missing `max_transitions_per_tick` and `allows_secret_results` — the two fields that `.beads/vb-xi2f.35/proof-to-rust-map.md:20` flags as "blocks verification of PO-K11").

The dead `ResourceContract` literally matches the master count while diverging from the production count. **The master is reading the dead twin.**

## Compile-Error Guard Check

```
$ grep compile_error  nodes.rs expressions.rs accessors.rs compiled_workflow.rs validation.rs validation/*.rs
0 matches
```

**There is no `compile_error!`, no `static_assert`, no `const _: () = assert!(...)` linking the duplicates to the canonical.** Drift is undetectable at build time.

## Will a future agent's edit to `nodes.rs` silently take effect?

**No. It cannot.** A `cargo check -p vb_core` does not parse `nodes.rs` because no `mod nodes;` is declared anywhere reachable. The build will be green. Tests will pass. The canonical 6-field `CompiledNode` will keep its `on_error` and `error_slot`. The agent will believe they edited the production type, ship, and the next reader of the master will not notice.

The only way `nodes.rs` becomes load-bearing is if someone later adds `pub mod nodes;` to `lib.rs` to "clean up" the dead file, at which point:
1. The 4-field `CompiledNode` collides with the 6-field one re-exported via `workflow::types`.
2. The dead `validation/nodes.rs` would activate and break `validation/resource.rs`.
3. The dead `validation/graph.rs` would fail to compile (missing `Branch`).
4. `kani_resource_contract_validation_18_fields.rs` would activate and try to import `vb_core::validation::resource::*`, which still does not exist in `lib.rs`.

This is a **ticking combinatorial explosion**, not a flat debt.

## Top 3 Worst Findings

1. **`CompiledNode` dead twin is missing `on_error` and `error_slot` (silent 2-field regression waiting to happen)** — 9/10. The dead `CompiledNode` is a 4-field struct. The canonical is a 6-field struct. Master line 572 documents the 4-field version. The dead file matches the master, the canonical does not. Future editor following the master will edit the dead file, see no effect, and assume either (a) they need to re-run the build, or (b) the build is doing the right thing. There is no `compile_error!` and no in-source pointer.

2. **`validation.rs` + `validation/{graph,nodes,resource,targets}.rs` (884 lines) is a parallel universe that references a `Branch` type that does not exist in the dead `nodes.rs`** — 8/10. The dead `validation/graph.rs:159` references `crate::nodes::Branch`. The dead `nodes.rs` has no `Branch` type. The dead `validation/resource.rs:13,15` references `crate::accessors` and `crate::expressions` modules that are also not in `lib.rs`. The dead validator **cannot compile as a unit** without dragging in a fictional module set.

3. **Master line 3443 cites `crates/vb_core/src/nodes.rs` as the canonical location for the `Finish` node, and master line 578 claims `ResourceContract` has 16 fields. Both are wrong against production.** — 7/10. Master says 16 fields; canonical has 18. Master cites `nodes.rs`; canonical is in `workflow/types.rs`. Master is **documenting the dead twin**, not the production artifact.

## Required Fixes Before Ship

1. **Update `velvet-ballistics-MASTER.md:572, 578, 3443`** to reflect the 6-field `CompiledNode`, 18-field `ResourceContract`, and `workflow/types.rs` file paths.
2. **Delete the dead `validation.rs` + `validation/{graph,nodes,resource,targets}.rs` directory** (884 lines).
3. **Delete `nodes.rs`, `expressions.rs`, `accessors.rs`** (393 lines) — or replace each with a `compile_error!("...")` pointer.
4. **Delete `compiled_workflow.rs.removed`** outright.
5. **For `compiled_workflow.rs`** (228 lines), either delete it or rewire it to be a `pub use` re-export.
6. **Add a `compile_error!` block at the top of each surviving dead file** (or simply delete them).
7. **Re-register `kani_resource_contract_validation_18_fields.rs`** in `lib.rs` *after* fixing the import.

Until the master is corrected and the dead files are either deleted or guarded with `compile_error!`, any new agent reading the master will edit dead code, and any new agent doing "cleanup" will detonate the build.

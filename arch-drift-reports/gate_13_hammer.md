# ARCH-DRIFT HAMMER REPORT
## File: `crates/vb_validate/src/gate_13_cycles.rs`
## Line Count: 590 (CRITICAL OVERRUN — limit is 300)
## Verdict: GUILTY — ZERO DEFENSES ACCEPTED

---

## EXECUTIVE SUMMARY

590 lines of gate validation code is an architectural crime. This file does ONE thing: detects cycles in a slot dependency graph. It should be ~120 lines. The rest is primitive obsession, god-function syndrome, and a 130-line test module that screams "we never refactored."

---

## VIOLATION 1: LINE COUNT (CRITICAL)

**Limit:** 300 lines
**Actual:** 590 lines
**Overrun:** 196%

This file ships 590 lines for a single validation gate. Even by aggressive DDD standards, this is inexcusable. Gate validation is a thin, focused concern. The cyclomatic complexity of `node_reads` alone (giant match on `CompiledNodeKind`) is a tell that abstraction was abandoned entirely.

---

## VIOLATION 2: PRIMITIVE OBSESSION — `Vec<Vec<usize>>` for Slot Graph

```rust
let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); slot_count];
```

**Domain fact:** This is a **slot dependency graph** — a directed graph where nodes are slot indices and edges represent "slot A writes to slot B".

**Crime:** Raw `Vec<Vec<usize>>` treats this as if it's a bunch of arrays. No encapsulation. Every caller can mutate this however they want. No type safety.

**Should be:**
```rust
struct SlotDependencyGraph {
    adj: Vec<SmallVec<[SlotIdx; 4]>>,
    //           ^ domain-typed, not raw usize
}
```

The edge list for each node should be `SmallVec<[SlotIdx; N]>` — a slot's out-degree is typically bounded. Using `SmallVec` avoids heap allocation for the common case.

---

## VIOLATION 3: PRIMITIVE OBSESSION — Magic Color State (`u8`)

```rust
let mut visited: Vec<u8> = vec![0; slot_count];
// ... later ...
if color == 1 { /* gray */ }
if color == 2 { /* black */ }
```

**Domain fact:** The DFS visitor implements a classic three-color cycle detection:
- `0` = white (unvisited)
- `1` = gray (in-progress, on current DFS stack)
- `2` = black (fully processed)

**Crime:** `u8` is a primitive. The semantics (WHITE/GRAY/BLACK) are completely opaque. Any arithmetic on this `u8` would be meaningless. No one can read `color == 1` and know what it means without reading the surrounding context.

**Should be:**
```rust
#[derive(Clone, Copy, PartialEq)]
enum VisitState { White, Gray, Black }

let mut visited: Vec<VisitState> = vec![VisitState::White; slot_count];
```

---

## VIOLATION 4: PRIMITIVE OBSESSION — Raw `usize` Throughout

Every function dealing with slots uses raw `usize`:
```rust
fn detect_cycle_dfs(slot: usize, adjacency: &[Vec<usize>], visited: &mut [u8])
```

`slot` is not a raw integer — it's a **slot index** with domain semantics. The `SlotIdx` newtype exists in `vb_core::ids` but is used only at the boundary. Internally, everything is naked `usize`.

**Every `usize` in this file representing a slot should be `SlotIdx`.**

---

## VIOLATION 5: GOD FUNCTION — `node_reads` (130 lines, 30+ match arms)

```rust
fn node_reads(node: &CompiledNode, expressions: &[ExprProgram]) -> Vec<SlotIdx> {
    let mut reads = Vec::new();
    match &node.kind {
        CompiledNodeKind::Nop | CompiledNodeKind::SetConst { .. } => {}
        CompiledNodeKind::Copy { source } => { reads.push(*source); }
        // ... 30 more arms ...
    }
    reads
}
```

This function is a Visitor pattern implemented as a 130-line match statement. Every time `CompiledNodeKind` gains a new variant, this match must be updated. There's no abstraction barrier.

**Problems:**
1. No abstraction — callers must know the internal shape of every `CompiledNodeKind` variant
2. Duplicated logic across branches — `CollectPage`, `CollectNext`, `CollectFinish` all push `collector_slot`
3. The `expressions.get(expr.as_usize())` pattern is repeated 4 times with no helper
4. The `if let ExprOp::LoadSlot(s) = op` pattern is repeated 4 times

**Should be:** `CompiledNodeKind` itself should expose a `fn slot_reads(&self) -> Vec<SlotIdx>` method (or equivalent visitor). The `CompiledNodeKind` enum lives in `vb_core` — this is where the behavior belongs, not in gate validation.

---

## VIOLATION 6: LEAKED DOMAIN LOGIC — DFS Algorithm Exposed

```rust
for slot in 0..slot_count {
    if visited.get(slot) == Some(&0) {
        detect_cycle_dfs(slot, &adjacency, &mut visited)?;
    }
}
```

The **algorithm** (iterative DFS with color marking) is mixed with the **domain concern** (building the slot dependency graph). A pure function `detect_cycle_dfs` operating on raw `usize`/slice types reveals nothing about why we're doing this or what the data means.

**Should be:** A `SlotDependencyGraph` type with a `fn has_cycle(&self) -> Option<Cycle>` method. The algorithm is an implementation detail of that type.

---

## VIOLATION 7: ERROR TYPE WEAKNESS — Stringly-typed Cycle Chain

```rust
ValidationError::SlotDependencyCycle {
    slot,
    chain: format!("slot {slot} -> slot {neighbor}"),
}
```

The `chain` field is a formatted `String`. This makes it impossible to:
- Programmatically inspect the cycle path
- Test specific cycle shapes
- Replay or serialize cycle errors
- Render them consistently

**Should be:** `chain: Vec<SlotIdx>` — a proper domain sequence of slot indices in the cycle.

---

## VIOLATION 8: BLOWN ABSTRACTION — `WorkflowParts` Passed raw

```rust
pub fn validate_gate_13_no_slot_cycles(parts: &WorkflowParts) -> ValidationResult<()> {
```

`WorkflowParts` is the entire compiled workflow. Gate 13 only cares about **slot dependencies**. The function signature leaks the entire domain object when it only needs a subgraph.

**Should be:** Accept a `&SlotDependencyGraph` — a focused, single-responsibility argument.

---

## VIOLATION 9: REPEATED INDEX BOUNDS CASTING

```rust
let out_usize = output.as_usize();
if out_usize < slot_count {
    for read_slot in reads {
        let read_usize = read_slot.as_usize();
        if read_usize < slot_count && read_usize != out_usize {
```

`as_usize()` called twice per iteration, with manual bounds checking. On 32-bit platforms this could truncate. The `SlotIdx::new()` constructor exists but is not used for validation.

---

## VIOLATION 10: TEST BOILERPLATE ABUSE (130 lines)

The `make_parts`, `finish_node`, `copy_node` helpers are fine. But the test nodes themselves are enormously verbose:
```rust
CompiledNode {
    id: StepIdx::new(0),
    output: Some(SlotIdx::new(0)),
    next: Some(StepIdx::new(1)),
    on_error: None,
    error_slot: None,
    kind: CompiledNodeKind::SetConst { value: ConstIdx::new(0) },
}
```

A `CompiledNodeBuilder` or a `test_helpers` module in `vb_core` would eliminate 80% of this noise.

---

## ROOT CAUSE ANALYSIS

The file grew organically without refactoring. The `node_reads` function started small and kept getting `CompiledNodeKind` arms added to it. The graph construction was inlined into the validation function rather than extracted. Tests were copy-pasted with minor variations.

**This is technical debt compounding over time, not a design choice.**

---

## PRESCRIPTION

### Minimum viable refactor (target: 280 lines):

1. **Newtype the graph:**
   ```rust
   // In vb_core, or vb_validate if isolated
   struct SlotGraph { adj: Vec<SmallVec<[SlotIdx; 4]>>, slot_count: SlotCount }
   ```

2. **Move `node_reads` to `CompiledNodeKind`:**
   The enum in `vb_core` should own `fn reads(&self) -> &[SlotIdx]`. Then `node_reads` disappears.

3. **Color enum:**
   ```rust
   enum Color { White, Gray, Black }
   ```

4. **Extract `SlotGraph::from_parts(parts: &WorkflowParts) -> Self`**

5. **Extract `SlotGraph::find_cycle(&self) -> Option<Cycle>`**

6. **Reduce test noise** with a `TestNode` builder or fixture module.

7. **Validation function becomes:**
   ```rust
   pub fn validate_gate_13_no_slot_cycles(parts: &WorkflowParts) -> ValidationResult<()> {
       let graph = SlotGraph::from_parts(parts)?;
       graph.find_cycle().map_or(Ok(()), |c| Err(ValidationError::SlotDependencyCycle(c)))
   }
   ```

---

## VERDICT

**ARCHITECTURAL GUILT: UNANIMOUS**

| Violation | Severity |
|-----------|----------|
| Line count overrun | CRITICAL |
| `Vec<Vec<usize>>` adjacency | MAJOR |
| `u8` magic color state | MAJOR |
| Raw `usize` slot handling | MAJOR |
| `node_reads` god function | CRITICAL |
| Algorithm/domain leakage | MAJOR |
| Stringly-typed cycle chain | MODERATE |
| `WorkflowParts` overkill param | MODERATE |
| Repeated bounds casting | MINOR |
| Test boilerplate | MINOR |

**Recommended action:** Refactor to ≤300 lines before any new bead can be opened on this module. The `SlotGraph` newtype and moving `slot_reads` into `CompiledNodeKind` are non-negotiable prerequisites.

---

*Generated by arch-drift-hammer on 2026-05-29*
*Workspace: `/home/lewis/src/velvet-ballistics/arch-drift-hammer`*

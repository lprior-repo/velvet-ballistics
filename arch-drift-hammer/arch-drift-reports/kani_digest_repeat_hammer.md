# ARCH-DRIFT REPORT: kani_digest_repeat.rs
## File: `/home/lewis/src/velvet-ballistics/crates/vb_compile/src/kani_digest_repeat.rs`
## Lines: 376 / 300 LIMIT — VIOLATION CONFIRMED

---

## LINE COUNT VERDICT: VIOLATION

```
376 lines > 300 line hard limit
STATUS: MUST SPLIT
```

---

## 1. FILE MAPPING — WHAT THIS FILE DOES

### Purpose
Kani harnesses (PO-001 through PO-005) verifying that `digest_step_primitive` correctly
incorporates `max_attempts` and `body` fields of the `Repeat` primitive into the blake3
hash state, producing distinct digests for distinct Repeat configurations.

### Harnesses (5 total)
| Proof ID | Harness | Unwind | Lines |
|----------|---------|--------|-------|
| PO-001 | `kani_repeat_max_attempts_consumed` | 4 | 98–144 |
| PO-002 | `kani_repeat_body_consumed` | 8 | 146–195 |
| PO-003 | `kani_repeat_different_params_different_digest` | 6 | 197–244 |
| PO-004 | `kani_repeat_both_impls_equivalent` | 4 | 246–295 |
| PO-005 | `kani_finish_set_digest_unchanged` | 6 | 297–376 |

### Helper Infrastructure
| Name | Lines | Purpose |
|------|-------|---------|
| `kani_string()` | 35–43 | Bounded symbolic string from `kani::any()` |
| `make_finish_step()` | 49–60 | Symbolic Finish StepAst builder |
| `make_set_step()` | 62–76 | Symbolic Set StepAst builder |
| `symbolic_finish_scalar()` | 78–86 | Symbolic ScalarValue (Integer or String) |
| `symbolic_set_body_step()` | 88–92 | Symbolic Set body step |

### Bind Points
- **Canonical implementation**: `mod_compile_lowering::digest_step_primitive` (part_05.rs:194)
- **Dead duplicate**: `compile::digest_step_primitive` (compile/mod.rs:243) — unreachable, NOT in module tree

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### VIOLATION 1: `u16` for `max_attempts` (line 113, 114, 164, 213, 273)

`u16` is used directly for `max_attempts` throughout all 5 harnesses. This is primitive
obsession — `max_attempts` is a domain concept ("retry budget") that deserves a NewType.

**Evidence:**
```rust
let max1: u16 = kani::any();
let max2: u16 = kani::any();
kani::assume(max1 != max2);
```

**Domain model violation**: `MaxAttempts` is a bounded natural number (1..=u16::MAX,
with 0 meaning infinite retries). Raw `u16` allows 0 to be treated as a no-op when it
actually means "run forever."

**Required refactor**: `struct MaxAttempts(u16)` with `impl TryFrom<u16>` for the
0-case distinction (zero = infinite). See `vb_core::budget.rs` for existing patterns.

---

### VIOLATION 2: `String` for step IDs and output/value fields (lines 51, 63, 88–91)

The helpers `kani_string()`, `make_finish_step()`, `make_set_step()` all produce or
consume raw `String`. Step identifiers are not just "strings" — they are domain symbols
that appear in control flow graphs and IR.

**Evidence:**
```rust
fn make_set_step(id: &str, output: &str, value: &str) -> StepAst {
    StepAst {
        id: id.to_string(),
        primitive: StepPrimitive::Set { output: output.to_string(), value: value.to_string() },
        ...
    }
}
```

**Required refactor**: `struct StepId(String)`, `struct OutputSlot(String)`,
`struct ValueSlot(String)` as NewTypes.

---

### VIOLATION 3: `Vec<StepAst>` for body with no Value Object (lines 120, 167, 170, etc.)

`body: Vec<StepAst>` is passed directly. The body of a Repeat is not a "vector of AST
nodes" — it is a **RepetitionBody** domain concept: an ordered sequence of steps that
collectively define one retry attempt's work.

**Evidence (line 120):**
```rust
let empty_body: Vec<StepAst> = vec![];
```

**Required refactor**: `struct RepetitionBody(Vec<StepAst>)` with a `From<Vec<StepAst>>`
implementation.

---

## 3. GOD RULE VIOLATIONS

### GOD RULE 1 (Uses `kani::any()` for symbolic generation): COMPLIANT ✓

Each harness uses `kani::any()` for symbolic max_attempts and body generation. No
hardcoded structural inputs are used for the Repeat primitive itself.

### GOD RULE 2 (Binds to actual implementation): COMPLIANT ✓

Line 28:
```rust
use crate::mod_compile_lowering::digest_step_primitive;
```
This correctly binds to the canonical part_05.rs implementation.

### GOD RULE 3 (No hardcoded structural inputs): COMPLIANT ✓

The `kani_repeat_max_attempts_consumed`, `kani_repeat_body_consumed`, and
`kani_repeat_different_params_different_digest` harnesses all generate inputs
symbolically via `kani::any()`.

---

## 4. ARCHITECTURAL DRIFT FINDINGS

### DRIFT-1: Dead Code Binding — compile/mod.rs::digest_step_primitive

The comment on line 252–258 documents that there is a **second, unreachable
implementation** of `digest_step_primitive` in `crates/vb_compile/src/compile/mod.rs`
(lines 243–261). This duplicate is:

- NOT exposed through any `mod` declaration in `lib.rs`
- NOT reachable from any public API (`compile_workflow`, `compile_source`)
- **Active drift**: the dead implementation diverges from the canonical one

**Divergence evidence:**
```rust
// compile/mod.rs:243-261
fn digest_step_primitive(hasher: &mut blake3::Hasher, primitive: &vb_yaml::ast::StepPrimitive) {
    match primitive {
        Set { output, value } => { hasher.update(b"set"); hasher.update(output.as_bytes()); hasher.update(value.as_bytes()); }
        Finish { result } => { hasher.update(b"finish"); match result { String(v) => hasher.update(v.as_bytes()), Integer(v) => hasher.update(&v.to_le_bytes()) };}
        other => { hasher.update(canonical_primitive_name(other).as_bytes()); }  // ❌ No Repeat arm — falls through to catch-all
    }
}
```

vs. canonical (part_05.rs:313–319):
```rust
vb_yaml::ast::StepPrimitive::Repeat { max_attempts, body } => {
    hasher.update(b"repeat");
    hasher.update(&max_attempts.to_le_bytes());
    for step in body {
        hasher.update(step.id.as_bytes());
        digest_step_primitive(hasher, &step.primitive)?;
    }
}
```

**Impact**: PO-004 (line 246–295) acknowledges this drift but cannot compare implementations
because the dead one is unreachable. The integration tests (PO-011/PO-012) exercise both
public entry points, but they converge on part_05.rs — the compile/mod.rs path is dead.

**Classification**: Structural drift — dead code creating divergent implementations.

---

### DRIFT-2: Missing Arms in compile/mod.rs::digest_step_primitive

The dead `compile/mod.rs::digest_step_primitive` handles only `Set`, `Finish`, and
falls through to `canonical_primitive_name` for everything else. It is **missing explicit
arms** for: `ForEach`, `Ask`, `Together`, `Collect`, `Repeat`.

**Impact on PO-004**: The harness cannot verify "both implementations equivalent" because
the dead implementation silently falls through for Repeat — it only hashes `b"repeat"` via
`canonical_primitive_name` without consuming `max_attempts` or `body`.

---

### DRIFT-3: Blocker Documentation Is Stale Architecture Debt

`BLOCKER-BLAKE3-INLINEASM` (lines 15–23) documents that blake3's CPU feature detection
uses inline assembly that Kani cannot model. This is a **legitimate technical blocker**,
but the compensation (unit tests, integration tests, proptest) is distributed across:
- `digest_unit_tests.rs` (PO-008–PO-010)
- Integration tests (PO-011–PO-012)
- Proptest (PO-006–PO-007)

This compensation scatter violates **Single Source of Truth** — a verifier reading this
file cannot easily cross-reference where the compensation tests live.

---

## 5. DDD ASSESSMENT (Scott Wlaschin)

### Domain Concepts Present
| Concept | Representation | Status |
|---------|---------------|--------|
| Repeat primitive | `StepPrimitive::Repeat { max_attempts, body }` | Partially correct — fields correct, no Value Object wrapper |
| MaxAttempts | Raw `u16` | ❌ Primitive obsession |
| RepetitionBody | Raw `Vec<StepAst>` | ❌ Primitive obsession |
| StepId | Raw `String` | ❌ Primitive obsession |
| Digest computation | Procedural `digest_step_primitive` match | ✓ Function-based, not hidden in struct methods |

### Workflow Assessment
The `kani_digest_repeat.rs` file is a **verification harness**, not domain logic.
The actual Repeat digest workflow lives in `part_05.rs:313–319`. The harnesses are
correctly structured as formal verification artifacts — they test the workflow, they
are not the workflow.

---

## 6. REQUIRED REFACTORS (Priority Order)

### P0 — Split File (Line Count Violation)

```
kani_digest_repeat.rs (376 lines) must be split into:
├── kani_digest_repeat_max_attempts.rs   (~150 lines: PO-001, PO-003)
├── kani_digest_repeat_body.rs           (~150 lines: PO-002)
├── kani_digest_repeat_idempotency.rs    (~120 lines: PO-004, PO-005)
```

Each sub-file should get its own `#[cfg(kani)]` module with its own docstring
referencing its proof obligations.

### P1 — NewTypes for Domain Primitives (Primitive Obsession)

In the verification helpers (NOT the production code):

```rust
// helpers module — not production, only test/verification
struct MaxAttempts(u16);
struct StepIdStr(String);

impl MaxAttempts {
    fn new(val: u16) -> Self { MaxAttempts(val) }
    fn as_u16(&self) -> u16 { self.0 }
}
```

### P2 — Remove Dead Code

`compile/mod.rs::digest_step_primitive` (lines 243–261) should be deleted or made
private to eliminate the divergent implementation. PO-004's blocker comment confirms
it is unreachable.

### P3 — Consolidate Blocker Compensation References

The comment block (lines 15–26) should link to explicit file paths for the
compensation tests, rather than using vague identifiers (PO-008–PO-012).

---

## 7. VERDICT SUMMARY

| Category | Status | Count |
|----------|--------|-------|
| Line Count | ❌ VIOLATION | 376 > 300 |
| Primitive Obsession | ❌ VIOLATION | 3 violations |
| Dead Code | ❌ VIOLATION | 1 unreachable impl |
| GOD RULES 1-3 | ✅ COMPLIANT | 3/3 |
| DDD Structure | ⚠️ PARTIAL | Domain concepts present, missing Value Objects |
| Blocker Documentation | ⚠️ SCATTERED | Compensation test refs not centralized |

---

## FINAL STATUS

```
STATUS: REFACTOR REQUIRED
Line limit: 376/300 (126% of limit)
Primitive obsession: 3 violations
Dead code divergence: confirmed
```

**Next action**: Split into 3 sub-modules before any new proof obligations are added.
All P1/P2 remediation should happen after the file is split to keep each chunk
under 150 lines.

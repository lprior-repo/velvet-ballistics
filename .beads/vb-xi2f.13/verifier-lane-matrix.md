# Verifier Lane Matrix: vb-xi2f.13

## Lane Applicability Grid

Rows are proof seeds. Columns are verifier lanes. ✅ = required, 🔵 = recommended (conditionally), ❌ = not applicable.

| Proof Seed | Risk Category | Kani | Verus | Flux | Proptest | cargo-fuzz | TLA+ | Loom | Miri |
|---|---|---|---|---|---|---|---|---|---|
| PS-TEMPORAL-001 (layout/width parity) | temporal, layout | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| PS-TEMPORAL-002 (body fallthrough) | temporal, body | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| PS-TEMPORAL-003 (otherwise span) | temporal, otherwise | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| PS-ARITH-001 (width overflow) | arithmetic | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| PS-ARITH-002 (stepidx overflow) | arithmetic | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| PS-INVARIANT-001 (slot unique) | invariant | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| PS-INVARIANT-002 (slot disjoint) | invariant | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| PS-FANOUT-001 (fanout limit) | fanout | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| PS-TYPE-001 (boolean slot) | type | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| PS-LIVENESS-001 (no-otherwise error) | liveness | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| PS-CONCURRENCY-001 (no race) | concurrency | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| PS-INPUT-001 (when parse) | hostile-input | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ |
| PS-INPUT-002 (depth nesting) | hostile-input | ❌ | ❌ | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ |
| PS-EMISSION-PARITY (emission count) | layout | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| PS-YAML-FREE-IR (no YAML in IR) | anti-hallucination | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

## Lane Counts

| Lane | Required | Recommended | Not Applicable | Total |
|---|---|---|---|---|
| Kani | 13 | 0 | 2 | 15 |
| Verus | 1 | 0 | 14 | 15 |
| Flux | 2 | 0 | 13 | 15 |
| Proptest | 5 | 0 | 10 | 15 |
| cargo-fuzz | 2 | 0 | 13 | 15 |
| TLA+ | 0 | 0 | 15 | 15 |
| Loom | 0 | 0 | 15 | 15 |
| Miri | 0 | 0 | 15 | 15 |

## Non-Applicable Rationale Detail

### TLA+ (ALL 15 seeds — not applicable)
The lowering pipeline (`compile_source` → `lower_canonical_choose` → `lower_choose`) is a pure sequential computation that maps YAML AST to IR nodes. There is no temporal behavior, no distributed coordination, no retry/recovery logic, no queue ordering, and no interleaved actor transitions. The workflow-model.md state machine is a computation pipeline, not a concurrent protocol. TLA+ would over-model a deterministic function.

### Loom (14 seeds — not applicable; PS-CONCURRENCY-001 — not required)
PS-CONCURRENCY-001 is explicitly tagged `behavior_affecting: false`. The choose dispatch at runtime is single-threaded per workflow run (`replay_choose_slot` operates on a single `RunFrame`). No `tokio::spawn`, `Arc<Mutex>`, channels, `Atomic*`, or shared-memory concurrency exists in the affected code. Hazard H11 confirms: "No process-level concurrency hazard."

### Miri (ALL 15 seeds — not applicable)
The affected files (`part_01.rs`, `part_02.rs`, `part_04.rs`, `part_05.rs`, `part_06.rs`) contain zero `unsafe` blocks, zero raw pointer operations, zero `MaybeUninit`, zero FFI calls, and zero interior mutability types. The lowering is implemented entirely in safe Rust with checked arithmetic.

### Non-Kani seeds:
- **PS-CONCURRENCY-001**: Not behavior-affecting. A Kani harness would verify a sequential dispatch that is already tested by unit tests.
- **PS-INPUT-002 (kani column)**: The deeply nested choose scenario (exponential blowup) is better suited to fuzzing and proptest for coverage; Kani's bounded model checking cannot effectively explore deep nesting without exponential unwinding. The depth limit is enforced at the YAML parser level (`vb_yaml`), which is outside the scope of this bead's changes.

# Codebase Map - vb-xi2f.33

## Bead
**ID**: vb-xi2f.33
**Title**: P1: digest covers ask semantics

## Summary of Discovery

The canonical digest function (`canonical_digest` / `digest_step_primitive`) in the compiler does NOT hash the semantic fields of the `Ask` primitive. When ask properties change (prompt text, timeout), the digest remains unchanged. This means two workflows that differ only in their ask prompt or timeout will produce identical digests, violating the semantic soundness contract.

The bug exists in **two duplicate implementations** of the same logic:
- `mod_compile_lowering/part_05.rs` (active canonical compilation path)
- `compile/mod.rs` (legacy/simplified compilation path)

Additionally, `compile/mod.rs` does not even support Ask compilation (returns `UnsupportedStepPrimitive`).

## Scope

### Core Files — Digest Computation

| Path | Role | Risk Tags |
|------|------|-----------|
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | **PRIMARY**: `canonical_digest()`, `digest_step_primitive()`, `canonical_primitive_name()` — the semantic digest embedded in `WorkflowParts.digest` during canonical compilation | public-api, semantic-integrity |
| `crates/vb_compile/src/compile/mod.rs` (lines 220-261) | **SECONDARY/DUPLICATE**: Same `canonical_digest()` and `digest_step_primitive()` with identical bug; `compile_source()` at line 25 only supports Set/Do/Finish, returns Unsupported for Ask | public-api, semantic-integrity |
| `crates/vb_compile/src/mod_compile_core.rs` (lines 113-115) | `compute_compiled_digest()` — raw blake3 over source bytes (artifact-level, NOT the semantic digest in the workflow; this one IS sensitive to all changes) | public-api |
| `crates/vb_compile/src/mod_compile_lowering/part_01.rs` (line 46) | Calls `canonical_digest(source)` to produce the digest embedded in the compiled workflow | compilation-pipeline |
| `crates/vb_core/src/ids/mod.rs` (lines 339-356) | `WorkflowDigest` type — 32-byte blake3 hash, `from_bytes()`, `as_bytes()` | type-definition |
| `crates/vb_core/src/workflow/mod.rs` (lines 272-297) | `WorkflowParts` struct — carries the `digest` field that gets embedded in `CompiledWorkflow` | data-structure |
| `crates/vb_core/src/compiled_workflow.rs` | `CompiledWorkflow` type — `digest()` accessor at line 101 | public-api |
| `crates/vb_core/src/nodes.rs` (lines 161-167) | `CompiledNodeKind::Ask { prompt, timeout_slot }` and `CompiledNodeKind::AskResume { answer }` — IR types for ask | data-structure |

### Core Files — Ask Compilation

| Path | Role | Risk Tags |
|------|------|-----------|
| `crates/vb_yaml/src/ast/types.rs` (lines 244-250) | `StepPrimitive::Ask { prompt: String, timeout: Option<String> }` — YAML-level AST type with semantic fields | parser |
| `crates/vb_yaml/src/ast/parse_steps.rs` (lines 314-328) | `parse_ask()` — parses prompt as required string, timeout as optional string from YAML | parser |
| `crates/vb_compile/src/ast/types.rs` (lines 183-191) | `StepKindAst::Ask { prompt, answer, timeout }` — compiler-internal AST (post-slot-resolution) | compilation-pipeline |
| `crates/vb_compile/src/mod_compile_lowering/part_02.rs` (lines 67-69) | Routes `StepPrimitive::Ask` to `lower_canonical_ask()` | compilation-pipeline |
| `crates/vb_compile/src/mod_compile_lowering/part_04.rs` (lines 166-185) | `lower_canonical_ask()` — extracts prompt/timeout strings into slots, calls `lower_ask()` | compilation-pipeline |
| `crates/vb_compile/src/mod_compile_lowering/part_07.rs` (lines 113-152) | `lower_ask()` — emits `CompiledNodeKind::Ask` and `CompiledNodeKind::AskResume` IR nodes | compilation-pipeline |
| `crates/vb_compile/src/mod_compile_lowering/part_08.rs` (lines 214, 233) | Compile primitive dispatch matching for Ask | compilation-pipeline |
| `crates/vb_compile/src/mod_compile_validation/part_02.rs` (line 209) | `Ask` enum variant in `CompilePrimitive` | validation |
| `crates/vb_compile/src/mod_compile_validation/part_03.rs` (lines 26, 45, 162, 212) | Ask shape validation, `validate_ask_shape()` | validation |

### Test Files

| Path | Role | Coverage Gaps |
|------|------|--------------|
| `crates/vb_compile/tests/v1_primitive_lowering.rs` (lines 66-110, 280-283, 592-597, 1002-1003, 1024, 1248-1265) | Primitive lowering tests including ask; tests ID overflow, node kind sequences, slot assertions | **No digest sensitivity tests for ask**; tests only verify IR structure, not that changing ask fields changes digest |
| `crates/vb_compile/src/tests/error_variant_tests.rs` (lines 682-684, 762-803) | `WorkflowDigest::from_bytes` creation test, `compiled_digest_is_deterministic` test, `different_sources_produce_different_digests` test | ONLY tests `compute_compiled_digest` (raw-source blake3), **never tests `canonical_digest`** |
| `crates/vb_compile/tests/vb_core_yaml_e2e_chain_strict_yaml.rs` | E2E YAML compilation | No digest-specific tests |
| `crates/vb_compile/tests/vb_a001_for_each_topology.rs` | ForEach topology tests | Includes Ask/AskResume kind name mapping but no digest tests |
| `crates/vb_compile/src/kani_lower_control.rs` (lines 132-277) | Kani harnesses for ask/repeat lowering overflow; `assert_ask_nodes()`, `assert_ask_start()`, `assert_ask_resume()` | Verifies IR structure but **no Kani harnesses for digest** |
| `crates/vb_compile/src/proptest_error_parity.rs` (lines 64-67) | Proptest ask primitive generation strategy | No digest property tests |
| `crates/vb_core/src/workflow/tests.rs` (lines 3249-3265) | Ask/AskResume node construction tests | Tests node structure only, no digest integration |
| `crates/vb_core/src/engine/step.rs` (lines 457-492) | Ask dispatch in engine with `EngineSignal::AwaitingAsk` tests | Tests runtime behavior, not digest |

### Verification Artifacts

| Path | Status |
|------|--------|
| `verification/` directory | **MISSING**: No TLA+, Verus, or Kani models for canonical digest correctness |
| No digest-related Kani harnesses exist | The `kani_lower_control.rs` proves ask IR shape but not digest coverage |

## Key Symbols

### Digest Functions
- `canonical_digest(source: &WorkflowSource) -> WorkflowDigest` — semantic digest (BUGGY)
- `digest_step_primitive(hasher, primitive)` — dispatches per-primitive hashing (BUGGY for Ask and most primitives)
- `canonical_primitive_name(primitive) -> &'static str` — returns "ask" for Ask (used as only hash input)
- `compute_compiled_digest(source: &[u8]) -> WorkflowDigest` — raw blake3 over bytes (NOT buggy, but not the embedded digest)

### Ask Types
- `vb_yaml::ast::StepPrimitive::Ask { prompt: String, timeout: Option<String> }` — YAML AST
- `vb_compile::ast::StepKindAst::Ask { prompt: SlotIdx, answer: SlotIdx, timeout: Option<SlotIdx> }` — compiler AST
- `vb_core::CompiledNodeKind::Ask { prompt: SlotIdx, timeout_slot: Option<SlotIdx> }` — runtime IR
- `vb_core::CompiledNodeKind::AskResume { answer: SlotIdx }` — runtime IR

## Root Cause Analysis

The `digest_step_primitive` function in both `mod_compile_lowering/part_05.rs` and `compile/mod.rs` uses a catch-all arm:

```rust
other => {
    hasher.update(canonical_primitive_name(other).as_bytes());
}
```

Only `Set` and `Finish` get full field hashing. `Ask` (and `Do`, `Choose`, `ForEach`, `Together`, `Collect`, `Aggregate`, `Repeat`, `Wait`) contribute only their canonical name string to the digest. For Ask specifically, neither the `prompt` text nor the `timeout` value is hashed.

**Impact**: Two workflows with identical structure but different ask prompts will produce identical digests, potentially allowing workflow substitution attacks where a user-facing prompt is silently changed.

## Risk Tags

- **semantic-integrity**: Digest does not capture ask semantics — workflow identity is not a function of its ask content
- **public-api**: The digest is exposed via `CompiledWorkflow::digest()` and `WorkflowParts::digest` and used at runtime for admission/idempotency checks
- **security**: Prompt substitution could go undetected by digest verification

## Open Questions

1. Should `digest_step_primitive` be extended to cover ALL primitives' semantic fields, or just Ask?
2. Should the two duplicate `canonical_digest` implementations be unified into a single canonical version?
3. Should the legacy `compile/mod.rs` path (which doesn't support Ask) be deprecated or removed?
4. What is the correct hash structure for `prompt` (plain text) and `timeout` (optional string)?
5. Should the digest also cover the `Do` (action) primitive's fields for parity?

## Excluded Paths (Not in Scope)

- `crates/vb_runtime/` — runtime engine, not compiler
- `crates/vb_storage/` — persistence layer
- `crates/vb_ipc/` — IPC layer
- `crates/vb_codegen/` — code generation
- `xtask/` — build tooling
- `fuzz/` — fuzzing targets (no ask-specific fuzzing found)
- Non-Ask primitives in digest (broader issue, separate bead)

## Downstream Owners

- **rust-contract**: Defines the digest contract for ask fields (what exactly must be hashed?)
- **proof-planner**: Plans Kani harnesses to prove digest captures ask semantic changes
- **proof-writer**: Writes Kani/proptest harnesses for digest-ask sensitivity
- **test-planner / test-writer**: Adds unit tests for `canonical_digest` Ask coverage
- **holzman-rust**: Implements the fix in `digest_step_primitive()`

## Recommended Fix Strategy

1. In `digest_step_primitive()`, add an explicit `Ask { prompt, timeout }` arm that hashes `b"ask"`, `prompt.as_bytes()`, and `timeout.as_deref().unwrap_or("").as_bytes()` (or a sentinel for `None`)
2. Do the same in both copies (`part_05.rs` and `compile/mod.rs`) OR unify to one
3. Fix `compile/mod.rs::compile_source()` to support Ask compilation if it's still a live path
4. Add tests proving that changing ask prompt or timeout changes the digest

# Red Queen Fuzzing Strategy for Velvet Ballistics

**Version**: 1.0.0  
**Date**: 2026-05-24  
**Status**: Design — not yet implemented  
**Target**: Full adversarial evolutionary fuzzing across all crates, all codec surfaces, and all execution paths.

---

## Table of Contents

1. [Current State Audit](#1-current-state-audit)
2. [Engine Layer Strategy](#2-engine-layer-strategy)
3. [Harness Type Taxonomy](#3-harness-type-taxonomy)
4. [Coverage Matrix: Crate-by-Crate](#4-coverage-matrix-crate-by-crate)
5. [Corpus Strategy](#5-corpus-strategy)
6. [CI Integration Tiers](#6-ci-integration-tiers)
7. [Mutation Resistance Verification](#7-mutation-resistance-verification)
8. [Implementation Roadmap](#8-implementation-roadmap)
9. [Gap Analysis](#9-gap-analysis)
10. [Governance & Ownership](#10-governance--ownership)

---

## 1. Current State Audit

### 1.1 Package Layout

- **Package**: `velvet-ballistics-fuzz` (`fuzz/Cargo.toml`)
- **Library**: `fuzz/src/lib.rs` — 3010 lines, 38 `pub fn fuzz_*` shared harness bodies
- **Bridge module**: `fuzz/fuzz_targets.rs` — 101 lines, thin wrappers for C ABI and callable entrypoints
- **libfuzzer targets**: `fuzz/fuzz_targets/*.rs` — 12 targets using `#![no_main]` + `fuzz_target!` macro
- **stdin-based targets**: `fuzz/src/bin/*.rs` — 47+ targets using stdin read + `fuzz_lib::fuzz_*` dispatch
- **Corpus directories**: 6 active corpora (compiled_ir, decode_record, expr_eval, ipc_frame, journal_event, vb_f04l_yaml_compiler_compile, yaml_events)
- **Script**: `scripts/fuzz-minimization.sh` — libfuzzer minimization wrapper

### 1.2 Harness Patterns Observed

Three distinct patterns exist in the codebase:

**Pattern A — libfuzzer macro** (`fuzz_targets/*.rs`):
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| { /* inline logic */ });
```
Used by: `vb_f04l_yaml_compiler_compile`, `decode_record`, `journal_event`, `vb_storage_codec`, `check_doc_taint_consistency_accepts_arbitrary_markdown`, `ui_redaction_artifact`, `expr_eval` (in fuzz_targets), `lex_expr`, and 4 `vb_5xs4_*` targets.

**Pattern B — stdin dispatch** (`src/bin/*.rs`):
```rust
#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    run_with_stdin(fuzz_lib::fuzz_compiled_ir)
}
```
Used by: 47 targets in `src/bin/`. Every target includes its own copy of `run_with_stdin` and `write_stderr`.

**Pattern C — callable bodies** (`fuzz_targets.rs`):
```rust
pub fn yaml_events(data: &[u8]) {
    fuzz_lib::fuzz_yaml_events(data);
}
```
11 thin wrappers and 5 stub `LLVMFuzzerTestOneInput*` C ABI entries (all currently no-ops returning 0).

### 1.3 Engine Availability

| Engine         | Installed | Crate Dep               | Status            |
|---------------|-----------|-------------------------|--------------------|
| libfuzzer     | ❌ NOT installed | `libfuzzer-sys = "0.4"` | Crate present, binary missing |
| AFL++         | ❌ NOT installed | None                    | No integration     |
| honggfuzz     | ❌ NOT installed | None                    | No integration     |

`cargo-fuzz` binary is **not installed** on the development machine. The `fuzz-minimization.sh` script calls `cargo fuzz run` which will currently fail.

### 1.4 CI Status

Zero fuzzing CI jobs exist. No GitHub Actions workflows, no moon tasks, no cron-driven fuzz campaigns. The `fuzz/` crate is excluded from workspace (`exclude = ["fuzz"]` in root `Cargo.toml`).

---

## 2. Engine Layer Strategy

### 2.1 libfuzzer (Primary — Coverage-Guided Mutation + ASAN/UBSAN)

libfuzzer is the existing investment and should remain the **primary daily-driver engine**.

**Configuration per target** (`.options` files or CLI args):
```
[libfuzzer]
max_len = 65536          # Cap input size; most VB payloads < 16 MiB
runs = 0                 # Run until manually stopped or OOM
max_total_time = 3600    # 1-hour default for campaign mode
detect_leaks = 1
malloc_limit_mb = 2048
close_fd_mask = 3        # Close stdout/stderr in fork mode
```

**Sanitizer matrix** — each target should run through all 3 sanitizers:
| Sanitizer | Cargo flag | Catches |
|-----------|-----------|---------|
| Address (ASAN) | `-Zsanitizer=address` | Heap/stack buffer overflow, use-after-free, double-free |
| Memory (MSAN) | `-Zsanitizer=memory` | Uninitialized reads (requires MSAN-instrumented std) |
| Undefined (UBSAN) | `-Zsanitizer=undefined` | Integer overflow, null deref, misaligned access, type punning |

**Nightly toolchain requirement**: ASAN/UBSAN require `nightly` (the project already pins nightly per `docs/rust-governance.md`).

**Instrumentation profile** — dedicated Cargo profile:
```toml
[profile.fuzz]
inherits = "release"
debug = true
debug-assertions = true
overflow-checks = true
lto = "off"              # LTO breaks ASAN symbolization
opt-level = 2            # Balance coverage speed vs. optimization
codegen-units = 1
```

### 2.2 AFL++ (Secondary — Deterministic + Havoc Mutations)

AFL++ is the **deeper-coverage complementary engine**. Its deterministic stage finds edge cases that libfuzzer's coverage-guided mutation misses, and havoc mode explores combinatorial input spaces.

**Integration approach**: Use `cargo-afl` with the existing stdin-based targets (Pattern B). Those targets already read stdin and call into `fuzz_lib::fuzz_*` — perfect for AFL++'s `@@` file-input model.

```bash
# Build with AFL++ instrumentation
cargo afl build --manifest-path fuzz/Cargo.toml --bin compiled_ir

# Run with deterministic + havoc stages
cargo afl fuzz -i fuzz/corpus/compiled_ir/ -o fuzz/afl_out/compiled_ir/ \
  target/debug/compiled_ir
```

**AFL++ specific benefits for VB**:
- **Deterministic bit flips**: Excellent for catching magic-byte validation bypasses in `vb_storage` codec (MAGIC_JOURNAL_EVENT = 0x56425254, etc.)
- **Dictionary support**: Create AFL dictionary files for VB-specific magic bytes, record kinds, and schema versions to speed up discovery.
- **Persistent mode**: Convert Pattern B targets to AFL persistent loops (`__AFL_LOOP(10000)`) for 10-50x throughput.

**Dictionary file** (`fuzz/dicts/vb_storage.dict`):
```
# Magic bytes
magic_journal="VBRT"
magic_blob="VBBL"
magic_compiled="VBCA"
magic_snapshot="VBSN"
magic_workflow="VBWS"
magic_index="VBIR"

# Record kinds
kind_journal_event="\x00\x00"
kind_workflow_source="\x01\x00"
kind_compiled_ir="\x02\x00"

# Schema version
schema_current="\x01\x00"
```

### 2.3 honggfuzz (Tertiary — Hardware-Assisted Feedback)

honggfuzz provides **hardware counter feedback** (Intel PT/BTS, PERF) that catches paths invisible to software coverage. Use for deep-campaign targets on dedicated hardware.

**Priority targets for honggfuzz** (I/O and compute heavy):
- `vb_storage_codec` — codec state machines have deep branch trees
- `expr_bytecode` — expression evaluation has many opcode paths
- `compiled_ir` — workflow construction has complex validation paths
- `ipc_frame` — frame decoding has format-dependent branches

**Integration**:
```bash
cargo hfuzz run compiled_ir
```

### 2.4 Engine Rotation Strategy

```
                    ┌──────────────────────────────────────┐
                    │           CI Pipeline                 │
                    │                                      │
  PR (smoke) ──────┤  libfuzzer 1 min × 5 seeds/target    │
  Nightly ─────────┤  libfuzzer 1 hr × all targets         │
  Weekly ──────────┤  libfuzzer 12 hr + AFL++ 6 hr         │
  Monthly ─────────┤  libfuzzer 24 hr + AFL++ 12 hr        │
                    │  + honggfuzz 8 hr (top-N targets)    │
                    └──────────────────────────────────────┘
```

---

## 3. Harness Type Taxonomy

Every existing and future fuzz target should be classified into one or more of these harness types. Each type has a specific verification goal and assertion pattern.

### 3.1 Parser/Codec Fuzz Targets

**Goal**: Verify every `decode_*`, `parse_*`, `deserialize_*` function never panics and returns typed errors for all inputs.

**Pattern**:
```rust
fuzz_target!(|data: &[u8]| {
    // 1. Call the parser with arbitrary bytes
    let result = crate::parse_thing(data);
    // 2. Verify Result, never panic
    match result {
        Ok(value) => { /* structural invariants on value */ }
        Err(e) => { /* verify it's a known error variant */ }
    }
});
```

**Existing targets** (15 of 38):
- `yaml_events` — `parse_yaml_events()`, `validate_yaml_profile()`, `build_source_map()`
- `ipc_frame` — `decode_frame_header()`, `decode_frame_payload()`
- `ipc_decode` — `IpcFrameHeader::decode()` with various max_payload bounds
- `journal_event` — `decode_record::<JournalEvent>()`
- `compiled_ir` — `postcard::from_bytes::<WorkflowParts>()` then `try_from_parts()`
- `capability_name_schema` — `validate_with_contracts()` with arbitrary names
- `capability_contract_schema` — `validate_with_contracts()` with arbitrary contracts
- `boundary_inventory_parser` — `parse_inventory()`
- `expr_bytecode` — `postcard::from_bytes::<ExprOp>()` then `try_from_ops()`
- `expression` — expression lexer + parser + compiler + evaluator chain
- `strict_artifact_decoder` — `postcard::from_bytes::<AcceptedArtifact>()` strict decode
- `accepted_artifact_decode` — artifact decode
- `vb_ui_model_postcard_decode` — `decode_postcard::<OutputEnvelope>()`
- `vb_qi37_12_persisted_payload_decode` — persisted payload decode
- `recovery_decode` — recovery record decode

**Missing parser targets** (see §4 Coverage Matrix):
- `vb_expr::parse_expr()` — expression parser on raw token streams
- `vb_storage::decode_slot_written_extra()` — slot extra decode
- `vb_runtime::decode()` (drive retry policy) — drive decode
- `vb_codegen::decode()` — codegen packed decode
- `vb_ui_snapshot::parse_tokens_from_toml()` — UI tokens TOML parser
- `vb_ui_makepad::parse()` — Makepad token parser
- `vb_ui_model::emitter::binary::decode_postcard()` for ALL envelope types
- `vb_boundary_inventory::BoundaryStatus::parse()` — status parse

### 3.2 Roundtrip Fuzz Targets

**Goal**: Verify `encode(decode(x)) == x` or `decode(encode(x)) == decode(x)` for every codec.

**Existing targets** (4 of 38):
- `journal_event` — round-trip encode/decode with equality check
- `ipc_frame` — header encode/decode round-trip with byte-level equality assertion
- `slot_value_roundtrip` — postcard round-trip for SlotValue
- `vb_storage_codec` — comprehensive round-trip covering all magic bytes and record kinds

**Missing roundtrip targets**:
- `WorkflowParts` ↔ postcard ↔ `WorkflowParts` (currently compile-only, no encode path)
- `IpcPayload` ↔ postcard ↔ `IpcPayload` (all 15+ variants)
- `JournalEvent` ↔ postcard ↔ `JournalEvent` (partially covered by journal_event)
- `CompiledWorkflow` → `WorkflowParts` → postcard → `WorkflowParts` (structural roundtrip)
- `ExprProgram` → `ExprOp[]` → `ExprProgram` (ops identity)
- `BoundaryInventory` ↔ `BoundaryInventory` (if serialization roundtrip exists)
- `UiTokens` ↔ `UiTokens` (serialization roundtrip)

### 3.3 Differential Fuzz Targets

**Goal**: Two independent code paths produce the same result for the same input.

**Existing targets** (2 of 38):
- `generated_compare` — `validate_compiled_workflow()` vs. `try_from_parts()` must agree on success/failure
- `digest_coherence` — `blake3::hash()` vs. `verify_digest_match()` must agree

**Missing differential targets**:
- **Direct eval vs. compile+evaluate**: Parse expression → evaluate directly vs. parse expression → compile to bytecode → evaluate bytecode
- **YAML events vs. compiled AST**: `parse_yaml_events(text)` → `YamlCompiler::compile_from_events()` vs. `YamlCompiler::compile(text)`
- **Admission paths**: `admission::validate()` vs. `vb_validate::validate_with_contracts()` for same inputs
- **Workflow graph equivalence**: Two workflows with different YAML structure but same semantic meaning should produce identical `WorkflowDigest`
- **Recovery replay vs. fresh execution**: replay events through recovery engine vs. execute them fresh — should land in same state

### 3.4 Structure-Aware Fuzz Targets

**Goal**: Fuzz inputs that respect the structural grammar, not just random bytes, to reach deep validation paths.

**Approaches**:
1. **`arbitrary` crate**: Implement `Arbitrary` for key domain types, then use `arbitrary::Unstructured` to generate structurally valid but randomized instances
2. **Custom mutators**: Write AFL++ custom mutators that preserve header structure while randomizing payloads
3. **Grammar-based**: Use `libfuzzer` custom mutator to generate valid YAML/postcard structures

**Existing structure-aware targets** (3 of 38):
- `vb_storage_codec` — generates valid events then corrupts specific fields (schema version, header length, record kind)
- `taint_propagation` — builds valid workflow from derived parameters
- `verifier_gates` — builds valid WorkflowParts with randomized node counts and types

**Missing structure-aware targets** (high priority):
- `Arbitrary` impl for `WorkflowParts` — generate structurally valid but randomized workflows
- `Arbitrary` impl for `JournalEvent` — generate valid events with randomized fields
- `Arbitrary` impl for `IpcPayload` — generate valid IPC messages
- `Arbitrary` impl for `ExprOp` sequences — generate semantically diverse bytecode programs
- Custom AFL mutator for `vb_storage` frame format — mutate within struct boundaries

### 3.5 Property Invariant Fuzz Targets

**Goal**: Verify domain invariants hold for all inputs: boundedness, monotonicity, idempotency.

**Existing targets** (5 of 38):
- `resource_budget` — StepBudget exhaustion never panics, zero budget executes zero transitions, executed ≤ budget
- `taint_propagation` — Clean inputs always produce Clean output, taint is monotonic (output taint ≥ input taint)
- `verifier_gates` — All 5 verifier gates never panic, validation errors are typed
- `action_tracker` — Action state machine invariants
- `budget_compute` — Budget arithmetic never overflows

**Missing property targets**:
- **Idempotency**: `run_until_blocked()` applied twice with same budget produces same final state
- **Determinism**: Two runs with same seed workflow and same budget produce identical output
- **Slot monotonicity**: Slot values never decrease in size after assignment
- **Digest stability**: Same `WorkflowParts` always produces same `WorkflowDigest`
- **Admission monotonicity**: If admission accepts artifact A at step N, it also accepts A at step N+1 with same policy
- **Recovery convergence**: Replaying any sequence of valid events from initial state reaches a consistent terminal state
- **Page boundary invariants**: `collect_page` output invariants (page count ≤ total items, each page ≤ page_size)
- **Queue ordering**: `ActionQueue` preserves FIFO ordering across push/pop cycles

### 3.6 Hostile Input Fuzz Targets

**Goal**: Verify graceful handling of deliberately invalid inputs: wrong magic, truncated frames, corrupted digests, overflow lengths.

**Existing targets** (6 of 38):
- `decode_record` — exercises ALL magic bytes including 0x00000000 and 0xFFFFFFFF
- `vb_storage_codec` — schema version ±1, unknown record kinds, wrong header length, payload truncation
- `ipc_frame` — truncated headers (0..IPC_HEADER_LEN), payload length mismatches
- `ipc_frame_fuzz_boundary` — boundary values in IPC frame fields
- `storage_envelope_fuzz_boundary` — boundary values in storage envelope
- `binary_payload_fuzz_boundary` — boundary values in binary payloads

**Missing hostile targets**:
- **All-zeros and all-0xFF buffers** at max payload size — verifies no infinite loops or OOM
- **Unicode edge cases**: Null bytes mid-string, invalid UTF-8 sequences, bidirectional override characters, extremely long grapheme clusters
- **Integer overflow probes**: Postcard length prefixes at `u32::MAX`, `u64::MAX`, negative values via zigzag
- **Recursive/cyclic structures**: YAML anchors with cycles (should be rejected, not loop)
- **Resource exhaustion**: Deeply nested expressions, exponentially large workflows, massive slot counts
- **Timing side-channels**: Identical outputs for inputs differing in secret bits

---

## 4. Coverage Matrix: Crate-by-Crate

### 4.1 `vb_core` — Engine, Workflow, IDs, Budget, Taint

| Surface                | Current Coverage | Harness Type         | Priority | Notes |
|------------------------|-----------------|----------------------|----------|-------|
| `WorkflowParts` (postcard decode) | ✅ compiled_ir, generated_compare | Parser + Differential | — | Strong coverage |
| `CompiledWorkflow::try_from_parts()` | ✅ compiled_ir | Parser | — | Validated with slot bounds |
| `RunFrame::new()` | ✅ resource_budget, taint_propagation | Property | — | Used as scaffolding |
| `engine::run_until_blocked()` | ✅ resource_budget | Property | — | Budget exhaustion verified |
| `StepBudget::new()` / `try_take()` | ✅ budget_compute, step_budget_new | Property | — | |
| `join_taint()` | ✅ taint_propagation | Property | — | Monotonicity verified |
| `ExprOp` bytecode | ✅ expr_bytecode, expression | Parser | — | |
| `ExprProgram::try_from_ops()` | ✅ expr_bytecode | Parser | — | |
| `ActionContract` validation | ✅ capability_name_schema, capability_contract_schema | Parser | — | |
| `StepState` machine | ❌ MISSING | Property | HIGH | State transition invariants |
| `ValueStore` operations | ❌ MISSING | Property | HIGH | Read-after-write consistency |
| `CompiledNodeKind` exhaustive | ✅ verifier_gates | Property | — | All variants exercised |
| `Diagnostic::add_*` | ❌ MISSING | Parser | MEDIUM | Never panics with extreme values |

### 4.2 `vb_yaml` — YAML Event Parser

| Surface                | Current Coverage | Harness Type         | Priority | Notes |
|------------------------|-----------------|----------------------|----------|-------|
| `parse_yaml_events()` | ✅ yaml_events | Parser | — | Via saphyr |
| `validate_yaml_profile()` | ✅ yaml_events, strict_yaml_profile | Parser | — | |
| `build_source_map()` | ✅ yaml_events | Parser | — | |
| `parse_workflow_source()` | ❌ MISSING | Parser | HIGH | Top-level YAML→AST path |
| `YamlEvent` type exhaustiveness | ❌ MISSING | Parser | MEDIUM | All event variants exercised |
| `events_types.rs` all variants | ❌ MISSING | Parser | MEDIUM | Variant match coverage |
| YAML with BOM, tabs, CRLF | ❌ MISSING | Hostile | MEDIUM | Encoding edge cases |

### 4.3 `vb_compile` — YAML→Workflow Compiler

| Surface                | Current Coverage | Harness Type         | Priority | Notes |
|------------------------|-----------------|----------------------|----------|-------|
| `YamlCompiler::compile()` | ✅ vb_f04l_yaml_compiler_compile | Parser | — | All error variants matched |
| `parse_expression()` | ❌ MISSING | Parser | HIGH | Expression compilation |
| `parse_ast()` | ❌ MISSING | Parser | HIGH | AST parsing from bytes |
| Expression bytecode compilation | ✅ expression (via chain) | Parser | — | Indirect coverage |
| `mod_compile_core` | ❌ MISSING | Parser | HIGH | Core compilation logic |
| Duplicate step name detection | ❌ MISSING | Hostile | MEDIUM | Exhaustive CompileError coverage |

### 4.4 `vb_expr` — Expression Evaluator

| Surface                | Current Coverage | Harness Type         | Priority | Notes |
|------------------------|-----------------|----------------------|----------|-------|
| `parse_expr()` | ❌ MISSING (lex_expr is lexer-only) | Parser | HIGH | Token→AST |
| `ExprAst` evaluation | ✅ expr_eval, expr_bytecode | Parser + Property | — | |
| Expression bytecode compilation | ✅ expression, expr_bytecode | Parser + Roundtrip | — | |
| `builtin_eval` functions | ❌ MISSING | Property | HIGH | Each built-in exercised |
| Constant folding (`bytecode/fold.rs`) | ❌ MISSING | Differential | HIGH | Folded vs. unfixed eval must agree |
| `environment.rs` value scope | ❌ MISSING | Property | MEDIUM | Scope isolation invariants |

### 4.5 `vb_storage` — Journal Codec, Admission, Recovery

| Surface                | Current Coverage | Harness Type         | Priority | Notes |
|------------------------|-----------------|----------------------|----------|-------|
| `decode_record_header()` | ✅ decode_record, vb_storage_codec | Parser + Hostile | — | All magics, all bounds |
| `decode_record::<T>()` | ✅ decode_record, journal_event, vb_storage_codec | Parser + Roundtrip | — | Comprehensive |
| `encode_record()` | ✅ journal_event, vb_storage_codec | Roundtrip | — | |
| `verify_digest_match()` | ✅ digest_coherence, vb_storage_codec | Differential + Hostile | — | |
| `parse_event()` | ✅ journal_event (fuzz_targets) | Parser | — | B11/B12 gates verified |
| `decode_slot_written_extra()` | ❌ MISSING | Parser | HIGH | Slot extra header |
| `admission` module | ✅ admission_flow, admission_fuzz, admission_input_surface | Parser + Property | — | |
| `trimming` module | ❌ MISSING | Property | MEDIUM | Trim invariants |
| `recovery` module | ✅ recovery_decode, replay_events | Parser | — | |
| `batch.rs` batch operations | ❌ MISSING | Property | MEDIUM | Batch atomicity invariants |
| `binary.rs` binary codec | ✅ binary_payload_fuzz_boundary | Hostile | — | |
| `blob_tests` (existing unit tests) | ✅ codec_miri_tests | Miri | — | Miri coverage exists |

### 4.6 `vb_runtime` — Runtime Engine, Action Queue, Admission

| Surface                | Current Coverage | Harness Type         | Priority | Notes |
|------------------------|-----------------|----------------------|----------|-------|
| `admission.rs` | ✅ admission_flow, admission_fuzz | Parser + Property | — | |
| `action.rs` | ✅ action_tracker | Property | — | |
| `action_queue.rs` | ❌ MISSING | Property | HIGH | FIFO ordering, capacity bounds |
| `counters.rs` | ❌ MISSING | Property | MEDIUM | Counter overflow, monotonicity |
| `durability_matrix.rs` | ❌ MISSING | Property | MEDIUM | Matrix invariants |
| `frame_pool.rs` | ❌ MISSING | Property | MEDIUM | Pool allocation/reuse |
| `idempotency.rs` | ❌ MISSING | Property | HIGH | Idempotency key collision handling |
| `primitives/retry.rs` | ❌ MISSING | Parser | HIGH | `decode()` and retry policy construction |
| `journal.rs` (thin) | ✅ journal_event | — | — | |
| `collect_tests.rs` (unit tests) | — | — | — | 103K lines unit tests |

### 4.7 `vb_ipc` — IPC Wire Protocol

| Surface                | Current Coverage | Harness Type         | Priority | Notes |
|------------------------|-----------------|----------------------|----------|-------|
| `decode_frame_header()` | ✅ ipc_frame, ipc_decode | Parser + Roundtrip | — | Strong coverage |
| `decode_frame_payload()` | ✅ ipc_frame | Parser | — | All payload variants |
| `IpcFrameHeader::decode()` with bounds | ✅ ipc_decode | Parser + Hostile | — | Truncation + bounds |
| `encode()` (header) | ✅ ipc_frame | Roundtrip | — | |
| `decode()` (frame_types) | ❌ MISSING | Parser | HIGH | `frame_types::decode()` |
| `decode_frame()` (frame_types) | ❌ MISSING | Parser | HIGH | Full frame decode |
| `codec::decode_payload()` | ❌ MISSING | Parser | HIGH | Direct codec path |
| `bounded.rs` | ❌ MISSING | Property | MEDIUM | Bounded buffer invariants |
| `action_output.rs` | ❌ MISSING | Roundtrip | MEDIUM | ActionOutput serialize/deserialize |
| `payloads.rs` | ❌ MISSING | Roundtrip | MEDIUM | All payload type roundtrips |
| `commands.rs` | ❌ MISSING | Hostile | LOW | Command parsing edge cases |
| `metrics.rs` | ❌ MISSING | Property | LOW | Counter overflow, reset behavior |

### 4.8 `vb_validate` — Plan Verification Gates

| Surface                | Current Coverage | Harness Type         | Priority | Notes |
|------------------------|-----------------|----------------------|----------|-------|
| Gate 7 (stack depth) | ✅ verifier_gates | Property | — | |
| Gate 8 (accessor) | ✅ verifier_gates | Property | — | |
| Gate 9 (slot usage) | ✅ verifier_gates | Property | — | |
| Gate 11 (action outputs) | ✅ verifier_gates | Property | — | |
| Gate 13 (loop detection) | ✅ verifier_gates | Property | — | |
| `validate_with_contracts()` | ✅ capability_name_schema, capability_contract_schema | Parser | — | |
| `diagnostic.rs` collector | ❌ MISSING | Property | MEDIUM | Never panics |
| `diag_render.rs` | ❌ MISSING | Parser | LOW | Output stability |

### 4.9 `vb_ui_model` — UI Output Model

| Surface                | Current Coverage | Harness Type         | Priority | Notes |
|------------------------|-----------------|----------------------|----------|-------|
| `decode_postcard::<OutputEnvelope>()` | ✅ vb_ui_model_postcard_decode | Parser | — | |
| `emitter/binary/mod.rs` all types | ❌ MISSING | Roundtrip | HIGH | All binary-emitted types |
| `emitter/yaml.rs` YAML emission | ❌ MISSING | Roundtrip | MEDIUM | YAML roundtrip |
| `canonical.rs` canonicalization | ❌ MISSING | Property | MEDIUM | Canonical form stability |
| `emitter/error.rs` error paths | ❌ MISSING | Hostile | LOW | Error rendering |

### 4.10 `vb_boundary_inventory` — Boundary Inventory Parser

| Surface                | Current Coverage | Harness Type         | Priority | Notes |
|------------------------|-----------------|----------------------|----------|-------|
| `parse_inventory()` | ✅ boundary_inventory_parser | Parser | — | |
| `inventory.rs` types | ❌ MISSING | Property | MEDIUM | Structural invariants |
| `BoundaryStatus::parse()` | ❌ MISSING | Parser | MEDIUM | Status string parse |
| `status.rs` status enum | ❌ MISSING | Parser | LOW | All status variants |

### 4.11 `vb_ui_snapshot` — UI Snapshot Tests

| Surface                | Current Coverage | Harness Type         | Priority | Notes |
|------------------------|-----------------|----------------------|----------|-------|
| `scan_release_artifact()` | ✅ ui_redaction_artifact | Parser | — | Sentinel detection |
| `parse_tokens_from_toml()` | ❌ MISSING | Parser | HIGH | TOML token parser |
| `fixture_parser.rs` | ❌ MISSING | Parser | MEDIUM | Fixture format parser |
| `redaction.rs` full module | ❌ MISSING | Parser | MEDIUM | All redaction rules |

### 4.12 `vb_codegen` — Code Generation

| Surface                | Current Coverage | Harness Type         | Priority | Notes |
|------------------------|-----------------|----------------------|----------|-------|
| `decode()` (packed) | ❌ MISSING | Parser | HIGH | Packed i64 decode |
| Generated code vs. source | ❌ MISSING | Differential | HIGH | Generated IR must match original |

### 4.13 `vb_ui_makepad` — UI Tokens

| Surface                | Current Coverage | Harness Type         | Priority | Notes |
|------------------------|-----------------|----------------------|----------|-------|
| `parse()` (tokens) | ❌ MISSING | Parser | HIGH | Makepad token parser |
| `parse_hex()` | ❌ MISSING | Parser | MEDIUM | Hex color parser |

### 4.14 `vb_doc` — Documentation Reconciliation

| Surface                | Current Coverage | Harness Type         | Priority | Notes |
|------------------------|-----------------|----------------------|----------|-------|
| `check_doc_taint_consistency()` | ✅ check_doc_taint_consistency | Parser | — | Markdown parse + assertion checks |

### 4.15 `xtask` — Build Tooling

| Surface                | Current Coverage | Harness Type         | Priority | Notes |
|------------------------|-----------------|----------------------|----------|-------|
| `parse_xtask_command()` | ✅ xtask_parse_argv_hostile, xtask_parse_options_hostile | Hostile | — | |
| Option parsing | ✅ xtask_parse_options_hostile | Hostile | — | |

---

## 5. Corpus Strategy

### 5.1 Seed Generation

Seeds should be generated from **existing valid test fixtures** and **production data snapshots**.

**Source 1: Test YAML fixtures** (`crates/workspace_tests/tests/fixtures/`)
```
valid/minimal.yaml                    → corpus/yaml_events/
valid/*.yaml                          → corpus/vb_f04l_yaml_compiler_compile/
pgo/minimal_save.yaml                 → corpus/yaml_events/
pgo/choose_true.yaml                  → corpus/yaml_events/
```

**Seed generation script** (`scripts/fuzz-seed-gen.sh`):
```bash
#!/usr/bin/env bash
# Generate fuzz corpus seeds from test fixtures
set -euo pipefail

CORPUS_BASE="fuzz/corpus"

# YAML fixtures → yaml_events corpus
for yaml in crates/workspace_tests/tests/fixtures/valid/*.yaml \
            crates/workspace_tests/tests/fixtures/pgo/*.yaml; do
    cp "$yaml" "$CORPUS_BASE/yaml_events/$(basename "$yaml")"
done

# YAML fixtures → vb_f04l_yaml_compiler_compile corpus
for yaml in crates/workspace_tests/tests/fixtures/valid/*.yaml; do
    cp "$yaml" "$CORPUS_BASE/vb_f04l_yaml_compiler_compile/$(basename "$yaml")"
done

# Generate postcard seeds from known-good test artifacts
# (requires test artifacts to be pre-generated via `cargo test -- --nocapture save-artifacts`)
```

**Source 2: Postcard/protobuf snapshots** from integration tests — serialize known-good domain objects and save as seeds.

**Source 3: Minimum-interesting inputs** — hand-crafted edge cases:
- Empty workflow (0 nodes)
- Single-node workflow (Nop / Finish)
- Maximum-depth workflow (cyclic retry chains)
- All-taint-level workflows
- Every `CompiledNodeKind` variant in isolation

### 5.2 Corpus Minimization

Use `cargo fuzz cmin` (libfuzzer corpus minimization) and `afl-cmin` (AFL corpus minimization):

```bash
# libfuzzer minimization (coverage-based)
cargo fuzz cmin --target x86_64-unknown-linux-gnu compiled_ir

# AFL minimization (run-time based)
afl-cmin -i fuzz/corpus/compiled_ir/ -o fuzz/corpus/compiled_ir.min/ \
  target/fuzz/compiled_ir
```

**Minimization schedule**: After every nightly campaign, minimize corpora before the weekly deep campaign.

### 5.3 Corpus Merging Across Targets

Corpora should be merged across related targets to share coverage discoveries:

```
yaml_events corpus ─────┐
                        ├──→ vb_f04l_yaml_compiler_compile (merged)
compiled_ir corpus ─────┘

journal_event corpus ───┐
                        ├──→ decode_record (merged)
recovery_decode corpus ─┘

ipc_frame corpus ───────┐
                        ├──→ ipc_decode (merged)
```

**Merge script** (`scripts/fuzz-merge-corpus.sh`):
```bash
# Merge corpora for related targets
cargo fuzz merge --target x86_64-unknown-linux-gnu \
  compiled_ir \
  fuzz/corpus/yaml_events/ \
  fuzz/corpus/vb_f04l_yaml_compiler_compile/
```

### 5.4 Corpus Versioning

Corpora should be checked into the repository (they're small binary blobs ≤50MB total) to enable deterministic regression replay.

```
fuzz/corpus/
├── compiled_ir/          # git tracked
├── decode_record/         # git tracked
├── expr_eval/             # git tracked
├── ipc_frame/             # git tracked
├── journal_event/         # git tracked
├── vb_f04l_yaml_compiler_compile/  # git tracked
├── yaml_events/           # git tracked
└── .gitkeep
```

Add to `.gitattributes`:
```
fuzz/corpus/** filter=lfs diff=lfs merge=lfs -text
```

---

## 6. CI Integration Tiers

### 6.1 Tier 1: PR Smoke (`fuzz-smoke` — ~2 min)

**Trigger**: Every PR push  
**Scope**: All 40+ targets, 10 seeds each, max 30s per target  
**Command pattern**:
```yaml
# .github/workflows/fuzz-smoke.yml
- name: Fuzz smoke
  run: |
    for target in $(cargo fuzz list --manifest-path fuzz/Cargo.toml); do
      cargo fuzz run "$target" \
        --target x86_64-unknown-linux-gnu \
        -- -max_total_time=30 -runs=10 -max_len=4096
    done
```

**Pass criteria**: Zero crashes, zero ASAN/UBSAN violations. Timeout on single-target is acceptable (indicates hang, not crash).

**Moon task** (`moon.yml`):
```yaml
tasks:
  fuzz-smoke:
    command: "bash scripts/fuzz-smoke.sh"
    inputs:
      - "fuzz/**"
      - "crates/**/src/**"
    platform: "system"
```

### 6.2 Tier 2: Nightly Campaign (`fuzz-nightly` — ~2 hours)

**Trigger**: Cron schedule (02:00 UTC daily)  
**Scope**: All targets, 1 hour each  
**Sanitizers**: ASAN only (MSAN/UBSAN too expensive for nightly)  
**Command**:
```yaml
- name: Fuzz nightly
  run: |
    for target in $(cargo fuzz list --manifest-path fuzz/Cargo.toml); do
      timeout 3600 cargo fuzz run "$target" \
        --target x86_64-unknown-linux-gnu \
        -- -max_total_time=3540 -max_len=65536 || true
      # Store crash artifacts
      mkdir -p fuzz-artifacts/"$target"
      cp fuzz/artifacts/"$target"/* fuzz-artifacts/"$target"/ 2>/dev/null || true
    done
```

**Post-campaign actions**:
1. Upload crash artifacts as CI artifacts (retention: 30 days)
2. Run corpus minimization on all targets
3. Open bead for any discovered crashes
4. Email/notify on any new unique crash

### 6.3 Tier 3: Weekly Deep Campaign (`fuzz-deep` — ~24 hours)

**Trigger**: Cron schedule (Saturday 00:00 UTC)  
**Scope**: Top-15 high-priority targets, 12-24 hours each  
**Dual-engine**: libfuzzer 12h + AFL++ 12h on different machines  
**Command**:
```yaml
- name: Fuzz weekly deep
  run: |
    # Phase 1: libfuzzer 12h
    for target in $HIGH_PRIORITY_TARGETS; do
      cargo fuzz run "$target" \
        --target x86_64-unknown-linux-gnu \
        -- -max_total_time=43200 &
    done
    wait

    # Phase 2: corpus merge
    bash scripts/fuzz-merge-corpus.sh

    # Phase 3: AFL++ 12h
    for target in $HIGH_PRIORITY_TARGETS; do
      cargo afl fuzz -i fuzz/corpus/"$target"/ -o fuzz/afl_out/"$target"/ \
        -V 43200 target/fuzz/"$target" &
    done
    wait
```

**High-priority targets for deep campaigns**:
1. `vb_storage_codec` — most complex codec, all magic variants
2. `expr_bytecode` — arbitrary opcode sequences
3. `compiled_ir` — workflow construction validation
4. `ipc_frame` — all IPC payload variants
5. `journal_event` — journal codec roundtrip
6. `vb_f04l_yaml_compiler_compile` — YAML→workflow compiler
7. `verifier_gates` — all verification gates
8. `taint_propagation` — taint monotonicity
9. `resource_budget` — budget exhaustion paths
10. `admission_flow` — admission state machine
11. `slot_value_roundtrip` — postcard roundtrip
12. `generated_compare` — differential validation
13. `digest_coherence` — digest verification
14. `expr_eval` — expression evaluation
15. `expression` — full expression pipeline

### 6.4 OSS-Fuzz Style Continuous Fuzzing

For production-grade continuous fuzzing, integrate with **ClusterFuzzLite** or a self-hosted **OSS-Fuzz compatible runner**.

**Dockerfile** (`fuzz/Dockerfile`):
```dockerfile
FROM gcr.io/oss-fuzz-base/base-builder-rust
RUN apt-get update && apt-get install -y make autoconf automake libtool
COPY . $SRC/velvet-ballistics
WORKDIR $SRC/velvet-ballistics
COPY fuzz/build.sh $SRC/
```

**Build script** (`fuzz/build.sh`):
```bash
cd $SRC/velvet-ballistics
cargo fuzz build --target x86_64-unknown-linux-gnu -O
for target in $(cargo fuzz list --manifest-path fuzz/Cargo.toml); do
    cp fuzz/target/x86_64-unknown-linux-gnu/release/$target $OUT/
done
```

### 6.5 CI Budget Estimation

| Tier        | Frequency | Targets | Time/Target | Total CPU-min/month | Cost (GHA @ $0.008/min) |
|------------|-----------|---------|-------------|--------------------|-----------------------|
| Smoke      | 30 PRs/day | 40 | 0.5 min | 600 min/day | ~$150/month |
| Nightly    | Daily | 40 | 60 min | 2,400 min/day | ~$600/month |
| Weekly     | Weekly | 15 | 1,440 min | 21,600 min/week | ~$175/week (~$700/month) |
| **Total** | | | | | **~$1,450/month** |

---

## 7. Mutation Resistance Verification

### 7.1 Crash Regression Tests

Every crash discovered by fuzzing must become a **permanent regression test** that verifies the fix.

**Process**:
1. Fuzzer discovers crash → crash artifact stored in `fuzz/artifacts/<target>/crash-<hash>`
2. Developer fixes the bug
3. Create regression test: `crates/workspace_tests/tests/fuzz_regressions.rs`
4. Add `#[test] fn regression_<bead_id>_<crash_hash>()` that replays the exact crash input
5. Assert that the fixed code returns `Ok` or a specific error, not a panic

**Example**:
```rust
#[test]
fn regression_vb_abc123_crash_sha256() {
    let input = include_bytes!("../../fuzz/artifacts/journal_event/crash-abc123");
    let result = vb_storage::journal::parse_event(input);
    assert!(result.is_err()); // Previously panicked, now returns error
}
```

### 7.2 Mutation Testing on Fuzz Harnesses

Verify that fuzz harnesses are **sensitive enough** to catch injected bugs. Use `cargo-mutants` (already in the verification fleet) with fuzz harnesses as test targets.

**Process**:
```bash
# Run mutation testing on fuzz harness library
cargo mutants -p velvet-ballistics-fuzz -- --test-fuzz-smoke

# Verify: mutants in codec validation logic SHOULD be caught by fuzz harnesses
# If a mutation is NOT caught, the harness needs strengthening
```

**Mutation resistance score**: Target ≥90% — meaning 90% of injected mutations are caught by existing fuzz harnesses.

### 7.3 Determinism Replay

Every fuzz target must be **deterministic** — replaying the same corpus seed should produce the same result 100% of the time.

**Verification script** (`scripts/fuzz-determinism-check.sh`):
```bash
#!/usr/bin/env bash
set -euo pipefail
target="$1"
seed_dir="fuzz/corpus/$target"

for seed in "$seed_dir"/*; do
    out1=$(mktemp)
    out2=$(mktemp)
    cargo fuzz run "$target" "$seed" -- -runs=1 > "$out1" 2>&1
    cargo fuzz run "$target" "$seed" -- -runs=1 > "$out2" 2>&1
    diff "$out1" "$out2" || {
        echo "NON-DETERMINISM in $target with seed $seed"
        exit 1
    }
    rm "$out1" "$out2"
done
echo "All seeds deterministic for $target"
```

### 7.4 Assertion Strength Audit

Review every fuzz harness assertion for **strength**. Weak assertions produce green runs without meaningful coverage.

**Assertion strength scale**:
| Level | Example | Score |
|-------|---------|-------|
| L0: None | `let _ = parse(data);` | FAIL |
| L1: No-panic | `let _ = parse(data);` (implicit) | Minimal |
| L2: Error typing | `assert!(matches!(result, Err(_)))` | Weak |
| L3: Specific errors | `assert!(matches!(result, Err(JournalError::BadMagic{..})))` | Good |
| L4: Structural invariants | `assert!(event.is_valid())` | Strong |
| L5: Equivalence properties | `assert_eq!(decoded, original)` | Excellent |
| L6: Differential | `assert_eq!(path_a(data), path_b(data))` | Gold |

**Current audit findings**:
- `boundary_inventory_parser` — L1 (no-panic only) — **needs strengthening to L3**
- `accepted_artifact_envelope_qi37_4_2` — L1 (field access only) — **needs strengthening**
- `readback_family_set` — stub (no assertions) — **needs implementation**
- `journal_event` (lib.rs) — L5 (roundtrip equality + error typing) — **excellent**
- `ipc_frame` — L5 (header roundtrip + error typing) — **excellent**
- `vb_storage_codec` — L5+ (roundtrip + hostile mutations) — **gold standard**

---

## 8. Implementation Roadmap

### Phase 1: Foundation (Week 1-2)
- [ ] Install `cargo-fuzz` binary (`cargo install cargo-fuzz`)
- [ ] Install `cargo-afl` and AFL++ runtime (`apt install afl++`)
- [ ] Install `cargo-honggfuzz` (`cargo install honggfuzz`)
- [ ] Create `[profile.fuzz]` in root `Cargo.toml`
- [ ] Create `.options` files for top-15 targets (max_len, detect_leaks, etc.)
- [ ] Refactor `src/bin/*.rs` to eliminate duplicated `run_with_stdin` — extract into `fuzz/src/stdin_runner.rs`
- [ ] Create `scripts/fuzz-smoke.sh` for CI
- [ ] Create `scripts/fuzz-seed-gen.sh` for corpus seeding

### Phase 2: Fill Gaps (Week 3-4)
- [ ] Implement 11 HIGH-priority missing fuzz targets (see §4 Coverage Matrix)
- [ ] Strengthen 3 weakest existing harnesses (L1→L3)
- [ ] Implement `Arbitrary` for `WorkflowParts`, `JournalEvent`, `IpcPayload`, `ExprOp`
- [ ] Create AFL dictionary files for `vb_storage` codec
- [ ] Write fuzz regression tests for any known historical crashes

### Phase 3: CI Integration (Week 5-6)
- [ ] Create `.github/workflows/fuzz-smoke.yml`
- [ ] Create `.github/workflows/fuzz-nightly.yml`
- [ ] Create `.github/workflows/fuzz-deep.yml`
- [ ] Add `moon fuzz-smoke` task
- [ ] Configure artifact upload for crash files
- [ ] Set up notification on new unique crashes

### Phase 4: Advanced (Week 7+)
- [ ] Implement AFL persistent mode for top-10 targets (10-50x throughput)
- [ ] Write custom AFL mutators for postcard and YAML formats
- [ ] Set up ClusterFuzzLite integration
- [ ] Run mutation testing on fuzz harnesses to verify sensitivity
- [ ] Implement corpus merging automation

---

## 9. Gap Analysis

### 9.1 Critical Gaps (Blocking)

| Gap | Impact | Resolution |
|-----|--------|-----------|
| `cargo-fuzz` binary not installed | Cannot run any fuzz target | Install via `cargo install cargo-fuzz` |
| Zero CI fuzz jobs | No automated crash detection | Implement Phase 3 |
| 11 HIGH-priority missing targets | Blind spots in validation surface | Implement Phase 2 |
| `xtask_parse_argv_hostile` depends on xtask crate | Circular dependency risk | Verify xtask is a workspace dep |

### 9.2 Architectural Gaps

| Gap | Impact | Resolution |
|-----|--------|-----------|
| Duplicated `run_with_stdin` in 47 files | Maintenance burden, inconsistency | Extract to shared module |
| No `Arbitrary` impls for core types | Can't do structure-aware fuzzing | Implement for WorkflowParts, JournalEvent, IpcPayload |
| No custom AFL mutators | AFL efficiency limited for binary formats | Write postcard-aware mutator |
| C ABI stubs are no-ops | `LLVMFuzzerTestOneInput*` functions return 0 | Wire to real fuzz bodies or remove |
| `fuzz/` excluded from workspace | Can't use workspace Cargo features | Consider conditional inclusion |

### 9.3 Coverage Gaps by Harness Type

| Type | Current Count | Target Count | Gap |
|------|--------------|-------------|-----|
| Parser/Codec | 15 | 28 | 13 missing |
| Roundtrip | 4 | 12 | 8 missing |
| Differential | 2 | 8 | 6 missing |
| Structure-aware | 3 | 10 | 7 missing |
| Property invariant | 5 | 16 | 11 missing |
| Hostile input | 6 | 14 | 8 missing |

---

## 10. Governance & Ownership

### 10.1 Bead Tracking

All fuzz-related work should be tracked as beads:
- Each missing target → 1 bead (HIGH priority)
- Each harness strengthening → 1 bead (MEDIUM)
- CI integration → 1 bead per workflow
- Regression from crash fix → 1 bead per crash

### 10.2 Fuzz Health Dashboard

Create a dashboard tracking:
- Total fuzz targets: 40 (current) → 65+ (target)
- Harnesses at L3+: 25 current → 50+ target
- Corpus seeds per target: 0-30 current → 50+ per target target
- Unique crashes/month: 0 current (no CI) → ≥1 target (stretch)
- Mutation resistance score: UNKNOWN → ≥90% target

### 10.3 Review Checklist

Every PR affecting a crates' public API must:
1. Check if the change touches a `decode_*`/`parse_*`/`serialize_*` function
2. If yes: verify a fuzz harness exists and covers the changed path
3. If no harness exists: create one (or file a bead)
4. Run `fuzz-smoke` on the affected target
5. Ensure no regression in existing corpus seeds

### 10.4 Formal Verification Synergy

Fuzz findings should feed into formal verification:
- **Kani**: Crash inputs become Kani harness seeds
- **Verus**: Invariant violations discovered by fuzzing become `proof fn` obligations
- **TLA+**: State space exploration in fuzzing validates TLA+ model accuracy
- **Miri**: Fuzz-discovered UB patterns become Miri test cases
- **Flux**: Integer overflow crashes inform refinement bounds

---

## Appendix A: Target Inventory

### A.1 Complete Fuzz Target List (47 total)

**fuzz_targets/*.rs (libfuzzer, 12 targets)**:
1. `vb_f04l_yaml_compiler_compile`
2. `decode_record`
3. `journal_event`
4. `vb_storage_codec`
5. `check_doc_taint_consistency_accepts_arbitrary_markdown`
6. `ui_redaction_artifact`
7. `expr_eval` (libfuzzer variant)
8. `lex_expr`
9. `vb_5xs4_generated_source_mapping`
10. `vb_5xs4_inventory_report`
11. `vb_5xs4_label_sufficiency`
12. `vb_5xs4_scan_source_text`

**src/bin/*.rs (stdin dispatch, 35 targets)**:
1. `vb_qi37_12_persisted_payload_decode`
2. `journal_event` (stdin variant)
3. `capability_name_schema`
4. `capability_contract_schema`
5. `compiled_ir`
6. `expression`
7. `generated_compare`
8. `collect_page_pagination`
9. `yaml_events`
10. `ipc_frame`
11. `strict_artifact_decoder`
12. `digest_coherence`
13. `readback_family_set`
14. `admission_input_surface`
15. `accepted_artifact_decode`
16. `accepted_artifact_envelope_qi37_4_2`
17. `accessor_traversal`
18. `action_tracker`
19. `admission_flow`
20. `admission_fuzz`
21. `binary_payload_fuzz_boundary`
22. `budget_compute`
23. `expr_bytecode`
24. `expr_eval`
25. `external_input_adapter_fuzz`
26. `extract_terminal`
27. `ipc_decode`
28. `ipc_frame_fuzz_boundary`
29. `recovery_decode`
30. `replay_events`
31. `resource_budget`
32. `slot_value_roundtrip`
33. `step_budget_new`
34. `storage_envelope_fuzz_boundary`
35. `strict_yaml_profile`
36. `taint_propagation`
37. `vb_ui_model_postcard_decode`
38. `verifier_gates`
39. `boundary_inventory_parser` (libfuzzer, in src/bin)
40. `xtask_parse_argv_hostile`
41. `xtask_parse_options_hostile`
42-47. Additional targets from fuzz/Cargo.toml not yet implemented

### A.2 Corpus Inventory

| Corpus | Seed Count | Source |
|--------|-----------|--------|
| `compiled_ir/` | ~10 | Unknown |
| `decode_record/` | ~30 | Valid journal events |
| `expr_eval/` | ~5 | Expression programs |
| `ipc_frame/` | ~5 | Valid IPC frames |
| `journal_event/` | ~10 | Valid journal events |
| `vb_f04l_yaml_compiler_compile/` | ~5 | Valid YAML workflows |
| `yaml_events/` | ~5 | Valid YAML documents |

---

## Appendix B: Quickstart Commands

```bash
# Install engines
cargo install cargo-fuzz
cargo install cargo-afl
cargo install cargo-honggfuzz

# Build all fuzz targets
cargo fuzz build --target x86_64-unknown-linux-gnu

# Smoke test a single target
cargo fuzz run journal_event --target x86_64-unknown-linux-gnu -- -max_total_time=60

# Mini-fuzz all targets (30s each)
for t in $(cargo fuzz list); do
  cargo fuzz run "$t" --target x86_64-unknown-linux-gnu -- -max_total_time=30 -max_len=4096
done

# Corpus minimization
bash scripts/fuzz-minimization.sh journal_event

# AFL run
cargo afl build --manifest-path fuzz/Cargo.toml --bin compiled_ir
cargo afl fuzz -i fuzz/corpus/compiled_ir -o fuzz/afl_out/compiled_ir \
  -x fuzz/dicts/vb_storage.dict target/fuzz/compiled_ir
```

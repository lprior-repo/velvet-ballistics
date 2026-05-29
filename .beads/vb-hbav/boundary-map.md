# Boundary Map — Fuzz Hardening (vb-hbav)

## Architecture Boundary Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          fuzz/ (Fuzz Crate)                             │
│                                                                         │
│  ┌─────────────────┐  ┌──────────────────┐  ┌─────────────────────────┐│
│  │  fuzz_targets/  │  │    src/bin/      │  │      src/lib.rs         ││
│  │  (libfuzzer)    │  │    (stdin)       │  │   (shared harnesses)    ││
│  │                 │  │                  │  │                         ││
│  │  fuzz_target!() │  │  run_with_stdin  │  │  fuzz_ipc_frame()       ││
│  │  + ASAN+coverage│  │  + no coverage   │  │  fuzz_journal_event()   ││
│  └────────┬────────┘  └────────┬─────────┘  │  fuzz_compiled_ir()     ││
│           │                    │             │  ... 42 total fns      ││
│           │                    │             └───────────┬─────────────┘│
│           │                    │                         │              │
└───────────┼────────────────────┼─────────────────────────┼──────────────┘
            │                    │                         │
            │   ALL CALL THROUGH │  fuzz_lib::fuzz_*       │
            │                    │                         │
            ▼                    ▼                         ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                       PRODUCTION CRATES                                 │
│                                                                         │
│  ┌───────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────┐   │
│  │ vb_storage │ │ vb_ipc  │ │ vb_core  │ │ vb_yaml  │ │ vb_compile │   │
│  │           │ │         │ │          │ │          │ │            │   │
│  │ decode_   │ │ decode_ │ │ try_from │ │ parse_   │ │ compile_   │   │
│  │ record()  │ │ frame() │ │ _parts() │ │ yaml()   │ │ workflow() │   │
│  │ encode_   │ │ decode_ │ │ eval_    │ │ build_   │ │            │   │
│  │ record()  │ │ payload │ │ expr()   │ │ source_  │ │            │   │
│  │ submit_   │ │         │ │ budget   │ │ map()    │ │            │   │
│  │ artifact()│ │         │ │ compute()│ │          │ │            │   │
│  │ recover_* │ │         │ │ engine::*│ │          │ │            │   │
│  │           │ │         │ │          │ │          │ │            │   │
│  │ ← 16 fuzz │ │← 4 fuzz │ │← 15 fuzz│ │← 3 fuzz │ │← 2 fuzz   │   │
│  │  targets  │ │ targets │ │ targets │ │ targets │ │ targets   │   │
│  └───────────┘ └──────────┘ └──────────┘ └──────────┘ └────────────┘   │
│                                                                         │
│  ┌───────────┐ ┌──────────┐ ┌──────────┐ ┌─────────────────┐           │
│  │vb_expr    │ │vb_runtime│ │vb_validate│ │vb_boundary_inv  │           │
│  │           │ │          │ │           │ │                 │           │
│  │ lex_expr()│ │collect_  │ │validate_  │ │ parse_          │           │
│  │ parse_    │ │ page()   │ │ gate_07() │ │ inventory()     │           │
│  │ expr()    │ │admission │ │ gate_08() │ │ validate_       │           │
│  │ compile_  │ │ module   │ │ ...       │ │ evidence_ref()  │           │
│  │ expr()    │ │          │ │ gate_13() │ │                 │           │
│  │ eval_*    │ │          │ │           │ │                 │           │
│  │           │ │          │ │           │ │                 │           │
│  │← 5 fuzz   │ │← 2 fuzz │ │← 1 fuzz  │ │← 1 fuzz target│           │
│  │  targets  │ │ targets │ │ target   │ │                 │           │
│  └───────────┘ └──────────┘ └──────────┘ └─────────────────┘           │
└─────────────────────────────────────────────────────────────────────────┘
```

## Boundary Layers

### B1: Pure Fuzz Core (fuzz/src/lib.rs)

**Location**: `fuzz/src/lib.rs` (3245 lines)

**What it contains**: 42+ `pub fn fuzz_*` functions that each:
1. Accept `data: &[u8]` (raw fuzz input)
2. Parse/decode bytes into domain types
3. Call production crate APIs
4. Assert invariants on results

**Rules**:
- No I/O except through production crate APIs (tempfile for journal admission tests)
- No network, no filesystem expectations (tempfile is acceptable)
- No `unwrap`, `expect`, `panic` (clippy enforced)
- All allocations bounded (`MAX_FUZZ_PAYLOAD`, `FUZZ_MAX_EXPR_OPS`, etc.)
- Every function must return `()` — assertions are the output

**Boundary to production crates**: Direct function calls to public APIs of `vb_storage`, `vb_ipc`, `vb_core`, `vb_expr`, `vb_yaml`, `vb_validate`, `vb_boundary_inventory`, `vb_runtime`, `vb_compile`.

### B2: libfuzzer Shell (fuzz/fuzz_targets/*.rs)

**Location**: `fuzz/fuzz_targets/*.rs` (12 active files)

**Pattern**:
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_TARGET(data);
});
```

**Rules**:
- One file per target, exactly one `fuzz_target!()` invocation
- MUST NOT contain assertion logic (delegates to lib.rs)
- MUST be declared in Cargo.toml `[[bin]]`
- `name` must be unique across all `[[bin]]` entries

**Boundary to fuzz core**: Calls exactly one `fuzz_lib::fuzz_*` function.

**Boundary to libfuzzer runtime**: The `fuzz_target!()` macro links against `libfuzzer-sys`, providing the `LLVMFuzzerTestOneInput` entry point.

### B3: Stdin Shell (fuzz/src/bin/*.rs)

**Location**: `fuzz/src/bin/*.rs` (47 files, including 38 with duplicated boilerplate)

**Pattern** (current — duplicated):
```rust
#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    run_with_stdin(fuzz_lib::fuzz_TARGET)
}
// ... ~22 lines of duplicated run_with_stdin/write_stderr ...
```

**Pattern** (target — after refactor):
```rust
#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    bin_common::run_with_stdin(fuzz_lib::fuzz_TARGET)
}
```

**Rules**:
- Gated behind `#[cfg(feature = "fuzz")]` — not compiled in normal builds
- Reads ALL stdin into `Vec<u8>`, passes to fuzz function
- No coverage feedback, no ASAN integration
- Purpose: smoke-testing via pipes (`echo $input | cargo run --bin TARGET --features fuzz`)

**Boundary to fuzz core**: Calls exactly one `fuzz_lib::fuzz_*` function.

### B4: Corpus Layer (fuzz/corpus/)

**Location**: `fuzz/corpus/<target_name>/` directories (7 active)

**What it contains**: Binary seed files that the fuzzer uses as starting points for mutation.

**Rules**:
- All files are binary, not version-controlled as text
- Seeds MUST NOT cause harness panics
- Seeds SHOULD include edge cases (empty, single byte, magic bytes, valid, corrupt)
- Structure-aware targets need ≥ 5 seeds; all targets need ≥ 1

**Boundary to fuzz core**: Seeds are fed by libfuzzer runtime into `fuzz_target!()` closures, which call `fuzz_lib::fuzz_*`.

### B5: Campaign Artifacts (fuzz/artifacts/)

**Location**: `fuzz/artifacts/<target_name>/` (auto-created by libfuzzer on crash)

**What it contains**: Inputs that caused crashes, OOMs, timeouts, or leaks.

**Rules**:
- Must be triaged within 24 hours of campaign completion
- Crash artifacts are evidence — do not delete until bead is closed
- Minimized crashes become regression seeds in corpus

## Production Crate Boundaries

| Crate | Fuzz Targets | Boundary Functions Exercised |
|-------|-------------|------------------------------|
| **vb_storage** | 16 | `decode_record`, `encode_record`, `submit_artifact`, `decode_record_header`, `FjallJournal::open`, `workflow_source`, `compiled_ir`, `run_header`, `events_for_run`, `put_compiled_ir`, `recovery::replay_events`, `recovery::extract_terminal`, `recovery::ActionReplayTracker` |
| **vb_core** | 15 | `WorkflowParts`, `CompiledWorkflow::try_from_parts`, `validate_compiled_workflow`, `RunFrame::new`, `engine::eval_expr_with_store`, `engine::eval_accessor_with_store`, `engine::run_until_blocked`, `StepBudget::new`, `budget::WholeWorkflowBudget::compute`, `ExprOp`, `ExprProgram::try_from_ops`, `ConstValue`, `SlotValue`, `ValueStore`, `AccessorProgram`, `Taint`, `join_taint` |
| **vb_ipc** | 4 | `decode_frame_header`, `decode_frame_payload`, `IpcFrameHeader::decode`, `validate_frame_magic`, `IPC_HEADER_LEN`, `MaxPayloadBytes`, `IpcError`, `IpcPayload` |
| **vb_expr** | 5 | `lexer::lex_expr`, `parser::parse_expr`, `bytecode::compile_expr_with_pool`, `eval::eval_expr_program` |
| **vb_yaml** | 3 | `validate_yaml_profile`, `parse_yaml_events`, `build_source_map`, `build_semantic_source_map`, `SourceSpan::new` |
| **vb_runtime** | 2 | `primitives::collect::collect_page`, `primitives::collect::CollectStates`, `admission::StorageArtifactStore`, `admission::AcceptedArtifactStore::load_accepted_artifact` |
| **vb_validate** | 1 | `shared::validate_with_contracts`, `gates::validate_gate_07_*`, `gates::validate_gate_08_*`, `gates::validate_gate_09_*`, `gates::validate_gate_11_*`, `gates::validate_gate_13_*`, `diagnostic::diagnostic_from_error`, `ValidationError` |
| **vb_compile** | 2 | `compile_workflow` |
| **vb_boundary_inventory** | 1 | `boundary_inventory::parse_inventory`, `boundary_inventory::validate_evidence_reference_bytes` |

## Crates With Zero Fuzz Coverage (Out of Scope for This Bead)

These crates have zero fuzz targets and are NOT covered by vb-hbav:

| Crate | Status | Requires Separate Bead |
|-------|--------|------------------------|
| `vb_cli` | 2 existing (arg parsing, options) | N/A — already covered |
| `vb_codegen` | `generated_compare` (claimed fixed in C.21) | C.21 verification |
| `vb_proof_kernels` | Zero fuzz | **Separate P0 bead needed** |
| `vb_benchmark` | Zero fuzz | Separate P2 bead |
| `vb_doc` | 1 existing (`check_doc_taint`) | Already covered |

## Fuzz Crate Dependency Graph

```
velvet-ballastics-fuzz
├── vb_storage (via path dep)
├── vb_ipc (via path dep)
├── vb_core (via path dep)
├── vb_expr (via path dep)
├── vb_yaml (via path dep)
├── vb_runtime (via path dep)
├── vb_validate (via path dep)
├── vb_compile (via path dep)
├── vb_boundary_inventory (via path dep)
├── vb_doc (via path dep)
├── postcard (via crates.io, deserialization of fuzz input)
├── blake3 (via crates.io, digest computation)
├── libfuzzer-sys (via crates.io, fuzz runtime)
├── tempfile (via crates.io, journal admission tests)
└── crc32c (via crates.io, checksum verification)
```

## Isolation Guarantees

1. **No filesystem leakage**: `tempfile::tempdir()` creates isolated directories. All journal operations use these temp dirs. No production data paths are touched.
2. **No network**: The fuzz crate has zero network dependencies at the dependency level.
3. **No I/O in harness body**: Only `RunFrame::new`, `ValueStore` operations, and `submit_artifact` with temp journals — all in-memory or temp-disk operations.
4. **No unsafe**: `fuzz/Cargo.toml` enforces `unsafe_code = "forbid"` at the crate level. The `fuzz_targets.rs` file uses `#[unsafe(no_mangle)]` for orphan libfuzzer C ABI stubs — these are stubs (return 0) and must be replaced or removed during hardening.
5. **No randomness**: Fuzz harnesses are deterministic given input bytes. No `rand` dependency.

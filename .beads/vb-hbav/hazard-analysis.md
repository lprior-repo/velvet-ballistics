# Hazard Analysis — Fuzz Hardening (vb-hbav)

## H1: Hostile Input Hazards

### H1.1: Unbounded Allocation from Attacker-Controlled Length
- **Risk**: A length field in postcard-decoded bytes claims 2^32 elements. The harness allocates a Vec of that size → OOM or panic.
- **Affected**: All targets using `postcard::from_bytes` on types with variable-length collections (vectors, boxed slices, strings).
- **Mitigation**: Every `postcard::from_bytes` call on types with alloc fields MUST be bounded. Existing bounds: `MAX_FUZZ_PAYLOAD (4096)`, `FUZZ_MAX_EXPR_OPS (64)`, `FUZZ_SLOT_COUNT (16)`, `FUZZ_MAX_NODES (32)`, `FUZZ_MAX_ACCESSOR_DEPTH (16)`.
- **Gap**: Some targets use `postcard::from_bytes::<Vec<JournalEvent>>` without an explicit bound on the vector length. The postcard wire format does not protect against this.
- **Severity**: CRITICAL (OOM in fuzz harness, false-positive crash).

### H1.2: UTF-8 Boundary Violation
- **Risk**: `std::str::from_utf8(data)` failure path causes early-return in text-based harnesses, silently dropping coverage. Arbitrary bytes that happen to decode as valid UTF-8 may still contain invalid YAML/expression syntax.
- **Affected**: `fuzz_yaml_events`, `fuzz_expression`, `fuzz_capability_name_schema`, `fuzz_capability_contract_schema`, `fuzz_strict_yaml_profile`, `fuzz_compile_source_ast_marks`, `fuzz_diagnostic_code_from_str`, `fuzz_diagnostic_from_error`, `fuzz_span_bridge`.
- **Mitigation**: Already handled — all text functions early-return on `from_utf8` failure. No hazard in current code.
- **Severity**: MINOR (coverage gap for non-UTF-8 inputs, but harness doesn't panic).

### H1.3: Slice Bounds Panic
- **Risk**: `data[x]` or `data[x..y]` without bounds check causes index-out-of-bounds panic.
- **Affected**: All harnesses that perform manual slicing. `data[0]`, `data.get(..32)`, `data[..IPC_HEADER_LEN]`.
- **Mitigation**: Most harnesses use `data.get()` with safe fallback. Some use `data.len() < N { return; }` guards. The `check_node_slots` function uses unchecked indexing on assumed-valid internal structures.
- **Gap**: `fuzz_targets.rs` has `data: &[u8]` passed to functions; each function must validate length before indexing.
- **Severity**: CRITICAL (panic in fuzz harness → false crash, blocks fuzzing).

### H1.4: Arithmetic Overflow on Fuzz-Derived Values
- **Risk**: `u16::from(byte)` and unchecked `saturating_add` patterns may produce unexpected values that cause downstream panics.
- **Affected**: All harnesses that derive numeric parameters from bytes (`byte0.wrapping_rem(16)`, `u16::from(byte0)`).
- **Mitigation**: Existing code uses `wrapping_rem`, `saturating_add`, `checked_rem`. Clippy `arithmetic_side_effects` is allowed in fuzz code (`#![allow(clippy::arithmetic_side_effects)]`).
- **Gap**: The `allow` makes fuzz code a regression risk if an unchecked arithmetic is introduced in new hardening code.
- **Severity**: MAJOR.

### H1.5: Type Confusion in postcard Decode
- **Risk**: `postcard::from_bytes::<WorkflowParts>(data)` succeeds for bytes that are not valid `WorkflowParts` but happen to deserialize. The resulting struct may have invalid field values (e.g., slot_count = 0, empty nodes) that cause downstream panics.
- **Affected**: `fuzz_compiled_ir`, `fuzz_generated_compare`, `fuzz_expr_eval`, `fuzz_admission_fuzz`, `fuzz_strict_artifact_decoder`.
- **Mitigation**: `CompiledWorkflow::try_from_parts` catches many invalid states. Slot bound checks in `check_node_slots` catch out-of-bounds references.
- **Severity**: MAJOR.

## H2: Temporal Workflow Hazards

### H2.1: Unsynchronized FjallJournal Access
- **Risk**: Multiple fuzz target instances running simultaneously might access the same temp directory, causing file-lock contention or corrupted journal files.
- **Affected**: All targets using `tempfile::tempdir()` + `FjallJournal::open` (`fuzz_admission_flow`, `fuzz_digest_coherence`, `fuzz_readback_family_set`, `fuzz_accepted_artifact_decode`, `fuzz_admission_input_surface`).
- **Mitigation**: Each fuzz target invocation creates a fresh tempdir. `cargo fuzz run` runs a single target at a time by default.
- **Severity**: MINOR (only relevant for parallel fuzzing, not in scope for Phase 1).

### H2.2: Build-Cache Poisoning
- **Risk**: Stale build artifacts from a failed `cargo fuzz build` cause subsequent builds or runs to use corrupted binaries.
- **Affected**: All targets.
- **Mitigation**: `cargo fuzz build` per-target, fresh `target/` directory per profile.
- **Severity**: MINOR.

### H2.3: ASAN Build Artifact Conflict
- **Risk**: Mixing sanitized and unsanitized binaries in the same `target/` directory causes linker errors or silent ASAN disablement.
- **Affected**: All targets when switching between `cargo fuzz run TARGET` and `RUSTFLAGS="-Zsanitizer=address" cargo fuzz run TARGET`.
- **Mitigation**: Use separate build profiles. The EXECUTE.md workflow specifies `RUSTFLAGS` per-command.
- **Severity**: MAJOR.

## H3: Rust Core Invariant Hazards

### H3.1: `#![allow(clippy::indexing_slicing)]` in lib.rs
- **Risk**: The fuzz library allows unchecked indexing/slicing. A future harness addition that accidentally indexes out of bounds will not be caught by clippy.
- **Affected**: All code in `fuzz/src/lib.rs`.
- **Mitigation**: All existing code checks bounds before indexing. New code must follow the same pattern.
- **Severity**: MAJOR (process risk, not current code risk).

### H3.2: `#![allow(clippy::as_conversions)]` in lib.rs
- **Risk**: `u16::from(byte)` and similar conversions may be lossy or cause unexpected values.
- **Affected**: All numeric conversions from `u8` to `u16`, `usize`, `u32`.
- **Mitigation**: `u16::from(u8)` is infallible. Downstream code clips values with `wrapping_rem` and `saturating_add`.
- **Severity**: MINOR.

### H3.3: Stub C ABI Functions Return Success
- **Risk**: `fuzz_targets.rs` defines 5 `#[unsafe(no_mangle)] pub extern "C" fn LLVMFuzzerTestOneInput*` stubs that unconditionally return 0 (success). If any code path accidentally calls these stubs instead of the real libfuzzer entry points, it will silently report success regardless of input.
- **Affected**: All libfuzzer targets if the stub symbols shadow real ones.
- **Mitigation**: These stubs must be removed or properly guarded. They were created as scaffolding and should not exist in production fuzz code.
- **Severity**: CRITICAL (false negative — fuzzer reports success but does no work).

### H3.4: Missing `unsafe_code = "forbid"` in individual target files
- **Risk**: The crate-level `unsafe_code = "forbid"` in Cargo.toml covers all files, but individual `fuzz_target!()` macros may internally use unsafe. This is expected and acceptable.
- **Affected**: None (libfuzzer-sys internal `unsafe` is expected).
- **Severity**: NONE.

## H4: Bounded State Hazards

### H4.1: Exhaustion of Temp Directories
- **Risk**: High-rate fuzzing creates and destroys many `tempfile::tempdir()` instances. On some systems, this can exhaust inode limits or fill `/tmp`.
- **Affected**: Admission-related targets (5 targets use tempdir).
- **Mitigation**: Each fuzz iteration creates and drops a fresh tempdir. The OS cleans up on drop.
- **Severity**: MINOR (only at extreme execution rates >10k/sec).

### H4.2: Corpus Size Explosion
- **Risk**: Without corpus minimization, the fuzzer's corpus directory grows unboundedly, consuming disk space.
- **Affected**: All targets, especially structure-aware targets that generate large inputs.
- **Mitigation**: `cargo fuzz cmin TARGET` periodically minimizes corpus. CI should enforce max corpus size.
- **Severity**: MINOR.

### H4.3: Seed Corpus Regression
- **Risk**: A seed file that was benign today causes a panic tomorrow after production code changes. The seed itself becomes a regression test.
- **Affected**: All targets with seed corpora.
- **Mitigation**: Seeds are committed to the repository and versioned. A seed that causes a panic is a feature, not a bug — it means the regression is caught.
- **Severity**: NONE (by design).

## H5: Concurrency Hazards

### H5.1: Not in scope for this bead
- **Assessment**: No concurrency in fuzz harness code (single-threaded, deterministic execution given input). The `cargo fuzz run` command runs a single worker by default.
- **Future risk**: Parallel fuzzing (`-jobs=N`) may introduce races in shared state. This is deferred to Phase 2+.
- **Severity**: NONE for Phase 1.

## H6: Unsafe / Provenance Hazards

### H6.1: libfuzzer-sys FFI Boundary
- **Risk**: `libfuzzer-sys` provides the C ABI entry point `LLVMFuzzerTestOneInput`. This is FFI and internally unsafe. We trust the cargo-fuzz crate.
- **Affected**: All `fuzz_targets/*.rs` targets.
- **Mitigation**: `cargo-fuzz` is the standard Rust fuzzing infrastructure. The `fuzz_target!()` macro handles unsafe correctly.
- **Severity**: NONE (standard tooling).

### H6.2: Orphan `#[unsafe(no_mangle)]` Stubs
- **Risk**: The 5 stubs in `fuzz_targets.rs` use `#[unsafe(no_mangle)]` to expose `LLVMFuzzerTestOneInput*` symbols. These are stubs that return 0. If a real libfuzzer target accidentally links against a stub, it silently does nothing.
- **Affected**: Any libfuzzer target with a name collision.
- **Mitigation**: Remove the stubs. They are dead code and a hazard.
- **Severity**: CRITICAL.

## H7: Performance Hazards

### H7.1: Expensive Admission Targets
- **Risk**: `fuzz_admission_flow`, `fuzz_admission_fuzz`, `fuzz_digest_coherence`, `fuzz_readback_family_set`, `fuzz_accepted_artifact_decode`, `fuzz_admission_input_surface` all create `tempdir()` + `FjallJournal::open()`. This is expensive per-iteration and slows fuzzing.
- **Affected**: 6 admission-related targets.
- **Mitigation**: For Phase 1, this is acceptable. Phase 2+ should refactor to use in-memory journal stubs.
- **Severity**: MINOR (slower fuzzing, not incorrect).

### H7.2: postcard Serialization in Hot Loop
- **Risk**: `fuzz_digest_coherence` calls `postcard::to_allocvec(&parts)` and `blake3::hash()` on every iteration. This is expensive but necessary for the coherence invariant.
- **Affected**: `fuzz_digest_coherence`, `fuzz_slot_value_roundtrip`.
- **Mitigation**: Acceptable for correctness. Performance is secondary to finding bugs.
- **Severity**: NONE.

## H8: Release / API Hazards

### H8.1: API Drift Between fuzz Crate and Production Crates
- **Risk**: Production crate APIs change (renamed functions, changed signatures, removed error variants) and the fuzz crate is not updated. Build failures result.
- **Affected**: All targets.
- **Mitigation**: `cargo fuzz build` is the CI gate. Drift causes build failure, not silent incorrectness.
- **Severity**: MAJOR (broken CI, not missed bugs).

### H8.2: Error Variant Exhaustiveness Drift
- **Risk**: A new `JournalError` variant is added to `vb_storage`. The fuzz harness's `match e { ... }` block does not include the new variant. The wildcard `_ => {}` arm silently accepts it. The fuzzer stops asserting that the new variant is a valid error.
- **Affected**: All harnesses with exhaustive error matching (`fuzz_journal_event`, `fuzz_storage_envelope_boundary`, `fuzz_binary_payload_boundary`, `fuzz_vb_qi37_12_persisted_payload_decode`).
- **Mitigation**: CI should compare fuzz harness error match arms against production error enum variants. A mismatch should produce a warning (not an error, because wildcard is valid for forward-compat).
- **Severity**: MAJOR.

### H8.3: vb_boundary_inventory Quality Module Removal
- **Risk**: `vb_boundary_inventory::quality::test_loop_inventory` module was removed, making 4 `vb_5xs4_*` fuzz targets uncompilable. These targets are already commented out in Cargo.toml.
- **Affected**: `vb_5xs4_generated_source_mapping`, `vb_5xs4_inventory_report`, `vb_5xs4_label_sufficiency`, `vb_5xs4_scan_source_text`.
- **Mitigation**: These targets need separate beads for API compatibility analysis.
- **Severity**: MAJOR (4 dead targets).

## Hazard Severity Summary

| Hazard | Severity | Affected Targets | Mitigation Status |
|--------|----------|-----------------|-------------------|
| H1.1: Unbounded allocation | CRITICAL | postcard targets | Partially mitigated (bounds exist but not universal) |
| H1.3: Slice bounds panic | CRITICAL | All targets | Mostly mitigated, needs audit in new hardening code |
| H3.3: C ABI stubs | CRITICAL | All libfuzzer targets | NOT mitigated — stubs must be removed |
| H6.2: Orphan unsafe stubs | CRITICAL | All libfuzzer targets | NOT mitigated — same as H3.3 |
| H1.4: Arithmetic overflow | MAJOR | All targets | Mitigated by saturating/wrapping ops |
| H1.5: Type confusion | MAJOR | postcard targets | Partially mitigated by try_from_parts |
| H2.3: ASAN build conflict | MAJOR | All targets | Mitigated by EXECUTE.md workflow |
| H3.1: Clippy allows | MAJOR | lib.rs | Mitigated by code review |
| H7.1: Expensive admission | MINOR | 6 targets | Acceptable for Phase 1 |
| H8.1: API drift | MAJOR | All targets | Caught by CI build |
| H8.2: Error exhaustiveness drift | MAJOR | TypedError harnesses | Needs CI check |

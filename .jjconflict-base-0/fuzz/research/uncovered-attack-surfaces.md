# Fuzz Attack Surface Analysis — Uncovered Crates

**Date:** 2026-05-24  
**Repository:** velvet-ballistics  
**Scope:** 9 crates with ZERO or minimal fuzz coverage

---

## Existing Fuzz Coverage (for reference)

The `fuzz/fuzz_targets/` directory contains 12 targets:
- `check_doc_taint_consistency_accepts_arbitrary_markdown.rs` — vb_doc text input
- `decode_record.rs` — record decoding
- `expr_eval.rs` — expression evaluation
- `journal_event.rs` — journal events
- `lex_expr.rs` — expression lexing
- `ui_redaction_artifact.rs` — UI redaction
- `vb_5xs4_generated_source_mapping.rs`
- `vb_5xs4_inventory_report.rs`
- `vb_5xs4_label_sufficiency.rs`
- `vb_5xs4_scan_source_text.rs`
- `vb_f04l_yaml_compiler_compile.rs`
- `vb_storage_codec.rs`

## Analysis Methodology

Each crate's source tree was read in full. Every `pub fn` was classified by input category:

| Category | Priority | Description |
|----------|----------|-------------|
| `&[u8]` | **CRITICAL** | Raw bytes — highest priority for fuzzing |
| `&str`/`String` | **HIGH** | Text input |
| Parsed/deserialized type | **HIGH** | Types decoded from bytes/text |
| Numeric (overflow/underflow) | **MEDIUM** | Any numeric type subject to arithmetic |
| Collections | **MEDIUM** | Collection types with bounds |

---

## 1. vb_boundary_inventory — `crates/vb_boundary_inventory/src/`

### Fuzz Coverage: NONE (zero fuzz targets exist for this crate)

### Attack Surface Summary

This crate is a **JSON-parsing boundary inventory system**. It reads JSON from `boundary-surfaces.txt` config files and parses inventory documents from `&[u8]`. Every function that takes `&[u8]` or reads from disk is a direct fuzz target.

### pub fn Attack Surface Catalog

#### CRITICAL — Raw Bytes (`&[u8]`)

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `parse_inventory` | `boundary_inventory/parser.rs:7` | `pub fn parse_inventory(bytes: &[u8]) -> Result<BoundaryInventory, BoundaryInventoryError>` | **PRIMARY FUZZ TARGET.** Accepts raw `&[u8]`, calls `serde_json::from_slice()`. Malformed JSON, deeply nested JSON, oversized arrays, invalid UTF-8 edge codepoints. |
| `validate_evidence_reference_bytes` | `boundary_inventory/validation.rs:9` | `pub fn validate_evidence_reference_bytes(bytes: &[u8]) -> Result<EvidenceReference, BoundaryInventoryError>` | **PRIMARY FUZZ TARGET.** Accepts raw `&[u8]`, calls `str::from_utf8()`, then string pattern matching. Non-UTF-8 bytes, overlong sequences, path traversal strings. |

#### HIGH — String/Text Input

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `BoundaryCandidate::new` | `boundary_inventory/types.rs:58` | `pub fn new(source_path: impl Into<PathBuf>, marker: impl Into<String>) -> Self` | Marker strings — path traversal via `source_path`, malicious marker names. |
| `ReviewStatus::from_serialized` | `boundary_inventory/types.rs:180` | `pub fn from_serialized(value: impl Into<String>) -> Self` | String input — unbounded strings can create `Other(String)` variant. |
| `EvidenceReference::free_text` | `boundary_inventory/types.rs:147` | `pub fn free_text(text: impl Into<String>) -> Self` | Unbounded text — no size limit. |
| `ClassifiedBoundary::new` | `boundary_inventory/types.rs:76` | `pub fn new(input: ClassifiedBoundaryInput) -> Self` | `String` fields in input struct — `id` construction, path traversal in `source_path`. |

#### HIGH — Deserialized/Parsed Types

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `classify_boundary` | `boundary_inventory/api.rs:30` | `pub fn classify_boundary(candidate: BoundaryCandidate) -> Result<ClassifiedBoundary, BoundaryInventoryError>` | Takes `BoundaryCandidate` with String `marker` — unknown marker strings produce errors but may cause unexpected behavior. |
| `validate_inventory` | `boundary_inventory/api.rs:54` | `pub fn validate_inventory(inventory: BoundaryInventory, workspace: WorkspaceRoot) -> Result<ValidatedBoundaryInventory, BoundaryInventoryError>` | Consumes full inventory — duplicate IDs, stale freshness, invalid review status combinations. |
| `inventory_completion_status` | `boundary_inventory/api.rs:73` | `pub fn inventory_completion_status(inventory: ValidatedBoundaryInventory) -> Result<UnsafeIsolationStatus, BoundaryInventoryError>` | Empty records, mismatched counts. |

#### MEDIUM — Numeric Types

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `FreshnessMarker::new` | `boundary_inventory/types.rs:161` | `pub fn new(source_version: u64, schema_version: u64, evidence_version: u64) -> Self` | `u64` values — stale detection at `boundary_inventory/validation.rs:97` compares `u64` values; overflow not possible but logical inversions at extremes. |
| `ValidatedBoundaryInventory::with_schema_version` | `boundary_inventory/inventory.rs:62` | `pub fn with_schema_version(schema_version: u32) -> Self` | `u32` — schema version only validated at `validate_inventory()` against `Some(1)`. |

### Recommended Fuzz Targets

1. **`fuzz_boundary_inventory_parse`** — Feed arbitrary `&[u8]` to `parse_inventory()`
2. **`fuzz_boundary_inventory_evidence_ref`** — Feed arbitrary `&[u8]` to `validate_evidence_reference_bytes()`
3. **`fuzz_boundary_inventory_validate`** — Construct arbitrary `BoundaryInventory` structs and call `validate_inventory()`

---

## 2. vb_doc — `crates/vb_doc/src/`

### Fuzz Coverage: PARTIAL (1 target: `check_doc_taint_consistency_accepts_arbitrary_markdown.rs`)

### Attack Surface Summary

This crate is a **text analysis engine** that scans markdown for taint vocabulary violations and evidence claims. It operates on `&str` text input. The existing fuzz target covers `check_doc_taint_consistency()`.

### pub fn Attack Surface Catalog

#### HIGH — String/Text Input

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `MasterDocSnapshot::for_workspace_text` | `lib.rs:20` | `pub fn for_workspace_text(path: PathBuf, text: &str) -> Self` | Arbitrary text strings. |
| `check_doc_taint_consistency` | `reconcile.rs:54` | `pub fn check_doc_taint_consistency(text: &str) -> Result<ContradictionReport, DocReconcileError>` | **HAS FUZZ TARGET.** Arbitrary markdown text. Edge cases: empty strings, only whitespace, maximum-length strings, unicode confusables. |
| `plan_taint_doc_reconciliation` | `reconcile.rs:16` | `pub fn plan_taint_doc_reconciliation(doc: MasterDocSnapshot, policy: EvidencePolicy) -> Result<DocPatchPlan, DocReconcileError>` | **NO FUZZ TARGET.** Consumes text via `MasterDocSnapshot`. |
| `scan_for_stale_clean_only_text` | `reconcile.rs:38` | `pub fn scan_for_stale_clean_only_text(doc: MasterDocSnapshot) -> Result<ContradictionReport, DocReconcileError>` | **NO FUZZ TARGET.** Same text scanning, returns contradiction report. |
| `validate_evidence_bounded_wording` | `reconcile.rs:61` | `pub fn validate_evidence_bounded_wording(doc: MasterDocSnapshot, evidence: EvidenceIndex) -> Result<EvidenceBoundedReport, DocReconcileError>` | **NO FUZZ TARGET.** Evidence claim validation against known claims. |
| `validate_taint_vocabulary_consistency` | `reconcile.rs:68` | `pub fn validate_taint_vocabulary_consistency(doc: MasterDocSnapshot) -> Result<TaintVocabularyReport, DocReconcileError>` | **NO FUZZ TARGET.** |
| `EvidenceSupport::cited` | `evidence.rs:47` | `pub fn cited(sentence: &str, artifact: &str) -> Self` | Unbounded strings. |
| `EvidenceSupport::pending` | `evidence.rs:56` | `pub fn pending(sentence: &str) -> Self` | Unbounded sentence strings. |
| `EvidenceIndex::from_supports` | `evidence.rs:15` | `pub fn from_supports(supports: Vec<EvidenceSupport>) -> Self` | Unbounded vector size. |

#### MEDIUM — Collections

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `EvidencePolicy::strict_bounded` | `lib.rs:35` | `pub fn strict_bounded(workspace_root: PathBuf) -> Self` | PathBuf from user input. |

### Recommended Fuzz Targets

1. **`fuzz_doc_reconciliation_plan`** — Feed arbitrary text to `plan_taint_doc_reconciliation()`
2. **`fuzz_doc_evidence_bounded`** — Feed arbitrary text + evidence index to `validate_evidence_bounded_wording()`
3. **`fuzz_doc_scan_stale_text`** — Feed arbitrary text to `scan_for_stale_clean_only_text()`

---

## 3. vb_codegen — `crates/vb_codegen/src/`

### Fuzz Coverage: NONE (zero fuzz targets)

### Attack Surface Summary

This is a **Rust code generator** that compiles `CompiledWorkflow` IR into generated Rust source. The primary attack surface is the `CompiledWorkflow` IR structure itself — deeply nested node graphs, pathological expression stacks, and boundary-value numeric IDs.

### pub fn Attack Surface Catalog

#### CRITICAL — Raw Bytes/Serialized IR (indirect)

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `emit_rust_workflow` | `lib.rs:87` | `pub fn emit_rust_workflow(workflow: &CompiledWorkflow) -> CodegenResult<String>` | **PRIMARY TARGET.** Takes `CompiledWorkflow` — fuzz via constructing pathological IR: max node count, max expression ops, deeply nested accessor paths, overflow-prone step/slot indices. |
| `compare_generated_to_ir` | `lib.rs:2141` | `pub fn compare_generated_to_ir(source: &str, workflow: &CompiledWorkflow) -> CodegenResult<()>` | **Requested by controller.** Scans generated source for forbidden patterns — arbitrary source strings from the codegen pipeline. |
| `format_generated_rust` | `lib.rs:2074` | `pub fn format_generated_rust(source: &str) -> CodegenResult<String>` | **SPAWNS rustfmt PROCESS** with arbitrary source string piped via stdin. Command injection risk and DoS via giant source. |
| `compile_check_generated_rust` | `lib.rs:2113` | `pub fn compile_check_generated_rust(source: &str, temp_dir: &std::path::Path) -> CodegenResult<()>` | **SPAWNS rustc PROCESS** — writes arbitrary source to temp_dir, invokes rustc. Path traversal via temp_dir. |
| `emit_trybuild_fixture` | `lib.rs:2058` | `pub fn emit_trybuild_fixture(workflow: &CompiledWorkflow, fixture_path: &std::path::Path) -> CodegenResult<()>` | Writes generated source to disk at arbitrary path. Path traversal. |
| `validate_generated_subset` | `lib.rs:123` | `pub fn validate_generated_subset(workflow: &CompiledWorkflow) -> CodegenResult<()>` | **BOUNDARY CHECK.** Validates the IR before emission — fuzz with pathological IR to find validation gaps. |

#### MEDIUM — Numeric Types (Generated Code Arithmetic)

The generated code contains `checked_add`, `checked_sub`, `checked_mul`, `checked_div` operations on `i64` values from expression evaluation. Fuzzing the codegen with IR that produces edge-case math is critical:

- `ExprOp::Add` → `checked_add` with `i64::MAX` + `1`
- `ExprOp::Sub` → `checked_sub` with `i64::MIN` − `1`
- `ExprOp::Mul` → `checked_mul` with `i64::MAX` × `2`
- `ExprOp::Div` → `checked_div` with `0` divisor
- `ExprOp::Gt/Lt/Gte/Lte` → comparisons at `i64::MIN`/`i64::MAX`

#### HIGH — Collection Types (Generated Code)

- **ExprStack** capacity: hardcoded `MAX_EXPRESSION_STACK = 64` — fuzz with expressions requiring >64 stack frames
- **ListStore** / **ObjectStore** capacities computed from `checked_metric_mul`/`checked_metric_add` — fuzz with IR that causes capacity overflow
- **Accessor paths** bounded by `ACCESSOR_MAX_PATH_DEPTH = 16` — fuzz with deep paths

### Recommended Fuzz Targets

1. **`fuzz_codegen_emit_workflow`** — Construct pathological `CompiledWorkflow` IR and call `emit_rust_workflow()`
2. **`fuzz_codegen_validate_subset`** — Feed pathological IR to `validate_generated_subset()`
3. **`fuzz_codegen_compare_to_ir`** — Feed generated source text to `compare_generated_to_ir()`
4. **`fuzz_codegen_format_rust`** — Feed arbitrary source text to `format_generated_rust()` (process spawning)

---

## 4. vb_proof_kernels — `crates/vb_proof_kernels/src/`

### Fuzz Coverage: NONE (zero fuzz targets)

### Attack Surface Summary

This is a **pure computation kernel crate** designed for Verus/Aeneas extraction. It contains no I/O, no parsing, and no alloc. However, several functions accept raw numeric inputs that could trigger logic bugs in callers.

### pub fn Attack Surface Catalog

#### MEDIUM — Raw Numeric Types

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `EnvelopeHeader::validate_before_alloc` | `envelope_header.rs:64` | `pub fn validate_before_alloc(&self, max_payload: u64) -> ValidationResult` | Consumes `max_payload` u64 — extremely large values produce `ValidationResult::Ok` for any payload_len (since `payload_len() <= max_payload` is always true for small payloads with large max). |
| `envelope_header::validate_header_before_alloc` | `envelope_header.rs:94` | `pub fn validate_header_before_alloc(header: &EnvelopeHeader, max_payload: u64) -> ValidationResult` | Same as above — public wrapper. |
| `EnvelopeHeader::payload_len` | `envelope_header.rs:56` | `pub fn payload_len(&self) -> u64` | Combines two `u32` fields using bit shifts — tested but no fuzz for pathological combinations. |
| `Budget::sequential_add` | `resource_budget.rs:27` | `pub fn sequential_add(&mut self, other: &Budget)` | `u64` saturating arithmetic — saturating at `u64::MAX` is verified by unit tests but not fuzzed. |
| `Budget::loop_mul` | `resource_budget.rs:57` | `pub fn loop_mul(&mut self, iterations: u64)` | `u64` saturating multiplication — fuzz with `iterations=0` (zero loops), `iterations=u64::MAX`. |
| `Budget::branch_max` | `resource_budget.rs:42` | `pub fn branch_max(&mut self, other: &Budget)` | Pure max — low risk. |
| `Policy::within` | `resource_budget.rs:93` | `pub fn within(&self, budget: &Budget) -> Vec<&'static str>` | Checks budget fields against policy — fuzz with boundary values. |
| `sequential_compose` | `resource_budget.rs:114` | `pub fn sequential_compose(a: &Budget, b: &Budget) -> Budget` | Budget composition — overflow and saturating edge cases. |
| `branch_compose` | `resource_budget.rs:120` | `pub fn branch_compose(a: &Budget, b: &Budget) -> Budget` | Same — low risk. |
| `loop_compose` | `resource_budget.rs:126` | `pub fn loop_compose(body: &Budget, iterations: u64) -> Budget` | `iterations` can be 0 or `u64::MAX` — saturating mul. |
| `Taint::rank` | `taint.rs:15` | `pub fn rank(&self) -> u8` | Pure enum — no risk. |
| `join_taint` / `join_many` | `taint.rs:24-33` | Various | Pure algebra — verified correct by tests. |
| `is_valid_transition` / `validate_transition` | `step_state.rs:54,66` | Various | Pure state machine — no risk. |

#### MEDIUM — Note on `validate_header_crc` and `compute_header_crc`

These are **stub functions** (`envelope_header.rs:98,102`) that return constant values (`0` and `true`). They are not real CRC implementations, so they pose no fuzz risk currently, but any future CRC implementation will need fuzzing.

### Recommended Fuzz Targets

1. **`fuzz_proof_kernels_budget_math`** — Feed arbitrary `Budget` structs to `sequential_compose`, `branch_compose`, `loop_compose` with arbitrary iteration counts
2. **`fuzz_proof_kernels_envelope_header`** — Construct arbitrary `EnvelopeHeader` structs and test `validate_before_alloc` with arbitrary `max_payload`
3. **`fuzz_proof_kernels_policy_bounds`** — Feed arbitrary budgets to `Policy::within`

---

## 5. vb_ui_model — `crates/vb_ui_model/src/`

### Fuzz Coverage: NONE (zero fuzz targets)

### Attack Surface Summary

This crate defines the **postcard binary envelope protocol** used for IPC between CLI and server. The `encode_postcard()` and `decode_postcard()` functions are the single highest-value fuzz targets in this analysis — they handle raw `&[u8]` from the network.

### pub fn Attack Surface Catalog

#### CRITICAL — Raw Bytes (`&[u8]`)

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `decode_postcard` | `emitter/binary/mod.rs:166` | `pub fn decode_postcard<'a, T: Deserialize<'a> + core::fmt::Debug>(bytes: &'a [u8], expected_kind: EnvelopeKind, max_payload_len: u32) -> Result<T, EmitterError>` | **#1 PRIORITY FUZZ TARGET.** Accepts raw `&[u8]` from network. Parses binary header (magic, version, CRC32C, BLAKE3 digest), then deserializes postcard. Attacks: corrupted headers, CRC bypass, BLAKE3 collisions, truncated payloads, oversized payload_len, postcard deserialization of arbitrary types. |

#### HIGH — String/Text Input

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `canonicalize_cli_artifact` | `canonical.rs:74` | `pub fn canonicalize_cli_artifact(json: &serde_json::Value, kind: EnvelopeKind) -> Option<CanonicalUiArtifact>` | Takes arbitrary `serde_json::Value` — deeply nested JSON, max array size, malformed fields. |
| `EnvelopeKind::parse` | `envelope/types.rs:87` | `pub fn parse(value: &str) -> Option<Self>` | String parsing — case-sensitive, unknown values return None. Low risk. |
| `SchemaVersion::new` | `envelope/types.rs:24` | `pub fn new(value: u16) -> Result<Self, EnvelopeError>` | `u16` — only `>= 1` is valid. Low risk. |
| `DiagnosticEntry::new` | `envelope/types.rs:181` | `pub fn new(code: String, message: String, detail: Option<String>) -> Result<Self, EnvelopeError>` | String length validation against `MAX_DIAGNOSTIC_STRING_LEN = 4096` — boundary testing. |
| `PayloadEnvelope::from_json` | `envelope/types.rs:261` | `pub fn from_json(value: serde_json::Value) -> Self` | Arbitrary JSON wrapper — low risk. |

#### HIGH — Serialization/Deserialization (via postcard)

The `encode_postcard`/`decode_postcard` pair is generic over `T: Serialize`/`T: Deserialize`, meaning any type reachable via postcard can trigger bugs. The binary header parsing (`decode_cli_header`) reads 52 bytes and performs:
- `read_u32` at offset 0 (magic)
- `read_u16` at offset 4 (schema version)
- `read_u16` at offset 6 (kind)
- `read_u32` at offset 8 (header_len)
- `read_u32` at offset 12 (payload_len)
- 32-byte BLAKE3 digest at offset 16
- CRC32C at offset 48

**Specific Attack Vectors:**
- Input shorter than 52 bytes → `UnexpectedEof`
- `header_len != 52` → `HeaderLengthMismatch`
- `payload_len > max_payload_len` → `PayloadTooLarge`
- `payload_len` exceeding remaining input → `UnexpectedEof`
- `payload_len=u32::MAX` → `PayloadLengthOverflow` during `usize::try_from`
- BLAKE3 digest mismatch → `PayloadDigestMismatch`
- Schema version `< 1` → `MigrationRequired`
- Schema version `> 1` → `UnsupportedSchemaVersion`
- Postcard deserialization failures → `PostcardDecodeFailed`

#### MEDIUM — Envelope Types

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `CompareCliUiArtifacts::compare` | `canonical.rs:138` | `pub fn compare_cli_ui_artifacts(cli: &CanonicalUiArtifact, ui: &CanonicalUiArtifact) -> ParityMatch` | Structure comparison — mismatch detection. Low risk. |

### Recommended Fuzz Targets

1. **`fuzz_ui_model_decode_postcard`** — Feed arbitrary `&[u8]` to `decode_postcard::<serde_json::Value>()` (CRITICAL)
2. **`fuzz_ui_model_canonicalize_json`** — Feed arbitrary JSON to `canonicalize_cli_artifact()`

---

## 6. vb_benchmark — `crates/vb_benchmark/src/`

### Fuzz Coverage: NONE (zero fuzz targets)

### Attack Surface Summary

This crate contains **performance benchmarking metadata types** and computation functions. All functions accept plain data types — there is no parsing or I/O. The primary attack surface is arithmetic edge cases.

### pub fn Attack Surface Catalog

#### HIGH — String/Text Input

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `capture_metadata` | `lib.rs:194` | `pub fn capture_metadata(name: &str, baseline: Option<Duration>, result: Duration, command: &str, commit_hash: &str, environment: &str, budget_us: u64) -> Result<BenchmarkMetadata, EvidenceError>` | **PRIMARY TARGET.** String validation: `commit_hash` must be non-empty ASCII hex — fuzz with non-ASCII, empty, special characters. `name`/`command`/`environment` are unbounded. |

#### MEDIUM — Numeric Types

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `baseline_within_budget` | `lib.rs:233` | `pub fn baseline_within_budget(baseline: Duration, budget_us: u64) -> bool` | `u128` comparison — `Duration::as_micros()` returns `u128`, compared against `u64` budget. |
| `result_exceeds_threshold` | `lib.rs:242` | `pub fn result_exceeds_threshold(result: Duration, baseline: Duration, threshold_pct: u64) -> bool` | `saturating_mul` and `saturating_add` of `u128` values with `threshold_pct=0` or `threshold_pct=u64::MAX`. |
| `latency_within_budget` | `lib.rs:252` | `pub fn latency_within_budget(elapsed: Duration, budget_us: u64) -> bool` | `budget_us == 0` returns `false` — fuzz with zero budget and Duration::ZERO. |
| `budget_utilization_percent` | `lib.rs:264` | `pub fn budget_utilization_percent(elapsed: Duration, budget_us: u64) -> u128` | `checked_mul(10000)` on `u128` — overflow returns `u128::MAX`. `budget_us == 0` returns `u128::MAX`. |
| `check_evidence_gate` | `lib.rs:285` | `pub fn check_evidence_gate(metadata: &BenchmarkMetadata, threshold_pct: u64) -> Result<(), EvidenceError>` | Combines all of the above — `saturating_mul` and `saturating_sub` for regression detection. |

#### MEDIUM — Duration Type

All functions accept `std::time::Duration` — the maximum representable `Duration` is ~584k years. `as_micros()` returns `u128`. Conversion to `u64` via `as` cast in `capture_metadata` could truncate for pathological Duration values, though practical benchmark durations never exceed `u64::MAX` microseconds (~584k years).

### Recommended Fuzz Targets

1. **`fuzz_benchmark_capture_metadata`** — Feed arbitrary strings to `capture_metadata()`
2. **`fuzz_benchmark_math`** — Feed arbitrary Duration/budget values to all math functions

---

## 7. vb_ui — `crates/vb_ui/src/`

### Fuzz Coverage: NONE (zero fuzz targets)

### Attack Surface Summary

This crate is the **Makepad UI application** with an IPC bridge to the backend server. The IPC bridge processes **network responses** from the server. While Makepad rendering is not a fuzz target, the `ipc_bridge.rs` processes `IpcResponse` types and `IpcPayload` types from the network.

### pub fn Attack Surface Catalog — IPC Entry Points

#### HIGH — Deserialized/Externally-Originated Types

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `IpcBridge::send` | `ipc_bridge.rs:190` | `pub fn send(&self, request: IpcRequest) -> Result<(), String>` | Takes `IpcRequest` enum — `SubmitRun { input: Vec<u8> }` contains arbitrary postcard bytes. `Connect { socket_path: PathBuf }` contains arbitrary socket paths. `AnswerAsk { answer: Vec<u8> }` contains arbitrary bytes. |
| `IpcBridge::poll` | `ipc_bridge.rs:200` | `pub fn poll(&mut self) -> Vec<IpcReply>` | Returns `Vec<IpcReply>` — responses originate from the IPC server (external). Each variant carries `String` or `IpcResponse` payloads. |
| `IpcBridge::new` | `ipc_bridge.rs:181` | `pub fn new() -> Self` | Spawns background thread — `PathBuf` from `Connect` used as Unix socket path. |

#### IPC Response Processing (internal, but attack surface)

The `ipc_thread()` function (private, `ipc_bridge.rs:235`) processes:
- `IpcResponse::RuntimeError { message: String }` — arbitrary error messages
- `IpcResponse::AcceptedRun { run_id: 42 }` — numeric run IDs
- `IpcResponse::TraceCount { count: u32 }` — arbitrary counts
- `IpcResponse::PayloadError { message, .. }` — arbitrary messages
- `IpcResponse::WorkflowResolutionRequired` / `WorkflowDigestMismatch`

These responses come from `IpcClient::recv_response()` which reads from a Unix socket — they are **externally-sourced data**.

### Recommended Fuzz Targets

1. **`fuzz_ui_ipc_response_processing`** — Fuzz the `reply_from_response`, `reply_from_submit`, `reply_from_answer`, `reply_from_drain_trace` functions with arbitrary `IpcResponse` values
2. **`fuzz_ui_ipc_request`** — Fuzz `IpcBridge::send()` with arbitrary `IpcRequest` variants

---

## 8. vb_ui_snapshot — `crates/vb_ui_snapshot/src/`

### Fuzz Coverage: NONE (zero fuzz targets)

### Attack Surface Summary

This crate is a **UI snapshot testing framework** that validates PNG screenshots, layout fixtures, and token values. It parses TOML config, validates PNG dimensions, checks colors, and validates text rendering.

### pub fn Attack Surface Catalog

#### CRITICAL — Raw Image Data (via `image` crate)

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `validate_png_dimensions` | `checks.rs:966` | `pub fn validate_png_dimensions(path: &Path) -> Result<(u32, u32), UiSnapshotError>` | **Opens arbitrary PNG files** via `image::open()` — malicious PNGs (decompression bombs, corrupted headers, oversized dimensions). |
| `check_overlap` | `checks.rs:282` | `pub fn check_overlap(screen_png: &Path) -> Result<OverlapResult, UiSnapshotError>` | Opens PNG + parses layout fixture. |
| `check_clipping` | `checks.rs:297` | `pub fn check_clipping(screen_png: &Path) -> Result<ClippingResult, UiSnapshotError>` | Opens PNG + parses layout fixture. |
| `check_chip_readability` | `checks.rs:314` | `pub fn check_chip_readability(screen_png: &Path) -> Result<ChipReadabilityResult, UiSnapshotError>` | Opens PNG. |
| `check_bounds` | `checks.rs:332` | `pub fn check_bounds(screen_png: &Path, outer_margin: u32, sidebar_width: u32, top_bar_height: u32) -> Result<BoundsResult, UiSnapshotError>` | Opens PNG + layout constants. |
| `check_selected_state` | `checks.rs:359` | `pub fn check_selected_state(screen_png: &Path) -> Result<SelectedStateResult, UiSnapshotError>` | Opens PNG. |
| `check_color_drift` | `checks.rs:781` | `pub fn check_color_drift(screen_png: &Path, tokens: &UiTokens) -> Result<ColorDriftResult, UiSnapshotError>` | Opens PNG, iterates over all pixels matching token colors — could be slow with large images. |
| `check_spelling` | `checks.rs:886` | `pub fn check_spelling(screen_png: &Path) -> Result<SpellingResult, UiSnapshotError>` | Opens PNG, scans for text — pixel-by-pixel scanning could be slow with pathological images. |
| `generate_blank_screenshot` | `checks.rs:990` | `pub fn generate_blank_screenshot(output_path: &Path, width: u32, height: u32) -> Result<(), UiSnapshotError>` | Writes PNG to arbitrary path — `width` and `height` unbounded (could attempt to allocate `width × height × 4` bytes). |

#### HIGH — String/Text Input (TOML Parsing)

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `parse_tokens_from_toml` | `tokens.rs:128` | `pub fn parse_tokens_from_toml(content: &str) -> Result<UiTokens, UiSnapshotError>` | **PRIMARY FUZZ TARGET.** Parses TOML config — arbitrary strings, deeply nested tables, invalid hex colors, negative integer values for `u32` fields. |
| `load_tokens_from_file` | `tokens.rs:121` | `pub fn load_tokens_from_file(path: &Path) -> Result<UiTokens, UiSnapshotError>` | Reads and parses TOML from disk. |
| `tokens_to_rust_constants` | `tokens.rs:286` | `pub fn tokens_to_rust_constants(tokens: &UiTokens) -> String` | Generates Rust source from token values — invalid hex colors produce `[0.0, 0.0, 0.0, 1.0]`. |

#### MEDIUM — Numeric/Layout Types

| Function | File | Signature | Attack Vector |
|----------|------|-----------|---------------|
| `Rect::new` | `layout_kernel.rs:29` | `pub fn new(x: u32, y: u32, width: u32, height: u32) -> LayoutKernelResult<Self>` | `u32` arithmetic — `x + width` or `y + height` overflow returns `CoordinateOverflow`. |
| `overlap_area_px` | `layout_kernel.rs:72` | `pub fn overlap_area_px(first: Rect, second: Rect) -> LayoutKernelResult<u32>` | `checked_sub` and `checked_mul` — underflow returns error. |
| `rect_contains` | `layout_kernel.rs:104` | `pub fn rect_contains(container: Rect, child: Rect) -> LayoutKernelResult<bool>` | Coordinate overflow. |
| `chip_is_readable` | `layout_kernel.rs:123` | `pub fn chip_is_readable(chip: Rect, contrast_milli: u32) -> bool` | `contrast_milli` comparison — values below `CHIP_MIN_CONTRAST_MILLI=4500` return false. |
| `selected_state_is_visible` | `layout_kernel.rs:130` | `pub fn selected_state_is_visible(viewport: Rect, indicator: SelectedIndicator) -> LayoutKernelResult<bool>` | Pattern match on enum. |
| `run_snapshot_command_for_fixture` | `snapshot.rs:11` | `pub fn run_snapshot_command_for_fixture(fixture_id: &str, command_line: &str) -> Result<SnapshotArtifact, UiSnapshotError>` | **STUB** — regex check for `--exit-code 17`. No actual command execution. |

#### MEDIUM — UiTokens (via serde Deserialize)

`UiTokens` derives `Serialize, Deserialize` — if tokens are loaded from deserialized sources, malicious data could inject arbitrary values into all 60+ fields (strings, u32, f32).

### Recommended Fuzz Targets

1. **`fuzz_ui_snapshot_parse_toml`** — Feed arbitrary TOML to `parse_tokens_from_toml()`
2. **`fuzz_ui_snapshot_layout_rect`** — Feed arbitrary (x,y,w,h) tuples to `Rect::new()` and layout functions
3. **`fuzz_ui_snapshot_png_dimensions`** — Feed crafted PNG path fixtures to `validate_png_dimensions()`
4. **`fuzz_ui_snapshot_color_drift`** — Feed arbitrary UiTokens to `check_color_drift()`

---

## 9. vb_verification — `crates/vb_verification/src/`

### Fuzz Coverage: NONE (zero fuzz targets)

### Attack Surface Summary

This is a **Kani verification harness crate** containing only `#[kani::proof]` functions. It has no public API beyond the crate root, and all functions are only compiled under `#[cfg(kani)]`.

### pub fn Attack Surface Catalog

**No public functions.** All functions are under `#[cfg(kani)] mod kani_harnesses` and use `kani::Arbitrary`, `kani::assume`, and `kani::assert`. These are proof harnesses for Kani's model checker, not runtime code.

#### Verification Harness Functions (not runtime attack surface)

| Function | File | Purpose |
|----------|------|---------|
| `hydrate_run_frame_precond_run_id_mismatch` | `lib.rs:41` | Kani proof — verifies hydrate_run_frame returns Err on run_id mismatch |
| `hydrate_run_frame_from_events_precond_empty` | `lib.rs:69` | Kani proof — verifies empty events return Err |
| `hydrate_run_frame_postcond_ok` | `lib.rs:85` | Kani proof — verifies matching run_id doesn't panic |

These Kani proofs test `vb_storage::recovery::hydrate` functions. The Kani harness uses `kani::Arbitrary` on a newtype `ArbitraryRunSnapshot` — this is **compliant** with the GOD RULES (no hardcoded shapes).

### Recommended Fuzz Targets

**No fuzz targets recommended.** This crate contains only Kani proof harnesses. The functions it tests (`hydrate_run_frame`, `hydrate_run_frame_from_events`) live in `vb_storage` and are not in scope for this analysis.

---

## Summary of Priority Fuzz Targets

| Priority | Crate | Function | Input Type | Rationale |
|----------|-------|----------|------------|-----------|
| **P0** | vb_ui_model | `decode_postcard` | `&[u8]` | Network-facing binary decoder — highest attack value |
| **P0** | vb_boundary_inventory | `parse_inventory` | `&[u8]` | JSON parser accepting raw bytes |
| **P0** | vb_boundary_inventory | `validate_evidence_reference_bytes` | `&[u8]` | UTF-8 validator + string pattern matching |
| **P1** | vb_codegen | `emit_rust_workflow` | `&CompiledWorkflow` | Code generator with complex IR validation |
| **P1** | vb_codegen | `format_generated_rust` | `&str` | Spawns rustfmt process with arbitrary input |
| **P1** | vb_ui_snapshot | `parse_tokens_from_toml` | `&str` | TOML config parser |
| **P1** | vb_ui_snapshot | `validate_png_dimensions` | `&Path` (PNG) | Image file parser from `image` crate |
| **P2** | vb_benchmark | `capture_metadata` | `&str`+numeric | String validation (hex commit hash) |
| **P2** | vb_doc | `plan_taint_doc_reconciliation` | `&str` | Text analysis (partial coverage exists) |
| **P2** | vb_proof_kernels | `sequential_compose`/`loop_compose` | `Budget`+`u64` | Budget arithmetic edge cases |
| **P3** | vb_ui | `IpcBridge::send` | `IpcRequest` | IPC request with Vec<u8> payloads |
| **P3** | vb_ui_snapshot | `Rect::new`/`overlap_area_px` | `(u32,u32,u32,u32)` | Layout arithmetic overflow |

### Input Vector Statistics

| Input Category | Count | Crates |
|----------------|-------|--------|
| `&[u8]` | **3** | vb_boundary_inventory (2), vb_ui_model (1) |
| `&str` / `String` | **12** | vb_doc (5), vb_boundary_inventory (2), vb_ui_snapshot (2), vb_benchmark (1), vb_codegen (1), vb_ui_model (1) |
| Parsed/deserialized types | **6** | vb_codegen (2), vb_boundary_inventory (2), vb_ui_model (2) |
| Numeric types (overflow/underflow) | **15** | vb_proof_kernels (8), vb_benchmark (4), vb_ui_snapshot (3) |
| Collections | **4** | vb_codegen (2), vb_ui_snapshot (1), vb_doc (1) |

### Verification Note

The `vb_verification` crate contains **Kani proof harnesses** but no runtime code and no public API. All verification harnesses use `kani::Arbitrary` (compliant with GOD RULE #1). No fuzzing is needed for this crate.

### Missing Fuzz Coverage: 8 of 9 crates have ZERO fuzz coverage

- ✅ `vb_doc` — PARTIAL (1 fuzz target exists)
- ❌ `vb_boundary_inventory` — NONE
- ❌ `vb_codegen` — NONE
- ❌ `vb_proof_kernels` — NONE
- ❌ `vb_ui_model` — NONE
- ❌ `vb_benchmark` — NONE
- ❌ `vb_ui` — NONE
- ❌ `vb_ui_snapshot` — NONE
- ❌ `vb_verification` — N/A (Kani-only crate)

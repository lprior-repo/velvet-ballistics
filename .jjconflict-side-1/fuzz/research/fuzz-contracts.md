# Fuzz Contracts Catalog — velvet-ballistics

**Generated:** 2026-05-24  
**Scope:** All existing contracts, mandates, findings, and CI gates requiring fuzzing  
**Sources scanned:** 10 files across contracts/, docs, .moon/, and root

---

## 1. velvet-ballistics-MASTER.md (Authoritative Build Plan)

### 1.1 Section 21 — IPC Protocol
- **File:** `velvet-ballistics-MASTER.md` lines 1062–1073
- **Mandate:** "5. Fuzz arbitrary bytes." for IPC decoder
- **Function/module:** `vb_ipc::frame` (decoder)
- **Invariants:** Validate magic before allocation; validate payload length before reading; decode Postcard into typed payloads only; return typed IPC errors for malformed frames
- **Fuzz target:** `ipc_frame`
- **Tool:** `cargo-fuzz`

### 1.2 Section 37 — Fuzz Targets
- **File:** `velvet-ballistics-MASTER.md` lines 1532–1544
- **Mandate:** Five required fuzz harnesses at `fuzz/src/bin/*.rs`:

| Target | Requirement |
|--------|-------------|
| `yaml_events` | Arbitrary UTF-8 bytes → parser never panics |
| `expression` | Arbitrary UTF-8 bytes → lexer/parser/compiler never panics |
| `ipc_frame` | Arbitrary bytes → decoder never panics, length checks hold |
| `journal_event` | Arbitrary bytes → Postcard decode failure is typed |
| `compiled_ir` | Arbitrary bytes → decode/validate never panics |

- **Coverage requirement:** All five targets MUST exist and MUST never panic on arbitrary bytes.
- **Tool:** `cargo-fuzz`

### 1.3 Section 40 — CI Gate
- **File:** `velvet-ballistics-MASTER.md` lines 1619–1658
- **Mandate:** CI must gate on `moon ci` which MUST include `fuzz-smoke` task.
- **Mandatory CI commands include:** `cargo fuzz build`
- **Fuzz-smoke task:** Must exist in Moon and gate CI.

### 1.4 Section 35 — Implementation Phases
- **File:** `velvet-ballistics-MASTER.md` lines 1383–1450
- **Phase 3:** "Strict YAML event parser — saphyr-parser wrapper, YAML profile rejection, source maps, **fuzz**."
- **Phase 30:** "Binary IPC — mio Unix socket loop, required commands, **frame fuzzing**."
- **Phase 34:** "Hardening — Full gates, sanitizer jobs, **fuzz expansion**, docs, bead evidence."
- **IPC area:** "Socket-loop **fuzz/backpressure evidence** and runtime integration gates remain required."
- **Tests/audits area:** "Full matrix gates, **fuzz**, Miri, coverage, mutants, sanitizer… still required."
- **Round 2 status rule:** Public function existence is NOT proof phase is complete unless "required tests, **fuzz/property coverage**, benchmark evidence… have actually passed."

### 1.5 Section 45 — Normative Runtime Semantics
- **File:** `velvet-ballistics-MASTER.md` lines 1868–1869
- **Definition of Done item 23:** "Full current-scope gates pass… **fuzz smoke**…"
- **DoD item 24:** "Every phase parent bead, function-cluster child bead, **fuzz target bead**, benchmark bead… is closed with evidence."

### 1.6 Section 42 — Bead Work Breakdown
- **File:** `velvet-ballistics-MASTER.md` lines 1724
- "Each **fuzz target requires its own bead**."

### 1.7 Section 50 — Threat Model
- **File:** `velvet-ballistics-MASTER.md` line 2771
- "Malformed IPC frames — Magic validation, length bounds, typed IPC errors, **fuzz coverage**"

### 1.8 Section 77.8 — Property Tests, Fuzz Harnesses, Proof Targets
- **File:** `velvet-ballistics-MASTER.md` lines 4810–4823
- **Fuzz rules for every binary decoder:**
  1. Fuzz arbitrary bytes
  2. Assert typed error or valid object
  3. Never panic
  4. Never allocate before length validation
- **Tool:** `cargo-fuzz`
- **Creation commands:** `cargo xtask fuzz-target new yaml_events`, `cargo xtask fuzz-target new ipc_frame`

### 1.9 Section 77.23 — AI-Safe Code Zones
- **File:** `velvet-ballistics-MASTER.md` lines 5094–5100
- **Zone `storage-decode`:** Marker `// velvet-zone: storage-decode`, rule: "No allocation before length validation, **fuzz coverage required**"

### 1.10 Evidence Patch Footer
- **File:** `velvet-ballistics-MASTER.md` line 5086
- `fuzz build: not required, parser untouched` — shows category where fuzz is conditionally waived

---

## 2. contracts/proof_obligations.yaml (Formal Proof Registry)

### 2.1 Proof Level L2 — Fuzz / Mutation / Crash-lab
- **Definition:** "Fuzzing, mutation testing, crash-lab fault injection"
- **Tool:** `cargo fuzz`
- All obligations at level L2 require fuzz execution evidence.

### 2.2 VB-CORE-SIGNAL-001 (EngineSignal Finished carries Taint)
- **Fuzz required:** `expr_eval` target
- **Command:** `cargo fuzz run expr_eval`
- **Crate:** `vb_core`

### 2.3 VB-IPC-DECODE-001 (Reject before allocation)
- **Fuzz required:** `ipc_frame` target
- **Command:** `cargo fuzz run ipc_frame`
- **Crate:** `vb_ipc`
- **Why fuzz:** Decoder validates magic + payload length before allocating bytes

### 2.4 VB-IPC-DECODE-003 (IPC frame format unified)
- **Fuzz required:** `ipc_frame` target
- **Command:** `cargo fuzz run ipc_frame`
- **Crate:** `vb_ipc`

### 2.5 VB-STORAGE-DECODE-004 (payload_len <= max)
- **Fuzz required:** `decode_record` target
- **Command:** `cargo fuzz run decode_record`
- **Crate:** `vb_storage`

### 2.6 VB-EXPR-001 (Bytecode == AST equivalence)
- **Fuzz required:** `expr_bytecode` target
- **Command:** `cargo fuzz run expr_bytecode`
- **Crate:** `vb_expr`

### 2.7 VB-EXPR-003 (F64 arithmetic/literals)
- **Fuzz required:** `expr_eval` target
- **Command:** `cargo fuzz run expr_eval`
- **Crate:** `vb_expr`

### 2.8 Proof Matrix by Crate
| Crate | Strategy | Requires Fuzz |
|-------|----------|----------------|
| `vb_expr` | "Property tests + differential tests. **Fuzz arbitrary expression bytes.**" | YES |
| `vb_yaml` | "**Fuzz** + golden diagnostics. Prove 'no panic, typed failure, no forbidden YAML accepted'." | YES |
| `vb_storage` | "Kani for envelope header decode. **Fuzz records.**" | YES |
| `vb_ipc` | "Kani + **fuzz**. Key property: 'reject before allocation'." | YES |

### 2.9 Evidence Schema — Verified Status Values
- `ipc_decoder`: `fuzzed_and_bounded_model_checked` | `bounded_model_checked` | `fuzzed`
- `replay_safety`: `model_checked_and_crash_lab_evidenced` | `model_checked` | `fuzzed`

---

## 3. contracts/invariants.yaml (Mechanical Invariants)

### 3.1 IPC Invariants (implicit fuzz obligations)
- **id: `ipc_reject_before_alloc`** — Must validate magic + payload_len before allocation → requires fuzz to prove this holds for arbitrary bytes
- **id: `ipc_no_payload_alloc_on_error`** — Failed header validation must not allocate → requires fuzz to exercise error paths
- **id: `record_magic_before_alloc`** — Record decoder validates magic before payload allocation → `decode_record` fuzz target
- **id: `record_crc_before_trust`** — CRC validated before Postcard trust → fuzz target must exercise this ordering

### 3.2 Taint Lattice Invariants
- **After scan commands:** No direct fuzz commands, but lattice properties are proven by Verus L4. Fuzz acts as defense-in-depth for expression propagation.

---

## 4. contracts/perf-budget.yaml
- **No fuzz mandates.** Performance benchmarks only.

---

## 5. contracts/*.cue files
- **No fuzz-relevant constraints found.** These define schema shapes for evidence bundles, UI tokens, CLI envelopes, diagnostics, and gate outputs — none reference fuzz.

---

## 6. .moon/tasks/all.yml (Moon CI Configuration)

### 6.1 `fuzz-smoke` Task
- **Lines:** 448–477
- **What it does:**
  1. `cargo fuzz build --target x86_64-unknown-linux-gnu`
  2. Runs 4 targets with `-max_total_time=1`: `yaml_events`, `ipc_frame`, `journal_event`, `compiled_ir`
- **Inputs trigger when:** `fuzz/**/*`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, and any source in crates `vb_core`, `vb_yaml`, `vb_expr`, `vb_compile`, `vb_ipc`, `vb_storage` change
- **runInCI:** `true`
- **Notable gap (per BIG-ASS-TESTING):** Only 4 of the many targets are run. No corpus is validated. Total time=1 second per target is a smoke check, not a fuzz run.

### 6.2 Toolchains
- `.moon/toolchains.yml` line 8: Rust nightly includes `cargo-fuzz` component

---

## 7. BIG-ASS-TESTING-TO-FIX.md (Multi-Round Audit)

### 7.1 Approved Targets (Round 1, Round 2)
- `yaml_events` — well-implemented with corpus ✅
- `expr_eval` — well-implemented with corpus (note: `expr_eval` vs `expression` naming drift)

### 7.2 REJECTED / LETHAL Findings

#### Round 2 Finding: `generated_compare` is a STUB
- Deserializes bytes, discards all results, no comparison

#### Round 3 Finding: `journal_event` fuzz target MISSING
- "fuzz/journal_event target does not exist — DRIFT-2 can't be closed"

#### Round 4 Findings (Critical):
1. **`generated_compare` STUB** — discards all results
2. **`compiled_ir` STUB** — discards results
3. **`ipc_frame` discards all decode results** — uses `match { Ok(_) | Err(_) => {} }`
4. **`expression` discards eval results** — `let _result =` never asserted
5. **`decode_record` uses `.ok()`** suppressing all failures
6. **`collect_page` pagination fuzz target MISSING ENTIRELY**
7. **Zero corpus entries** for `compiled_ir`, `generated_compare`, `ipc_frame`, `expression`
8. **No minimization config** — `fuzz/Cargo.toml` has no `[package.metadata.cargo-fuzz]`
9. **`fuzz-smoke` only builds, doesn't meaningfully run** — `-max_total_time=1` is cosmetic

### 7.3 MUST_FIX (7/8 lethal blockers involving fuzz)
- **MUST_FIX #7:** `journal_event` fuzz target missing (DRIFT-2)

---

## 8. test-plan-remaining-lethals.md (C.1–C.25 Findings)

### 8.1 Category 1: Fuzz Infrastructure (C.3, C.21–C.25)

| ID | Finding | Severity | Status |
|----|---------|----------|--------|
| C.3 | No minimization config in `fuzz/Cargo.toml` | MAJOR | Needs `[package.metadata.cargo-fuzz]` with `sancov_timeout = 60` and `libfuzzer_options = ["-len_control=1"]` |
| C.21 | `generated_compare` fuzz is STUB — drops `validate_compiled_workflow` and `try_from_parts` results | LETHAL | Must assert validation result, conversion result, and digest stability across two decodes |
| C.22 | `compiled_ir` fuzz is STUB — drops `try_from_parts` result, never validates node indices/slot bounds | LETHAL | Must assert digest equality, node_count consistency, and bounds on all node indices |
| C.23 | `ipc_frame` fuzz discards decode results — `Ok(_) \| Err(_) => {}` drops everything | LETHAL | Must assert on `Ok(decoded)` field invariants and exhaustively match `Err` error variants |
| C.24 | `expression` fuzz discards eval results — `let _result =` never asserted | LETHAL | Must assert no-panic, typed errors, taint monotonicity, and Clean-input → Clean-output |
| C.25 | `collect_page` pagination fuzz MISSING ENTIRELY | LETHAL | Must fuzz page_size boundaries, cursor positions, list/non-list inputs, empty lists |

### 8.2 C.3 — Minimization Config
- **What must exist:** `[package.metadata.cargo-fuzz]` with `sancov_timeout = 60` and `libfuzzer_options = ["-len_control=1"]`
- **Acceptance:** `cargo fuzz build --target <any>` produces `minimized_*` artifacts after `cargo fuzz run --dedup`
- **Test:** `cargo fuzz run taint_propagation -- -minimize_contribs=1` then `ls fuzz/corpus/taint_propagation/minimized` must not be empty

### 8.3 C.21 — generated_compare specifics
- Must decode bytes as `WorkflowParts` via postcard
- Must assert `validate_compiled_workflow` result
- Must assert `CompiledWorkflow::try_from_parts` result
- Must compare digest across two independent decode passes
- Must assert exact error variants on invalid input, exact equality on valid

### 8.4 C.22 — compiled_ir specifics
- Must validate `try_from_parts` returns same digest as decoded parts
- Must assert node_count matches parts
- Must assert all node indices are valid within workflow
- Must run: `cargo fuzz run compiled_ir -- -runs=100000` with corpus covering valid+invalid postcard-encoded `WorkflowParts`

### 8.5 C.23 — ipc_frame specifics
- Must distinguish `Ok` vs `Err` paths
- Must assert `decoded.payload.len() <= header.payload_len`
- Must exhaustively match error variants (not wildcard)

### 8.6 C.24 — expression specifics
- Must assert `eval_expr_program` returns `Result` never panics
- Must assert taint monotonicity: output taint ≥ max input taint
- Must assert Clean inputs produce Clean output
- Must assert type errors returned as typed `EvalError` variants

### 8.7 C.25 — collect_page specifics
- New target: `fuzz/src/bin/collect_page_pagination.rs`
- Must test page size boundaries (0, 1, max, overflow)
- Must test cursor positions across page boundaries
- Must test non-list collector types
- Must test empty lists
- Must assert page count = ceil(list_len / page_size)
- Must assert each page item count ≤ page_size
- Must assert non-list inputs return typed error

---

## 9. test-suite-review-c1-c25.md (Mode 2 Suite Inquisition)

### 9.1 Verdict: REJECTED
- Three LETHAL findings involving fuzz

### 9.2 LETHAL-2: Thin-wrapper fuzz targets mask weak assertions
- **Files:** `fuzz/fuzz_targets/generated_compare.rs`, `compiled_ir.rs`, `ipc_frame.rs`, `expression.rs`
- **Finding:** All four are thin wrappers (30–31 lines each) calling `fuzz_lib::fuzz_*`. Actual implementations in `fuzz/src/lib.rs` contain:
  - `fuzz/src/lib.rs:54` — `assert!(result.is_ok())` (weak)
  - `fuzz/src/lib.rs:214` — `match decode_frame_payload(...) { Ok(_) | Err(_) => {} }` (silent drop)
  - `fuzz/src/lib.rs:232,240,249` — `let _ = vb_ipc::IpcFrameHeader::decode(...)` (suppressed errors)
- **Required fix:** Either (a) move fuzz target bodies into the `.rs` files directly, OR (b) audit `lib.rs` and fix all weak assertions to use exact variant matching

### 9.3 MAJOR-2: `collect_page_pagination.rs` is a stub
- File at `fuzz/src/bin/collect_page_pagination.rs` (47 lines) calls `fuzz_lib::fuzz_collect_page_pagination` which **does not exist** in `fuzz/src/lib.rs`
- Must implement or remove

### 9.4 MINOR-4: `fuzz/src/lib.rs:1518–1520`
- Three `let _ = ...` in `fuzz_strict_artifact_decoder` suppress all decode results

---

## 10. test-review-remaining-lethals.md (Plan Review)

### 10.1 Summary
- 9 LETHAL findings, 4 MAJOR
- "8 new fuzz targets + 3 fixes" claimed but specifications exist unverified
- "PARTIAL — specifications exist but not verified"

### 10.2 Fuzz-Specific LETHALs
| ID | Issue | Detail |
|----|-------|--------|
| C.21 | `generated_compare` STUB | `assert!(validated.is_ok())` is weak assertion (LETHAL per skill) |
| C.23 | `ipc_frame` discards | Incomplete `matches!` with `// ... exhaustive error variants` placeholder |
| C.24 | `expression` discards | Taint monotonicity assertion can be deleted without test failure |
| C.22 | `compiled_ir` STUB | Slot index bounds check can be deleted without test failure |
| C.25 | Pagination fuzz missing | "max" page_size and overflow cursor undefined |

### 10.3 Mandatory Remediation for Fuzz Targets
Before resubmission:
1. **C.21:** Replace `assert!(validated.is_ok())` with `assert_eq!(validated, Ok(expected))` or exact value matching
2. **C.23:** Fill in exhaustive error variant match (no wildcard placeholder)
3. **C.22:** Add mutation checkpoint — removing slot index bounds check must fail a test
4. **C.24:** Add mutation checkpoint — removing taint monotonicity assertion must fail; consider adding type assertion on `SlotValue`

---

## 11. Existing Fuzz Targets (Directory Scan)

### 11.1 `fuzz/src/bin/*.rs` — 46 targets exist
Including: `yaml_events.rs`, `expression.rs`, `ipc_frame.rs`, `compiled_ir.rs`, `journal_event.rs`, `expr_eval.rs`, `decode_record.rs`, `generated_compare.rs`, `collect_page_pagination.rs`, `taint_propagation.rs`, `expr_bytecode.rs`, `resource_budget.rs`, `ipc_decode.rs`, `replay_events.rs`, `admission_fuzz.rs`, `slot_value_roundtrip.rs`, `strict_yaml_profile.rs`, `recovery_decode.rs`, `accessor_traversal.rs`, etc.

### 11.2 `fuzz/fuzz_targets/*.rs` — 12 targets exist
Including: `expr_eval.rs`, `journal_event.rs`, `decode_record.rs`, `vb_storage_codec.rs`, `lex_expr.rs`, `ui_redaction_artifact.rs`, etc.

### 11.3 Noted Duplication
- Both `fuzz/src/bin/` and `fuzz/fuzz_targets/` directories exist with some overlapping target names (`expr_eval`, `journal_event`, `decode_record`). Appears `fuzz/src/bin/` is the canonical location.

---

## 12. Summary: Complete Fuzz Contract Index

### 12.1 Functions/Modules That MUST Be Fuzzed

| # | Function / Module | Crate | Fuzz Target | Status | Source |
|---|-------------------|-------|-------------|--------|--------|
| 1 | YAML event parser | `vb_yaml` | `yaml_events` | ✅ APPROVED (has corpus) | §37, §46 |
| 2 | Expression lexer/parser/compiler | `vb_expr` | `expression` / `expr_eval` | ⚠️ `expression` discards results; `expr_eval` APPROVED | §37, §46, C.24 |
| 3 | IPC frame decoder | `vb_ipc` | `ipc_frame` | ❌ Discards all results | §21, §37, C.23 |
| 4 | Journal event (Postcard decode) | `vb_storage` | `journal_event` | ❌ Reported MISSING in audits but file exists | §37, DRIFT-2 |
| 5 | Compiled IR decode/validate | `vb_core` | `compiled_ir` | ❌ STUB (discards results) | §37, C.22 |
| 6 | Generated Rust vs IR comparison | `vb_codegen` | `generated_compare` | ❌ STUB (discards all results, no comparison) | Round 2, C.21 |
| 7 | Storage record decode | `vb_storage` | `decode_record` | ❌ Uses `.ok()` suppressing all failures | §18/49, Round 4 |
| 8 | collect_page pagination | `vb_runtime` | `collect_page_pagination` | ❌ MISSING ENTIRELY (stub binary, no implementation) | C.25, Round 4 |
| 9 | Expression bytecode vs AST | `vb_expr` | `expr_bytecode` | Required by VB-EXPR-001 | PO |
| 10 | Taint propagation | `vb_core` | `taint_propagation` | Exists (bin at least) | C.3, C.24 |
| 11 | Resource budget | `vb_core` | `resource_budget` | Exists | §45, PO |
| 12 | Strict YAML profile | `vb_yaml` | `strict_yaml_profile` | Exists | Phase 3 |
| 13 | Recovery decode | `vb_storage` | `recovery_decode` | Exists | §49 |
| 14 | Accessor traversal | `vb_core` | `accessor_traversal` | Exists | PO |
| 15 | Admission fuzz boundary | `vb_runtime` | `admission_fuzz` | Exists | §49 |
| 16 | Slot value roundtrip | `vb_core` | `slot_value_roundtrip` | Exists | PO |
| 17 | IPC decode (variant) | `vb_ipc` | `ipc_decode` | Exists | PO |
| 18 | Replay events | `vb_runtime` | `replay_events` | Exists | §49 |
| 19 | Unsafe-fuzz-cabi-isolation | (boundary) | `unsafe-fuzz-cabi-isolation` | Bead group exists | §42 |

### 12.2 Invariants That MUST Hold Under Fuzz

1. **Never panic** — applies to ALL binary decoders (§77.8, §37)
2. **No allocation before length validation** — IPC, storage decoders (§21, §18, §77.8, §77.23)
3. **Typed error or valid object** — every decode path must return either typed error or valid output (§77.8)
4. **Taint monotonicity** — output taint ≥ max input taint (C.24)
5. **Clean inputs → Clean output** — expression evaluation (C.24)
6. **Page count = ceil(list_len / page_size)** — collect_page pagination (C.25)
7. **Each page item count ≤ page_size** — collect_page pagination (C.25)
8. **Digest stability** — two independent decodes produce same digest (C.21, C.22)
9. **Node indices valid within workflow** — compiled_ir (C.22)
10. **decoded.payload.len() ≤ header.payload_len** — IPC frame (C.23)
11. **Exact error variant matching** — all fuzz targets must match exact variants, not `is_ok()/is_err()` (LETHAL-2)

### 12.3 Error Handling That MUST Be Exercised Under Fuzz

1. IPC: `PayloadTooLarge`, `FrameChecksumMismatch`, `BadMagic`, `HeaderLenInvalid` — exhaustive list
2. Storage: `BadMagic`, `SchemaMismatch`, `KindMismatch`, `PayloadTooLarge`, `CrcMismatch`, `DigestMismatch`
3. Expression: `EvalError` variants — type errors, stack overflow, unknown op, F64 unsupported
4. Postcard: All deserialization errors forwarded as typed errors
5. collect_page: `CollectPageNotList` for non-list sources
6. compiled_ir: Validation failures (unreachable nodes, forward-only violations, loop pairing)

### 12.4 Boundary Conditions That MUST Be Tested Under Fuzz

1. **Empty inputs** — zero-length byte slices, empty lists in pagination
2. **Max values** — page_size max, cursor overflow, payload_len at configured maximum
3. **One-byte inputs** — single byte must not panic
4. **Multi-MB inputs** — large inputs must be rejected or handled without panic
5. **Malformed magic** — wrong magic bytes at every position
6. **Truncated headers** — header_len mismatches
7. **Negative/impossible lengths** — payload_len > data.len()
8. **Corrupt CRC/BLAKE3** — manipulated digests
9. **Invalid discriminants** — enum discriminants with out-of-range values (Postcard)

### 12.5 ASAN/UBSAN Coverage Requirements

- Nightly sanitizer jobs **required** for: `vb_runtime`, `vb_ipc`, `vb_storage`, and all binary decoding crates (§40, line 1635)
- `.moon/tasks/all.yml` defines `sanitizer-address-check` task at line 479
- No explicit UBSAN task — only address sanitizer is configured
- No MSAN or TSAN tasks defined

### 12.6 Corpus / Seed Data Requirements

1. **Approved with corpus:** `yaml_events`, `expr_eval` ✅
2. **Zero corpus entries:** `compiled_ir`, `generated_compare`, `ipc_frame`, `expression` ❌
3. **Minimization config MISSING:** `fuzz/Cargo.toml` needs `[package.metadata.cargo-fuzz]` with `sancov_timeout = 60` and `libfuzzer_options = ["-len_control=1"]`
4. **Corpus coverage requirement:** Both valid AND invalid inputs for each target (C.22)

### 12.7 Fuzzer Requirements

- **Canonical tool:** `cargo-fuzz` (libfuzzer backend) — mandated by §37, §77.8, `.moon/toolchains.yml`
- **No AFL++ or honggfuzz mandates found** — only `cargo-fuzz` is referenced
- **Builder variant:** `cargo fuzz build` for CI, `cargo fuzz run` for evidence
- **Minimization:** `cargo fuzz run -- -minimize_contribs=1` and `cargo fuzz run --dedup`

### 12.8 Remaining Gaps (What Is Not Yet Satisfied)

| Gap ID | Description | Blocker? |
|--------|-------------|----------|
| GAP-1 | `generated_compare` is a STUB — no assertions | ❌ LETHAL |
| GAP-2 | `compiled_ir` is a STUB — drops results | ❌ LETHAL |
| GAP-3 | `ipc_frame` discards all decode results | ❌ LETHAL |
| GAP-4 | `expression` discards eval results, no taint assertion | ❌ LETHAL |
| GAP-5 | `collect_page_pagination` implementation MISSING | ❌ LETHAL |
| GAP-6 | `decode_record` uses `.ok()` suppressing failures | ❌ LETHAL |
| GAP-7 | No minimization config in `fuzz/Cargo.toml` | MAJOR |
| GAP-8 | Zero corpus for 4 targets | MAJOR |
| GAP-9 | `fuzz-smoke` only runs 1 sec per target — cosmetic | MAJOR |
| GAP-10 | Thin-wrapper pattern masks weak `lib.rs` assertions | LETHAL |
| GAP-11 | No UBSAN/TSAN tasks defined | MINOR |
| GAP-12 | `property_tests.rs` empty — proptest infrastructure missing | LETHAL (not fuzz, but cross-cutting) |

---

## Appendix A: Fuzz Evidence Commands (from proof_obligations.yaml)

```bash
cargo fuzz run expr_eval          # VB-CORE-SIGNAL-001, VB-EXPR-003
cargo fuzz run ipc_frame          # VB-IPC-DECODE-001, VB-IPC-DECODE-003
cargo fuzz run decode_record      # VB-STORAGE-DECODE-004
cargo fuzz run expr_bytecode      # VB-EXPR-001
cargo fuzz build                  # CI gate (§40)
cargo fuzz run yaml_events        # §37
cargo fuzz run journal_event      # §37
cargo fuzz run compiled_ir        # §37
cargo fuzz run collect_page_pagination  # C.25 (NEW)
```

## Appendix B: Fuzz Smoke CI Target List (from .moon/tasks/all.yml)

Currently runs only 4 targets:
- `yaml_events`
- `ipc_frame`
- `journal_event`
- `compiled_ir`

Missing from CI smoke: `expression`, `expr_eval`, `expr_bytecode`, `decode_record`, `generated_compare`, `collect_page_pagination`, `taint_propagation`, `resource_budget`, `replay_events`, etc.

---

## Appendix C: Key Reference Sections

| Document | Lines/Section | Topic |
|----------|---------------|-------|
| MASTER.md | §37 (1532–1544) | Fuzz target table |
| MASTER.md | §40 (1619–1658) | CI gate including fuzz-smoke |
| MASTER.md | §21 (1062–1073) | IPC decoder "Fuzz arbitrary bytes" |
| MASTER.md | §35 (1383–1450) | Implementation phases with fuzz requirements |
| MASTER.md | §42 (1724) | "Each fuzz target requires its own bead" |
| MASTER.md | §45 (1868–1869) | DoD requires fuzz smoke |
| MASTER.md | §77.8 (4810–4823) | Fuzz rules for every binary decoder |
| MASTER.md | §77.23 (5094–5100) | storage-decode zone requires fuzz coverage |
| proof_obligations.yaml | L2, multiple sections | Fuzz obligations per function |
| invariants.yaml | IPC, Storage sections | Invariants requiring fuzz coverage |
| .moon/tasks/all.yml | 448–477 | fuzz-smoke task definition |
| BIG-ASS-TESTING-TO-FIX.md | Rounds 1–4 | Audit with LETHAL findings |
| test-plan-remaining-lethals.md | C.3, C.21–C.25 | Detailed remediation plans |
| test-suite-review-c1-c25.md | full | REJECTED — 3 LETHAL in fuzz |
| test-review-remaining-lethals.md | 60–199 | Plan review — 9 LETHAL |

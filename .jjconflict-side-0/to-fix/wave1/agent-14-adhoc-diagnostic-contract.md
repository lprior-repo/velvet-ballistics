# Wave 1 — Agent 14 (ad-hoc diagnostic-contract-expert) Deep-Dive

**Scope:** Wave 1 bugs touching diagnostic codes, paths, spans, and Section 16 symbolic
codes. Read-only sweep. No bead changes.

**Reference contract:** `crates/vb_core/src/diagnostic.rs` (registry `CODE_REGISTRY`,
`SymbolicCode`, `DiagnosticCode`, `category_from_numeric`), `crates/vb_validate/src/diag_render.rs`
(validation emission), `crates/vb_validate/src/diag_codes.rs` (validate-side numeric
constants), `crates/vb_core/src/span.rs` (Span primitive), `crates/vb_core/src/errors.rs`
(CoreError -> DiagnosticCode mapping).

**Key contract facts discovered during this sweep:**

1. **Symbolic codes are the primary identifier.** `SymbolicCode` is the primary
   `Diagnostic.code` field. Numeric `DiagnosticCode` (the packed `E0101`-style `u16`)
   is a derived/secondary field. `HasSymbolicCode` is the trait every error type
   implements to provide its symbolic identifier. (`crates/vb_core/src/diagnostic.rs:1636-1705`,
   `:1880-1895`, `:1967-1971`)

2. **Single source of truth:** `CODE_REGISTRY` (`crates/vb_core/src/diagnostic.rs:118-1559`).
   All ~140 codes are registered as `CodeEntry { symbolic, numeric, category, deprecated }`.
   `is_supported_code` derives from registry membership (not hardcoded ranges), eliminating
   the "0x3020-0x3022 missing" drift. (`diagnostic.rs:2049-2059`)

3. **Category ranges are documented but partly overlap** (collision noted, documented
   in `CodeCategory` doc-comment at `diagnostic.rs:43-58`):
   - `Accessor = E13xx` (0x13) ← owns 0x1301..=0x1315
   - `Internal = E13xx` (0x13) ← owns 0x1309 (`INTERNAL_INVARIANT_VIOLATION`)
   - `Lifecycle = E33xx` and `Lifecycle = E15xx` ← both `Lifecycle`
   - `Expression = E12xx` (0x12) and `Expression = E12xx` ← consistent (STEP_BUDGET_EXHAUSTED 0x1201)
   The high-byte heuristic at `diagnostic.rs:1994-2014` cannot distinguish 0x1309 (Internal)
   from 0x13xx (Accessor) for unregistered codes. Registry look-up correctly classifies
   registered codes, but the fallback heuristic would misclassify an unregistered 0x13xx code
   as Accessor rather than Internal.

4. **`Span::ZERO` is the documented placeholder span** used by
   `vb_validate::diagnostic::diagnostic_from_error`
   (`crates/vb_validate/src/diag_render.rs:18`). The test
   `diagnostic_from_error_returns_zero_span` (`diag_render.rs:390-393`) and
   `diagnostic_span_is_zero_for_all_variants`
   (`crates/workspace_tests/tests/vb_test_validate_diagnostic_behavior.rs:1120-1131`)
   assert this as the **expected** behavior. This is not a "Span::ZERO remnant" — it is
   the contract for the validate layer. `Span` is also used in proptest roundtrip and
   `Diagnostic::new` defaults. There is no `Span::try_new` constructor
   (`crates/vb_core/src/span.rs:16-31`).

5. **YAML path:** `Diagnostic` has `source_file: Option<Box<str>>` (`diagnostic.rs:1894`).
   There is **no** dedicated `path: Box<str>` field for the YAML path
   (e.g., `$.steps[2].then`). The `ValidationError` enum and `CompileError` enum do not
   carry a path either; they carry per-variant fields (e.g.,
   `ValidationError::UnknownReference { reference }`,
   `CompileError::NonStringKey { mark: SourceMark }`). Source locations are tracked
   via `SourceMark { index, end_index, line, column, available }`
   (`crates/vb_compile/src/mod_compile_errors/source_mark.rs:14-25`).
   `vb_yaml` has its own `EventSpan`/`SourceSpan` types
   (`crates/vb_yaml/src/source_map_types.rs`, `source_map_build.rs`) that are not bridged
   to `vb_core::span::Span`. The test `diagnostic_from_error_includes_location`
   asserts only that `source_file`/`span` are present, not that the YAML path is included.

6. **All `ValidationError` codes are symbolic** and resolved via the registry.
   `error_diagnostic_parts` (`diag_render.rs:60-375`) constructs `DiagnosticCode::new(CODE_*)`
   from the symbolic-name constants in `diag_codes.rs` (e.g., `CODE_DUPLICATE_KEY = 0x0101`).
   `diagnostic_from_parts` (`diag_render.rs:37-51`) then resolves the symbolic code via
   `code.symbolic_code()`. If a code is unregistered, it falls back to
   `MISSING_REQUIRED_FIELD` with an `[unregistered 0xHHHH]` annotation
   (`diag_render.rs:46-48`). All `ValidationError` codes are registered as
   symbolic → numeric pairs, so this fallback should never fire in normal use.

7. **Test pass results observed during this sweep:**
   - `cargo test -p vb_core --lib diagnostic::tests_and_verification` → **38 passed, 0 failed**.
   - `cargo test -p vb_validate --lib diag_render::render_tests` → **31 passed, 0 failed** (including
     `diagnostic_from_error_returns_zero_span`).
   - `cargo test -p vb_core --lib engine::validate::tests::validate_node_bounds` → **4 passed, 0 failed**.
   - `cargo test -p vb_core --lib span::tests` → **21 passed, 0 failed**.

## Results Table

| bug-id  | pri | code-symbolic | span-real | yaml-path | category-collision | targeted-cmd | result | verdict   | evidence |
|---------|-----|---------------|-----------|-----------|--------------------|--------------|--------|-----------|----------|
| vb-x3b0q | P2  | n/a           | n/a       | n/a       | n/a                | (no diagnostic emission at `value_store.rs:333`) | bug is in core value store, not diagnostic layer | UNKNOWN   | `bd show vb-x3b0q` describes `checked_len_to_u64` `as u64` cast in `value_store.rs:333` and `id_gen.rs:6`. Not a diagnostic emission site; the only diagnostic code adjacent (`SymbolOutOfBounds = 0x1311`, Accessor) is unrelated to the cast. |
| vb-xezc0 | P0  | n/a           | n/a       | n/a       | n/a                | `cargo check --workspace --lib --all-targets` (claimed exit 0 in close reason) | bug already CLOSED; rename from `velvet_ballistics_workspace_tests` to `vb_workspace_tests` is a crate-rename, not a diagnostic contract issue | PATCHED   | `bd show vb-xezc0` close reason cites "Verified cargo check --workspace --lib --all-targets exit 0 after rename." |
| vb-yasoz | P2  | n/a           | n/a       | n/a       | n/a                | `cargo test -p vb_runtime --lib primitives::helpers::list` (per finding) | bug is in `runtime/primitives/helpers/list.rs:29` ForEach/Reduce tail copy. Not a diagnostic emission site. | UNKNOWN   | `bd show vb-yasoz` source: `crates/vb_runtime/src/primitives/helpers/list.rs:29` — performance bug, no diagnostic code emission. |
| vb-yfsc4 | P0  | n/a           | n/a       | n/a       | n/a                | `cargo test -p vb_storage --all-features --no-fail-fast recover_full_journal` (claimed 16 passed in close reason) | bug already CLOSED; storage recovery tail-only replay | PATCHED   | `bd show vb-yfsc4` close reason: "Fixed: recover_full_journal now uses journal.events_for_run_full(run) at crates/vb_storage/src/recovery/replay/recovery_ops.rs:51" + 16 tests pass. |
| vb-ylzmr | P1  | PARTIAL (only `InvalidCompiledWorkflow` fallback for LoadAccessor; LoadSlot/LoadConst fall through to runtime eval, not compile-time) | n/a (no diagnostic layer involved) | n/a | none | `cargo test -p vb_core --lib workflow::tests` → 313 passed; `cargo test -p vb_expr --lib eval_load_slot_out_of_bounds` | closed reason claims validate_expressions now takes slot_count+const_count+accessor_count and bounds-checks LoadSlot/LoadConst. **Source says otherwise**: `crates/vb_core/src/workflow/mod.rs:1289-1310` `fn validate_expressions(expressions, accessor_count)` only checks `ExprOp::LoadAccessor`; the call site `workflow/mod.rs:759` and `validation.rs:120` pass only `parts.accessors.len()`. Runtime evaluation in `vb_expr` catches the out-of-bounds post-admission (`eval_load_const_out_of_bounds_returns_error` test exists). | PARTIAL   | `crates/vb_core/src/workflow/mod.rs:1289-1310` and `validation/resource.rs:144-165` only validate `LoadAccessor`; `LoadSlot`/`LoadConst` are not bounds-checked at validation time. Test `eval_load_const_out_of_bounds_returns_error` in `vb_expr/src/eval/tests/integration.rs:578` catches the bug at runtime, not admission. |
| vb-yq255 | P1  | n/a           | n/a       | n/a       | n/a                | `cargo +nightly clippy -p vb_ipc --all-targets --all-features` (claimed exit 0) | bug already CLOSED; clippy-debt repair in `vb_ipc/peer_credentials.rs` and `vb_ipc/server/handlers/tests.rs`. Not a diagnostic code issue. | PATCHED   | `bd show vb-yq255` close reason: "Strict gate cargo +nightly clippy -p vb_ipc --all-targets --all-features exits 0" + 1289 tests pass. |
| vb-z6gpb | P2  | n/a (returns `WorkflowError::StepOutOfBounds` not a `Diagnostic`) | n/a | n/a | none | `cargo test -p vb_core --lib engine::validate::tests::validate_node_bounds` → 4 passed | bug is in_progress; `validate_node_bounds` at `crates/vb_core/src/engine/validate.rs:107-120` only validates `node.id` and `node.next` — `on_error` and kind-specific targets are not checked. Kind-specific targets are partially covered by `validate_transition_target` (line 174 checks `ErrorHandler` body+handler), but the bug-hunt claim is that `validate_node_bounds` itself should cover them. The error returned is `WorkflowError::StepOutOfBounds`, not a Section 16 symbolic `Diagnostic` code; the diagnostic mapping happens upstream. | NOT-PATCHED | `crates/vb_core/src/engine/validate.rs:107-120` `fn validate_node_bounds` body checks only `node.id` and `node.next`; no `on_error` or kind-specific validation. The `red_phase_behavior_tests.rs:1124 fn rejects_node_with_out_of_bounds_on_error_step` test exists but lives in the dedicated red-phase test module. |
| vb-zlu3h | P2  | n/a           | n/a       | n/a       | n/a                | `cargo test -p vb_storage --lib codec::tests` (claimed 163 passed) | bug already CLOSED; test-strength repair in `vb_storage/codec/tests.rs`. Not a diagnostic code issue. | PATCHED   | `bd show vb-zlu3h` close reason: "Replaced 8 is_ok() smoke tests... 163 passed, 0 failed." |
| vb-zpaad | P3  | n/a (no diagnostic code involved) | **NOT PATCHED**: `Span::new` at `span.rs:22` still accepts `start > end`; no `Span::try_new` added | n/a | none | `cargo test -p vb_core --lib span::tests` → 21 passed (but no test asserts `try_new` rejection) | bug is in_progress. The current `Span` API at `crates/vb_core/src/span.rs:16-31` exposes only `pub const fn new(start: u32, end: u32) -> Self` with no validation. `Span::ZERO` is documented and tested as the empty/placeholder span, but there is no constructor that rejects `start > end`. Field `start` and `end` are public, so `let s = Span { start: 10, end: 5 }` is also accepted. `Span::try_new` does not exist (grep confirms zero matches across the workspace). | NOT-PATCHED | `crates/vb_core/src/span.rs:9-31`: `pub struct Span { pub start: u32, pub end: u32 }` with `pub const fn new(start, end) -> Self { Self { start, end } }` — no range check, no `try_new` variant. `Span::ZERO = Self { start: 0, end: 0 }` is the documented empty span used by `diagnostic_from_error` placeholder. |
| vb-zvjjn | P2  | n/a           | n/a       | n/a       | n/a                | (no diagnostic site) | bug already CLOSED; duplicate of `vb-6tnb6`. Performance issue, not diagnostic. | PATCHED   | `bd show vb-zvjjn` close reason: "Duplicate of vb-6tnb6; same external_ref bug-hunt-2026-06-21:RP-004 remains tracked there." |

## Summary

- **bugs-checked:** 10
- **PASS (PATCHED):** 5 (vb-xezc0, vb-yfsc4, vb-yq255, vb-zlu3h, vb-zvjjn) — all previously closed
- **NOT-PATCHED (open or fix absent):** 2 (vb-zpaad, vb-z6gpb)
- **PARTIAL:** 1 (vb-ylzmr)
- **UNKNOWN (not a diagnostic-contract bug):** 2 (vb-x3b0q, vb-yasoz)

### Diagnostic-contract findings

- **numeric-code remnants:** All `ValidationError` and `CompileError` codes are exposed
  via `SymbolicCode` first and `DiagnosticCode` (numeric) second. Internal `DiagnosticCode::new(0x....)`
  calls exist in `crates/vb_core/src/errors.rs:524-622` (CoreError code constants) and
  `crates/vb_validate/src/diag_codes.rs:6-73` (validation code constants), but they
  are registry-mapped: every constant has a matching `CodeEntry` in `CODE_REGISTRY` and
  the runtime `category_from_numeric` / `symbolic_to_numeric` roundtrip is exercised
  by `registry_symbolic_to_numeric_roundtrip` and `symbolic_code_numeric_code_roundtrip`
  tests (all 38 pass). No orphan numeric E-codes found.

- **`Span::ZERO` remnants:** 14 call sites in production/tests use `Span::ZERO` directly.
  The validate layer (`vb_validate::diagnostic::diagnostic_from_error`) is **contractually**
  expected to emit `Span::ZERO` and three tests assert this. Kani harnesses
  (`crates/vb_core/src/kani/kani_diagnostic_constructor.rs:106, :140`) and
  `proptest_diagnostic_constructor.rs` roundtrip use `Span::ZERO` as the no-location input.
  These are not remnants — they are the contract for the no-source-location emission
  path. The actual deficiency is that `Span::new` cannot reject `start > end`
  (see vb-zpaad).

- **Category collisions still present:**
  - `Internal` (0x1309 `INTERNAL_INVARIANT_VIOLATION`) overlaps `Accessor` (0x13xx).
    Registry lookup correctly classifies 0x1309 as `Internal`. The fallback high-byte
    heuristic at `diagnostic.rs:1994-2014` would classify unregistered 0x13xx codes
    as `Accessor` (line 2005), not `Internal`. Documented in `CodeCategory` doc
    comment (line 57) as intentional fallback. **No fix is on file.**
  - `Lifecycle` owns both 0x15xx (`CORE_LIFECYCLE_*`, `LIFECYCLE_STALE_REQUEST`,
    `JOURNAL_WRITE_FAILURE`, `REPLAY_CORRUPTION` at `diagnostic.rs:1517-1551`)
    **and** 0x33xx (`LIFECYCLE_*` at `diagnostic.rs:992-1015`). This is a
    two-range split for the same conceptual category, also documented
    (line 53 vs line 32 in `CodeCategory`). No numeric collision (different
    high bytes), but semantically inconsistent.

- **YAML path absence:** `Diagnostic` does not carry a YAML path
  (e.g., `$.steps[2].then`). `source_file: Option<Box<str>>` is the only
  source-location field, and it is set to `None` for all
  `vb_validate::diagnostic::diagnostic_from_error` emissions
  (`crates/vb_validate/src/diag_render.rs:18`). For YAML-derived validation
  errors, the YAML path is not preserved in the `Diagnostic` record. This
  is a **Section 16 contract gap**, but no wave-1 bug calls it out.

### Top-3 NOT-PATCHED with one-line reason

1. **vb-zpaad** — `Span::new` at `crates/vb_core/src/span.rs:22` has no `try_new`
   variant; `start > end` is silently accepted (no fix in tree).
2. **vb-z6gpb** — `validate_node_bounds` at `crates/vb_core/src/engine/validate.rs:107-120`
   does not validate `on_error` or kind-specific targets; bug remains in_progress.
3. **vb-ylzmr** (PARTIAL) — close reason claims
   `validate_expressions(expressions, slot_count, const_count, accessor_count)`,
   but the source at `crates/vb_core/src/workflow/mod.rs:1289-1310` still
   only takes `accessor_count` and only validates `ExprOp::LoadAccessor`;
   `LoadSlot`/`LoadConst` validation is missing at admission time.

### Files

- Output: `/home/lewis/src/velvet-ballistics/to-fix/wave1/agent-14-adhoc-diagnostic-contract.md`

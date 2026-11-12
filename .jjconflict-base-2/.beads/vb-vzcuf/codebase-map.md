# Codebase Map: vb-vzcuf State 2 Explore

## Scope
- Bead: `vb-vzcuf` — fresh replacement for capped `vb-8mdp.4`.
- Target seam: accumulated journal batch-byte accounting, storage-visible typed accumulated-budget error/API, Rust-bound Verus/Flux/Kani proof seams, and prior capped evidence.
- Isolated workspace: `/home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-vzcuf`.
- Prior evidence context only: `/home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.4`.

## Production Files and Symbols

### Primary storage seam
- `crates/vb_storage/src/batch.rs`
  - `JournalWriteBatch<'j>` fields at lines 38-44: `inner`, `journal`, `staged_event_keys`, `aborted`, `_not_send_or_sync`.
  - `JournalWriteBatch::new` lines 49-57 initializes no byte accumulator or byte limit.
  - `JournalWriteBatch::append_event` lines 209-229:
    - builds key using `run_event_key(event.run_id(), event.seq())`;
    - rejects durable duplicates with `JournalError::DuplicateEvent` and sets `aborted = true`;
    - rejects when `self.inner.len() >= MAX_BATCH_COUNT` with `JournalError::QueueFull`;
    - calls `encode_record(... MAX_JOURNAL_EVENT_PAYLOAD_BYTES ...)` for per-record payload limit;
    - inserts encoded record into `self.inner`.
  - Current observed gap: no accumulated encoded byte total, no batch byte limit, no checked accumulated-byte arithmetic, no typed accumulated-budget `JournalError` branch, and no public accessor for staged journal-event bytes.
  - `len`/`is_empty` lines 232-242 report operation count and special-case `aborted`; they do not expose byte accounting.
  - `commit` lines 250-256 commits all staged operations unless `aborted`; `QueueFull` currently does not abort.

### Storage constants and encoding
- `crates/vb_storage/src/constants.rs`
  - `MAX_JOURNAL_EVENT_PAYLOAD_BYTES: u32 = 1_048_576` line 78 is the single-record payload cap.
  - `MAX_BATCH_COUNT: usize = 10_000` line 90 is the count cap.
  - No accumulated journal batch-byte cap constant found in this file.
- `crates/vb_storage/src/codec/mod.rs`
  - `encode_record` lines 21-32 postcard-serializes payload, validates payload length via `payload_len_u32`, and returns full envelope bytes.
  - Important distinction: `PayloadTooLarge { len, max }` here is per-record payload pressure, not accumulated batch pressure.
- `crates/vb_storage/src/error/mod.rs`
  - `JournalError::QueueFull` lines 44-46 is existing count/queue-full error.
  - `JournalError::PayloadTooLarge { len, max }` lines 109-116 is existing single-record payload error.
  - No `JournalBatchBytesExceeded`, `BudgetExceeded`, or equivalent storage-visible accumulated byte error variant found in `JournalError`.

### Core budget types adjacent but not storage admission
- `crates/vb_core/src/workflow/mod.rs`
  - `ResourceContract::max_journal_batch_bytes: u32` lines 222-225.
  - default `max_journal_batch_bytes: 1_048_576` lines 248-249.
  - `budget_error_detail` maps `BudgetError::JournalBatchBytesExceeded` to `max_journal_batch_bytes` lines 773-787.
- `crates/vb_core/src/validation/resource.rs`
  - `validate_resource_contract` checks `max_journal_batch_bytes` is non-zero and within `MAX_JOURNAL_BATCH_BYTES` at lines 19-34.
- `crates/vb_core/src/budget.rs`
  - `WholeWorkflowBudget::max_journal_batch_bytes` line 50 and copied from `ResourceContract` at lines 158 and 226.
  - `BoundednessPolicy::absolute_max_journal_batch_bytes` lines 349-350; default 1_048_576 line 375.
  - `validate_payload_budget` checks computed budget against policy lines 463-491.
  - `validate_u32_budget("journal", ...)` returns `BudgetError::JournalBatchBytesExceeded { actual, limit }` lines 494-500.
  - `BudgetError::JournalBatchBytesExceeded { actual: u32, limit: u32 }` lines 519-543.
  - Current limitation: this is core policy validation, not a storage `JournalWriteBatch::append_event` admission outcome.

## Existing Tests
- `crates/workspace_tests/tests/journal_batch_accounting_tests.rs`
  - Header claims B01/B03 `BudgetExceeded` coverage, but lines 48-51 explicitly state storage does not enforce byte limits directly and documents only core-layer `BudgetError` construction.
  - `batch_has_no_byte_limit_enforcement_at_storage_layer` lines 53-65 asserts `append_event` succeeds regardless of byte budget.
  - Count boundary tests lines 71-135 assert `JournalError::QueueFull` at `MAX_BATCH_COUNT`.
  - `budget_error_journal_batch_bytes_exceeded_exact_construction` lines 144-165 constructs `vb_core::budget::BudgetError::JournalBatchBytesExceeded` only.
  - No behavior test found for storage accumulated byte rejection, exact storage error fields, or no-mutation on accumulated byte overflow.
  - If storage byte enforcement becomes required, this test file contains outdated/contradictory assertions and should be updated by test/implementation lanes.
- `crates/workspace_tests/tests/journal_side_index_contracts.rs`
  - Exercises `JournalWriteBatch::append_event` atomicity and duplicate behavior; relevant for no-partial-mutation regression coverage around `append_event`.
- Other workspace tests reference `max_journal_batch_bytes` as workflow/resource policy input, e.g. `vb_qi37_2_4_integration_budget_errors.rs`, `vb_lp2v_admission_integration.rs`, and `vb_test_core_yaml_chain_behavior.rs`; these are not storage accumulated-byte admission tests.

## Existing Proof / Verification Inventory
- Verus directory exists: `verification/verus/`.
  - Relevant existing storage proofs include `vb_jnz9_journal_event_seq_valid.rs`, `vb_jpq724_events_for_run_production.rs`, and replay/normalization specs; none are named `vb_vzcuf` or `vb_8mdp_4`.
- Flux directory exists: `verification/flux/` with only `choose_refinements.flux`, `vb_compile/`, `vb_rpch_flux_r8.rs`, `vb_rpch_flux_r9.rs`, `vb_xi2f_compile_source.rs`, and `vb_xi2f_try_from_parts.rs`; no batch-byte Flux seam found.
- `crates/vb_storage/src/verification` is MISSING in the fresh workspace; no crate-wired Kani proof module path for storage batch-byte accounting found.
- `scripts/verify-verus.sh` registers Verus targets from `contracts/proof_obligations.yaml` and fails if required files are missing or trust scan finds assumptions.
- `scripts/flux-check-package.sh <package>` is the supported Flux package command; it rejects unsupported `--lib`, `--test`, `--tests`, `--benches`, and `--all-targets` flags.
- `scripts/kani-list.sh <package>` is the required Kani inventory command; use package-scoped runs, not root `cargo kani list`.

## Prior Capped Evidence Context
- Prior path: `/home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.4`.
- Prior proof-review evidence states `JournalWriteBatch::append_event` lacked accumulated byte total accounting, batch byte limit, and accumulated-budget typed error branch (`.beads/vb-8mdp.4/proof-review.md:17-23`).
- Prior proof-evidence records PO-009..PO-015 and PO-031..PO-037 blocked by absent production accumulated byte accounting/error API, with all planned `vb_8mdp_4_*` Verus/Flux/Kani Rust-bound artifacts missing (`.beads/vb-8mdp.4/proof-evidence.md:78-86`, `281-303`, `519-599`).
- Prior contract artifacts identify direct downstream obligations:
  - `proof-to-implementation-input.md`: byte equality/overrun/overflow semantics, typed byte-budget error, C6 error separation, tests and commands.
  - `proof-coverage-matrix.md`: C3/C4/C6/H7 obligations and files.
  - `error-taxonomy.md`: count, per-record payload, and accumulated batch-byte domains must be distinct.
  - `boundary-map.md`: no current public `can_fit_by_bytes` or `batch_bytes()` API; `StorageLimits`/`BatchBuilder` do not satisfy batch byte claims.

## Required Downstream Contract/Implementation Questions
1. What storage-visible API carries the batch byte limit into `JournalWriteBatch`? Candidate possibilities are constructor parameter, storage limits/policy object, or explicit helper invoked by runtime; UNKNOWN in current code.
2. What exact typed storage error should represent accumulated batch bytes exceeded? Current `JournalError` lacks a variant; core `BudgetError::JournalBatchBytesExceeded` exists but is not returned by `append_event`.
3. Should accumulated byte total include only journal event encoded values or all `OwnedWriteBatch` operations? Prior evidence says journal batch-byte accounting; current `self.inner.len()` counts all keyspace operations.
4. Guard precedence must be specified and tested: durable duplicate, count limit, per-record payload limit, accumulated byte limit. C6 requires controlled unrelated guards for exact error separation.
5. Need no-mutation semantics for accumulated-budget rejection: staged count/bytes unchanged, rejected key absent after commit, and no abort unless contract explicitly says otherwise.

## Risk Tags
- persistence: `JournalWriteBatch::commit` writes durable Fjall batch; wrong admission can persist oversized batch.
- public-api: `JournalError` and `JournalWriteBatch` API likely need a typed budget seam.
- performance: byte accounting should avoid double expensive serialization if possible; any performance claim needs evidence.
- proof: Verus/Flux/Kani artifacts currently missing for this seam; proofs must bind to production helpers, not copied verifier-only classifiers.
- arithmetic: accumulated byte total and limit comparisons require checked add and bounded u32/usize/u64 semantics.
- test-drift: existing `journal_batch_accounting_tests.rs` documents absence of storage byte enforcement and will conflict with new required behavior unless updated.

## State 2 Validator Evidence
- Pre-artifact validator command run from isolated workspace:
  - `/home/lewis/.agents/skills/go-skill/tools/go-skill-v9-validate --workspace "/home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-vzcuf" --bead vb-vzcuf --state 2 --source-checkout "/home/lewis/src/velvet-ballistics" --skill-root "/home/lewis/.agents/skills/go-skill" --mirror-root "/home/lewis/.opencode/skill/go-skill"`
  - Raw output: `STATUS: FAIL` / `E_MISSING_ARTIFACT codebase-map.md - required artifact missing or empty`.
- Post-artifact validator should be rerun after this map and JSONL scope are written.

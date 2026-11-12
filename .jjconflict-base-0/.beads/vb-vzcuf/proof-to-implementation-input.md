# Proof-to-Implementation Input: vb-vzcuf

## Production Targets to Bind

- `crates/vb_storage/src/batch.rs`
  - `JournalWriteBatch::new`
  - explicit limited constructor/factory chosen by implementation
  - `JournalWriteBatch::append_event`
  - staged byte accessor, e.g. `batch_journal_event_bytes()`
  - extracted pure admission helper, e.g. `admit_journal_event_bytes(staged, encoded_len, limit)`
- `crates/vb_storage/src/error/mod.rs`
  - accumulated budget `JournalError` variant with attempted/limit or equivalent fields
- `crates/vb_storage/src/codec/mod.rs`
  - `encode_record` returned `Vec<u8>.len()` consumed by storage accounting
- `crates/vb_core/src/workflow/mod.rs` and `crates/vb_core/src/budget.rs`
  - core policy bridge or explicit documented storage separation

## Proof Claims Requiring Implementation Mapping

1. The byte admission helper accepts exact fits and rejects overflow or over-limit attempts without unchecked arithmetic.
2. `append_event` calls `encode_record` once for the candidate and accounts using the full encoded value length returned by that call.
3. The accumulated-budget error is distinct from `QueueFull` and `PayloadTooLarge`, and guard precedence makes those errors observable under controlled preconditions.
4. Accumulated-byte rejection is non-aborting and leaves `inner`, same-batch key tracking, staged bytes, and later commit contents unchanged.
5. Every open batch carries a non-zero `JournalBatchByteLimit`, including the default constructor path.
6. Core `max_journal_batch_bytes` is bridged into the storage limit or storage separation is explicit and tested.
7. Same-batch duplicate accounting follows the documented policy. Current planning bound is conservative attempt accounting.

## Required Downstream Evidence Commands

- `verus --crate-type=lib verification/verus/vb_vzcuf_byte_admission.rs`
- `verus --crate-type=lib verification/verus/vb_vzcuf_append_decision.rs`
- `bash scripts/flux-check-package.sh vb_storage`
- `bash scripts/kani-list.sh vb_storage`
- `cargo kani -p vb_storage --features kani-vb-vzcuf --harness <vb_vzcuf_harness_name>`
- `cargo test -p workspace_tests --test journal_batch_accounting_tests vb_vzcuf -- --nocapture`
- `cargo fuzz run vb_vzcuf_journal_event_encoding -- -max_total_time=60` for the codec defense-depth lane if cargo-fuzz target is accepted by proof-plan review.

## Bridge Risks

- Verifier-only duplicate logic is invalid unless the bridge maps it to the production same-batch key/`OwnedWriteBatch` behavior.
- A package-level Flux pass is only a smoke check unless `vb_vzcuf` refinements are crate-wired or explicitly checked.
- Kani harnesses must generate arbitrary bounded structures; one hardcoded journal event shape is rejected by repository verification rules.

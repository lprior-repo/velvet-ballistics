# Trusted Base Plan — vb-0x1cb

- bead_id: vb-0x1cb
- state: 4 (proof-planner)
- lane_profile: rust_local_concurrency_empty
- captured_at: 2026-07-01T16:05:00Z

## Plan status

Trusted base entries are planned for State 5 (proof-writer). Each row declares
either an external_body marker (unavoidable abstraction), an `assume` (modeling
compromise), a stub (test-bench boundary), or an `extern_spec` (Flux binding
glue). No entry waives a behavior-affecting requirement. Entries are review
debt and do NOT constitute proof closure; proof-reviewer (state 4b) and
formal-verifier (state 12) own disposition.

## Trusted base ledger (planned)

| ID | Obligations | Artifact | Marker | Kind | Reason | Compensating Evidence |
|----|-------------|----------|--------|------|--------|------------------------|
| TBR-vb-0x1cb-001 | PO-001, PO-002 | `crates/vb_runtime/src/shard/tests/proptest_*.rs` | `Arbitrary` impl for `RunId` + journal rejection condition | external_body | proptest needs `Arbitrary` for `RunId` and for the journal-rejection `bool` flag to drive the 2×2 matrix; production `RunId::new` validates `> 0` and the bool is a probe flag. | Strategy uses `proptest::prop_compose!` over `(bool, bool)` plus the production `RunId::new` constructor validated by `vb_core::ids::tests`; the journal-rejection flag is constructed via `Arc<RwLock<…>>`-style indicator on the test journal stub. |
| TBR-vb-0x1cb-002 | PO-001, PO-002 | `crates/vb_runtime/src/shard/tests/proptest_*.rs` | `pub(crate)` access to `Shard::observe_run_state_rollback` and to `Shard::trace_ring` | extern_spec | proptest invokes the helper directly, then observes `trace_ring`. Production `pub(crate)` access lets proptest bypass the public `Shard::tick` envelope. | Production visibility is gated on the crate boundary (`pub(crate)`); outside the crate the helper remains inaccessible. Behavior test PO-003/PO-004 use the public tick envelope and the same witness. |
| TBR-vb-0x1cb-003 | PO-003, PO-004 | `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs, chunk_008.rs` | `SharedRuntimeJournal = std::sync::Arc<dyn RuntimeJournal>` test stub rejecting specific event variants | stub | Rejecting-stub replaces the production `FjallJournal` so behavior tests can deterministically induce `StorageJournalAppend(WriteLockPoisoned)` for `RunFinished`/`RunFailed` only. Mirrors `LegacyStepFailsJournal` pattern in `chunk_004.rs:236-339`. | Production behavior test for legacy `StepSucceeded` is existing (`chunk_004.rs:236-339`); the new tests mirror it; no production `FjallJournal` is exercised in either test. |
| TBR-vb-0x1cb-004 | PO-005 | `crates/vb_runtime/src/verification/flux/vb_0x1cb_run_rollback_failed_size_bound.rs` | `extern_spec` mirror for `TraceEvent` and `RollbackSite` enums | extern_spec | Flux requires `extern_spec` mirrors for runtime types so the refinement check ties back to production via the same name resolution path used by the existing `vb_y9d3v_action_ticket_refinements.rs`. | The mirror uses the canonical extern_spec pattern with `unimplemented!("extern spec — production body in trace/event.rs")`; production body is unchanged. |
| TBR-vb-0x1cb-005 | PO-005 | `crates/vb_runtime/src/verification/flux/vb_0x1cb_run_rollback_failed_size_bound.rs` | Flux nightly toolchain availability | assume | Flux depends on a nightly toolchain (rust-toolchain.toml pins this). | `rust-toolchain.toml` pins the nightly; `cargo flux -p vb_runtime --message-format human` exits 0 post-edit; the existing `vb_y9d3v_action_ticket_refinements.rs` proves the toolchain is on PATH. |
| TBR-vb-0x1cb-006 | PO-005 | `crates/vb_runtime/src/trace/event.rs` | `std::sync::Arc<RuntimeError>` allocation cost | assume | Two `Arc<RuntimeError>` allocations per dual-failure trace event are bounded, well below one cache line; no further refinement. | `PO-005` Flux extern_spec carries the size bound ≤ 25 bytes; behavior tests PO-003/PO-004 verify construction; no runtime allocation pressure concern under disk-pinned `trace_ring`. |
| TBR-vb-0x1cb-007 | PO-007 | `scripts/check-ignored-fallible-results.sh` | `bash` interpreter (≥5.x) and `rg` (ripgrep) | assume | The script depends on `bash ≥5.x` and `rg --files` for scanning. | The script is already wired into `moon :lint-src` and passes in CI today (per `to-fix/wave4/agent-12-adhoc-kani-harness.md`); same exec context is reused. |
| TBR-vb-0x1cb-008 | PO-006 | `crates/vb_runtime/src/shard/transitions.rs` | `observe_run_state_rollback` declared `#[must_use]` and has `pub(crate) fn` visibility | extern_spec | `#[must_use]` is enforced via `unused_must_use` lint promoted to `deny` via `cargo clippy -D clippy::let_underscore_must_use`; the helper's call sites (transitions.rs:100 and :202) MUST use the result, either via `?`-on-not-applicable paths OR via explicit `match` arms. | Compiletime enforcement: dropping the helper return value is a compile error. PO-006 captures the clippy slice. |

## Trusted base summary

- **Total entries**: 8
- **External bodies**: 1 (TBR-001)
- **Assumes**: 3 (TBR-005, TBR-006, TBR-007)
- **Stubs**: 1 (TBR-003)
- **Extern specs**: 3 (TBR-002, TBR-004, TBR-008)
- **Behavior-affecting**: 0 (all are modeling/proof debt)
- **Review state**: planned (owner_state 5)

## Validation plan

At State 6 (proof-reviewer) each entry must be:

1. Cited by proof-writer in the actual proof artifacts (proptest file, Flux extern_spec file, behavior test).
2. Reviewed for soundness — does the assumption hold in production?
3. Compensated by independent evidence (existing tests, behavior mirrors, clippy discharge).
4. Marked `reviewer_disposition: accepted` or `rejected` with findings.

No trusted base entry waives behavior-affecting requirements. This file is
not a waiver catalog; `waiver-candidates.jsonl` enumerates the (empty) waiver
state separately.

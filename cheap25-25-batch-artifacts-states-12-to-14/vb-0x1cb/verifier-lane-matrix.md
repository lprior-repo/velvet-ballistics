# Verifier Lane Matrix — vb-0x1cb

- bead_id: vb-0x1cb
- state: 4 (proof-planner)
- lane_profile: rust_local_concurrency_empty
- captured_at: 2026-07-01T16:05:00Z
- owner: proof-planner

## 1. Default-profile lane set

Rust default-profile lanes (per proof-planner SKILL §"Default profile"):
**kani, verus, flux-rs, proptest**.

Auxiliary lanes (per bead instruction): **cargo-clippy, moon-source-gate,
bash scripts/check-ignored-fallible-results.sh** (canonical evidence for
moon-source-gate).

Non-engaged lanes with explicit reason rows: **loom, miri, cargo-fuzz**.
TLA+ is globally removed across the repo and is NOT a lane row.

## 2. Seed × verifier applicability matrix

| Proof Seed | Verus | Kani | Flux-rs | Proptest | Loom | Miri | Cargo-fuzz |
|------------|-------|------|---------|----------|------|------|------------|
| proof-seed-vb-0x1cb-S1 (secondary bound, C-2) | not_applicable | not_applicable | not_applicable | required | not_applicable | not_applicable | not_applicable |
| proof-seed-vb-0x1cb-S2 (primary-wins, C-1) | not_applicable | not_applicable | not_applicable | required | not_applicable | not_applicable | not_applicable |
| proof-seed-vb-0x1cb-S3 (bounded-payload, C-3) | not_applicable | not_applicable | required | not_applicable | not_applicable | not_applicable | not_applicable |
| proof-seed-vb-0x1cb-S4 (#[must_use] helper, C-3) | not_applicable | not_applicable | not_applicable | required | not_applicable | not_applicable | not_applicable |
| proof-seed-vb-0x1cb-S5 (trace-ring-count, C-1, C-2) | not_applicable | not_applicable | not_applicable | required | not_applicable | not_applicable | not_applicable |
| proof-seed-vb-0x1cb-S6 (source-gate clean, C-5) | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable |
| proof-seed-vb-0x1cb-S7 (annotation removal, C-4, C-5) | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable | not_applicable |

S6 and S7 are release-blocker source-gate obligations; their coverage is
encoded as `moon-source-gate` + `bash scripts/check-ignored-fallible-results.sh`
verifier rows in `verifier-lane-decisions.jsonl` (the proof-planner skill
permits verifier values outside the default set when the bead profile
demands it; this is the Rust-y analogue of the special-gate row pattern
used for `code-review` in prior beads).

## 3. Applicability legend

- **required**: Mandatory verifier lane for the seed's risk profile.
- **not_applicable**: Lane is intentionally not engaged for the seed. Each
  row MUST cite a concrete artifact hash, source location, or stub boundary
  that closes the gap; reviewer will reject any silent omission.
- **blocked_tooling**: Tool unavailable; only used if a required lane lacks
  the verifier binary — none present in this bead's workdir.

## 4. Non-applicability evidence summary

| Lane | Reason (short) | Evidence Ref |
|------|----------------|--------------|
| loom (all seeds S1..S7) | Single-threaded `Shard::tick`; `JournalWriteBatch` is `!Send + !Sync`; no concurrent interleaving across rollback paths. Sequential `proptest` explores the dual-failure matrix without scheduler exploration. | `crates/vb_runtime/src/runtime.rs:198` (`tick_all` processes one shard per tick); `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:206` (kani stub branch); `Shard::tick` drains one `ShardCommand` atomically. |
| miri (all seeds) | `#![forbid(unsafe_code)]` in every scoped file; zero unsafe, no FFI, no raw pointers, no `MaybeUninit`. | `crates/vb_runtime/src/shard/transitions.rs:1`, `crates/vb_runtime/src/trace/event.rs:1`, `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs:1`, `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs:1`. |
| cargo-fuzz (all seeds) | No codec, parser, byte-level hostile input added in this bead; `TraceEvent::RunRollbackFailed` carries typed `RunId`, `RollbackSite`, and `Arc<RuntimeError>` only. | `crates/vb_runtime/src/trace/event.rs` (event enum is `#[non_exhaustive]` and never read from disk). |
| verus (S1..S7) | Bead instruction omits Verus; `rust-contract` decision surface deliberately skips Verus; Flux carries bounded-payload refinement S3, proptest carries behavior S1/S2/S4/S5, cargo-test + source-gate carry C-4/C-5/C-6/C-7. | `contract.md` C-7 (`lane_profile: rust_local_concurrency_empty`) + bead task description: "Lanes: rust-local, flux-rs (action_ticket_refinements), proptest, kani stub, cargo-clippy, moon-source-gate, bash scripts/check-ignored-fallible-results.sh". |
| kani (S1) | `#[cfg(kani)]` stub for `append_journal_event` returns `Ok(())`; the rollback error path (`Err(error)` from journal) is unreachable under `cargo kani`. The dual-failure obligation is satisfied by proptest PO-001/PO-002 directly. | `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:206-217` (`#[cfg(kani)] pub(crate) fn append_journal_event(...) { Ok(()) }` stub); existing `kani_ask_answer_lifecycle.rs:80-100` proves the stub branch in isolation. The stub MUST remain unchanged. |
| kani (S2) | Same kani stub boundary; primary-wins assertion requires the journal to reject, which the stub prevents. Proptest PO-001/PO-002 cover the branch by feeding the helper directly. | Same ref as above. |
| kani (S3) | Bounded-payload is a compile-time `size_of` invariant over the `TraceEvent` enum. A kani harness with `kani::any()` populating the variant degenerates to a structural check; the Flux `extern_spec` (`PO-005`) covers the same bound in a strictly stronger form. Adding kani lane row would duplicate evidence with the same value. | `crates/vb_runtime/src/verification/flux/vb_y9d3v_action_ticket_refinements.rs:27-34` (existing extern_spec pattern that already proves size invariants via `#[refined_by]`); PO-005 mirrors this pattern. |
| kani (S4) | The `#[must_use]` attribute and exhaustive `match` are language-level enforcement at compile time; clippy `unused_must_use` flags any return-value drop. Kani would add no value beyond the language-level guarantee. | `Cargo.toml` lint configuration for `unused_must_use` is `warn` by default; PO-006 promotes it to `deny` for the scope of `vb_runtime`. |
| kani (S5) | Same as S1 — the kani stub returns `Ok(()`, so neither rejection nor rollback is reachable under kani. The trace-ring-count assertion requires the dual-failure path. | Same ref as S1. |
| flux-rs (S1, S2, S4, S5) | These seeds are behavior-affecting, not refinement. Flux's refinement type system cannot faithfully model `Result<Option<RunState>, RuntimeError>` event counts in a way stronger than runtime tests. The bounded-payload is the one place refinement adds evidence; that is encoded in PO-005. | `crates/vb_runtime/src/verification/flux/vb_y9d3v_action_ticket_refinements.rs` (existing refinement file does not cover event-count semantics). |
| proptest (S3) | Bounded-payload is a type-level invariant; runtime generation does not exercise the size bound differently than `std::mem::size_of`. Flux `extern_spec` PO-005 is the canonical proof. | Same ref as S3 kani. |
| proptest (S6, S7) | Source-gate is a static check on `.rs` text and the allow ledger. Proptest does not operate on text or exit codes. | `scripts/check-ignored-fallible-results.sh:151-238` (`scan_tree` function emits `JustifiedException|…` and `ViolationFound|…` lines per-file/per-line). |

## 5. Auxiliary gate rows (bead instruction)

These rows encode the `cargo-clippy`, `moon-source-gate`, and
`scripts/check-ignored-fallible-results.sh` obligations. They sit outside
the default Rust profile verifiers but are required by the bead instruction
(verb "Lanes: rust-local, flux-rs …, cargo-clippy, moon-source-gate, bash scripts/check-ignored-fallible-results.sh").
Proof-reviewer dispositions them alongside the default lanes.

| Special lane | Required for | Encoding in `verifier-lane-decisions.jsonl` |
|--------------|--------------|---------------------------------------------|
| cargo-clippy | S7 (annotation removal) | `verifier: cargo-clippy` row |
| moon-source-gate | S6 (allow row removed + script exit 0) | `verifier: moon-source-gate` row |
| bash `scripts/check-ignored-fallible-results.sh` | S6, S7 (canonical evidence) | `verifier: bash-source-gate` row; evidence_command quoted verbatim |

## 6. Required → non-applicable counts

| Verifier | Required | Not Applicable | Total |
|----------|----------|----------------|-------|
| kani | 0 | 5 (S1, S2, S3, S4, S5) | 5 |
| verus | 0 | 7 (S1..S7) | 7 |
| flux-rs | 1 (S3) | 4 (S1, S2, S4, S5) + 2 outside default (S6, S7) | 7 |
| proptest | 4 (S1, S2, S4, S5) | 1 (S3) + 2 outside default (S6, S7) | 7 |
| loom | 0 | 7 (S1..S7) | 7 |
| miri | 0 | 7 (S1..S7) | 7 |
| cargo-fuzz | 0 | 7 (S1..S7) | 7 |
| cargo-clippy | 1 (S7) | 0 | 1 |
| moon-source-gate | 1 (S6) | 0 | 1 |
| bash-source-gate | 2 (S6 + S7 evidence) | 0 | 2 |
| **Total** | **9 required decisions** | **40 not_applicable** | **49 default-profile + 4 auxiliary = 53 rows** |

(PROOF-PLAN-REVIEWER would also need to flag Verus N/A as a deliberate
contract-explicit omission; see §4.)

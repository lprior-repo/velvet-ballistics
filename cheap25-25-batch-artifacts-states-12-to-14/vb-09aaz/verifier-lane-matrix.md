# Verifier Lane Matrix — vb-09aaz

bead_id: vb-09aaz
state: 4 (proof-planner)
maps proof seeds to verifier lanes

## Lane Symbol Legend

- ✅ = required (active lane, obligation planned)
- ✅ existing = covered by existing harness / artifact, no new obligation
- 🔄 = update required (existing artifact must be modified)
- — = not_applicable (with evidence cited)

## Matrix

| Proof Seed | Description | verus (WEAK_EXTERN) | kani | flux-rs | proptest | persistence (integration) | rust-local (api-surface) | loom | miri | cargo-fuzz | tla-plus |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| vb-09aaz-PS-001 | G8 IndexKeyConstruction abort-on-Err (C1, C4) | 🔄 | — | — | ✅ | — | — | — | — | — | — |
| vb-09aaz-PS-002 | Regression test batch_append_event_index_key_error_aborts_commit (C8) | — | — | — | — | — | ✅ | — | — | — | — |
| vb-09aaz-PS-003 | Verus mirror regeneration with G8 guard (C7) | 🔄 | — | — | — | — | — | — | — | — | — |
| vb-09aaz-PS-004 | Doc-comment update for G8 (C9) | — | — | — | — | — | ✅ | — | — | — | — |
| vb-09aaz-PS-005 | Proptest variant with arbitrary triples (C8) | — | — | — | ✅ | — | — | — | — | — | — |
| vb-09aaz-PS-006 | Master §49 Crash-Consistency Rule (C4, C5) | — | — | — | — | ✅ | — | — | — | — | — |
| vb-09aaz-PS-007 | Public API surface stability (C6) | — | — | — | — | — | ✅ | — | — | — | — |
| vb-09aaz-PS-008 | Guard precedence (C2) — 8-guard order G1..G8 | 🔄 | — | — | — | — | — | — | — | — | — |

## Active Lanes Detail

### Verus (WEAK_EXTERN mirror update)

- **PS-001, PS-003, PS-008**: production-binding mechanism is WEAK_EXTERN via
  `#[path = "production_inner/vb_vzcuf_PS_008_production.rs"]` (mirror) and
  `#[path = "extern_vb_vzcuf_PS_008.rs"]` (extern). Analogous for PS-009.
- Mirror drift gate: `scripts/check-production-inner-drift.sh` (zero tolerance).
- Production-binding gate: `scripts/check-verus-production-binding.sh`
  (AGENTS.md mandatory).
- The fix requires regenerating both mirrors; the existing 7-guard enumeration
  at PS-008 L78-95 and PS-009 L67-93 becomes 8-guard.
- New exec arg `index_key_ok: bool` (mirrors `encode_ok: bool` precedent).
- New Err(KeyCapacity) match arm in `assume_specification` with witness
  `!index_key_ok` and post-condition `spec_state_preserved_except_aborted`.
- New exec wrapper `wrapper_append_event_index_key_error`.

### Proptest

- **PS-001, PS-005**: proptest with arbitrary `ActionId × RunId × StepIdx`
  triples. Asserts abort invariant under all inputs.
- Property: `KeyCapacity` (defensive) → `is_aborted() == true` and
  `commit() == Err(BatchAborted)`.
- Implementation: extend `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs`
  or add new file `crates/vb_storage/tests/proptest_vb_hyog0_PS_010.rs`.

### Persistence (integration)

- **PS-006**: end-to-end test using a real Fjall database instance. After
  G8 KeyCapacity fires and the batch aborts, no journal event is durable
  and no index_action mutation is durable.
- Asserts: `events_for_run(run).is_empty()` after the aborted batch's
  commit attempt.

### Rust-local (api-surface)

- **PS-002, PS-004, PS-007**: rust-local test that
  `JournalWriteBatch::append_event` signature, error variant surface, and
  accessor surface are unchanged. Plus doc-comment update verification
  (Guard Precedence enumerates G8; Postconditions documents KeyCapacity
  abort invariant).

## Non-Applicable Lanes

| Lane | Reason | Evidence |
| --- | --- | --- |
| **kani** | Existing Kani harness at `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs` covers G3 durable-duplicate abort. Adding a parallel G8 harness would require duplicating the `SpecJournalWriteBatch` mirror with the new `index_key_ok: bool` argument. The WEAK_EXTERN Verus mirror update is the stronger verification (binds to production via `assume_specification`) and supersedes a parallel Kani harness for this single-guard delta. | `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:1-30` existing harness scope + boundary-map.md#verifier-boundary |
| **flux-rs** | No refinement types in the batch layer. The Verus mirror's `assume_specification` already provides refinement-style post-conditions per guard. | `boundary-map.md#verifier-boundary` |
| **loom** | `JournalWriteBatch` is `!Send + !Sync` via `PhantomData<*mut FjallJournal>` (types.rs:18-21). Single-threaded; no concurrent memory ordering. | `boundary-map.md#async-concurrency-boundary` |
| **miri** | `#![forbid(unsafe_code)]` at append_event.rs:1 applies crate-wide. Zero unsafe blocks, zero FFI, zero raw pointers. | `boundary-map.md#unsafe-ffi-boundary` |
| **cargo-fuzz** | G8 fix is a 1-line replacement of `?` with `map_err`. No parser, no codec, no hostile byte boundary. Defensive `KeyCapacity` unreachable for nominal inputs. | `boundary-map.md#parser-codec-boundary` + `workflow-model.md#KeyCapacity-reachability` |
| **tla-plus** | Lane removed from this skill (TLA+ removed; temporal flows via loom + proptest). Single-threaded batch state machine; no concurrent state transitions. | skill SKILL.md — TLA+ removed |

## Coverage Summary

| Category | Total seeds | verus | proptest | persistence | rust-local |
| --- | --- | --- | --- | --- | --- |
| G8 abort invariant (PS-001) | 1 | 1 🔄 | 1 ✅ | — | — |
| Regression test (PS-002) | 1 | — | — | — | 1 ✅ |
| Verus mirror regen (PS-003) | 1 | 1 🔄 | — | — | — |
| Doc-comment (PS-004) | 1 | — | — | — | 1 ✅ |
| Proptest variant (PS-005) | 1 | — | 1 ✅ | — | — |
| Master §49 integration (PS-006) | 1 | — | — | 1 ✅ | — |
| API stability (PS-007) | 1 | — | — | — | 1 ✅ |
| Guard precedence (PS-008) | 1 | 1 🔄 | — | — | — |
| **Total** | **8** | **3 🔄** | **2 ✅** | **1 ✅** | **3 ✅** |

**Legend**: 🔄 = WEAK_EXTERN mirror update (regenerate); ✅ = new obligation planned.
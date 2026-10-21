**Findings:** None.

All four clippy blockers are addressed correctly with no behavior changes:

1. **recovery/full.rs**: Three unused crate-private getters (`input()`, `output()`, `action_abi_digest()`) removed; struct fields, `new()`, `ticket()`, and `event_seq()` preserved. Import reordering is cosmetic.

2. **shard/helpers.rs**: Nested `if let` collapsed to let-chain `if let Some(kind) = declared && !slot_value_matches_kind(value, kind)` — clippy-suggested equivalent, identical error path.

3. **shard/impl_parts/chunk_001.rs**: `.ok_or_else(|| RuntimeError::RunNotFound)` replaced with `.ok_or(RuntimeError::RunNotFound)` — equivalent for unit variant. Remaining changes are formatter line-wrapping.

4. **shard/lifecycle/chunk_001.rs**: Blank `///` separator added before the descriptive paragraph following the numbered list. Remaining changes are formatter line-wrapping.

No public API changes. No new getter calls added. No changes to `recovery/full/pending.rs` validation. Tests pass (2449 passed, clippy clean). The `moon run :lint-src` failure is on an unrelated existing `needless_borrow` (tracked vb-3dlcn), outside this patch's scope.

STATUS: APPROVED
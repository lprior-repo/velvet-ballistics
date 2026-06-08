# Round 4 Agent A7 — Section 38 Property Tests Gap (CRITICAL)

**Reviewer:** black-hat-reviewer · **STATUS: REJECT — DO NOT SHIP**

Five of eleven Section 38 property tests have ship-blocker coverage gaps, two of which (`concurrency_safety` and `bytecode_ast_parity`) are newly identified beyond Round 3's audit. The 1-line `property_tests.rs` stub and the lying "TEMPORARILY DISABLED" comment are discipline failures that must be remediated before any release claim is honest.

## The Lying Comment (`vb_compile/src/lib.rs:64-66`)

```rust
// TEMPORARILY DISABLED: pre-existing proptest macro compatibility issue in bytecode_ast_parity.rs
// #[cfg(test)]
// mod property_tests;
```

**Verdict: COMMENT IS A LIE.** The file `bytecode_ast_parity.rs` does not exist on disk. `find . -name 'bytecode_ast_parity*'` returns zero matches. The disabled declaration targets `mod property_tests` — not even the file named in the comment. This means the comment is not just outdated, it never matched reality.

## Per-Missing-Property Severity Matrix

| Property | Severity | Worst-Case Impact | Verdict |
|----------|----------|-------------------|---------|
| `concurrency_safety` | **95/100** | Race condition in `IntrospectionRegistry` Drop vs Register; silent data corruption | **SHIP-BLOCKER** |
| `bytecode_ast_parity` | **90/100** | Cold-path compiler and hot-path interpreter diverge silently | **SHIP-BLOCKER** |
| `taint_propagation` | **75/100** | Taint lattice escape on fuzz inputs; Section 47 violation | **SHIP-BLOCKER** |
| `layout_stability` | **70/100** | HashMap-iteration bug breaks replay determinism | **SHIP-BLOCKER** (replay) |
| `error_recovery` | **65/100** | Panic in `recovery::replay` on fuzz-malformed journal records | **SHIP-BLOCKER** (storage) |
| `digest_stability` | **60/100** | Digest changes between compilations; replay broken | ACCEPTABLE-AS-DEBT if combined coverage wired |
| `for_each_ordering` (proptest) | **55/100** | `for_each_next` mishandles non-i64 item types | ACCEPTABLE-AS-DEBT |
| `resource_budget` | **50/100** | Retry-attempt and time-limit enforcement untested | ACCEPTABLE-AS-DEBT |
| `bound_enforcement` | **45/100** | Runtime-time bound enforcement diverges | ACCEPTABLE-AS-DEBT |

## The 1-line Stub

`vb_storage/src/property_tests.rs`:
```rust
// Stub to allow compilation — property_tests directory contains individual test modules
```

It contains no `#[test]`, no `#[cfg(test)] mod proptest_*`, nothing. A grep for `proptest` returns zero.

## Detail: `concurrency_safety` SHIP-BLOCKER

What passes for "concurrency tests" today:
- (a) `vb_runtime/src/journal/tests/chunk_004.rs:702, 739` — 2 deterministic unit tests with 4 threads × 10 events
- (b) `vb_runtime/src/models/loom/*` — 6 Loom models on a SIMPLIFIED BoundedQueue with its own private `Arc<AtomicUsize>`
- (c) `vb_storage/src/kani_vb_mrwe_7_concurrency.rs` — a 12-line Kani harness
- (d) `vb_runtime/src/primitives/reentry_tests.rs:proptest_reentry` — 6 proptest functions but ZERO cross thread boundaries

Production code with concurrency:
- `vb_runtime/src/shard/introspection.rs:49, 85` — `Arc<Mutex<HashMap<RunId, u64>>>` for `IntrospectionRegistry`
- `vb_storage/src/journal/core.rs:66` — `FjallJournal::write_lock: Mutex<()>`

Worst case: **race condition in the production `IntrospectionRegistry` that only manifests under fuzz load.** Specifically: `InspectHandle::drop` acquires the mutex, then conditionally removes the entry. If thread A drops a handle and thread B re-registers the same RunId in the gap, A removes B's entry. The next Drop on the now-orphaned handle is a no-op, but B's registration is silently lost.

## Detail: `bytecode_ast_parity` SHIP-BLOCKER

- File exists? **NO.** Zero matches anywhere in repo.
- Why the test was supposed to exist: `compile_expr_to_bytecode` produces bytecode that the runtime then executes. The spec demands "Compiled bytecode produces same result as AST interpretation."
- The Kani harness at `vb_compile_bytecode.rs` tests `compile_expr_to_bytecode` returns Ok/Err on the BOUNDARY conditions, not that runtime evaluation matches compiler output. **The two are completely different invariants.**
- Worst-case: A `compile_expr_to_bytecode` bug that produces bytecode that the interpreter interprets DIFFERENTLY than the AST would. **The compiler lies about behavior; the runtime executes the lie.**

## Detail: `taint_propagation` SHIP-BLOCKER

- File: `integration_taint_propagation.rs` (2,578 lines)
- Is it a proptest? **NO.** Zero `proptest!` macro. All 50+ tests are `#[test]`. This is hardcoded unit tests with hand-picked inputs.
- The Section 47 violation documented in BIG-ASS-TESTING-TO-FIX.md Round 4 (validate_taint rejects SecretResultLeak for Finish) is **exactly the kind of bug fuzz finds**. The 2,578 lines of unit tests did NOT catch it.

## Mandated Fixes Before Ship

1. **DELETE OR CREATE:** `vb_compile/src/lib.rs:64-66` must either be deleted or the file `crates/vb_compile/src/bytecode_ast_parity.rs` must be created and wired.
2. **CREATE A REGISTRY:** Add `crates/workspace_tests/tests/SECTION_38_REGISTRY.md` (or `.yaml`) that maps each of the 11 Section 38 property names to the actual test file(s).
3. **FILL `concurrency_safety`:** Add a proptest at `crates/vb_runtime/src/property_tests/concurrency_safety.rs`.
4. **FILL `taint_propagation`:** Add a proptest at `crates/vb_core/src/property_tests/taint_propagation.rs`.
5. **REPLACE `vb_storage/src/property_tests.rs`:** Delete the 1-line stub. Either move existing storage property tests into it, or remove the file entirely.
6. **EXTEND `prop2_for_each_n_items_all_reentry`:** Add an `arb_slot_value_list` strategy that covers Null, Bool, Text, List(List(T)), and FanoutLimit boundary values.

## Final Verdict: REJECT — DO NOT SHIP

**RECOMMENDATION:** Open a bead `vb-section38-property-registry` that (a) creates the registry, (b) files 4 new beads for the 4 ship-blocker gaps, (c) closes the 2 already-identified false-positive claims (the lying comment and the empty stub). All 6 beads must close before `moon ci` is green on the Section 38 contract clause.

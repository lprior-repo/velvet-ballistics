# Test Review — vb-qdg7t (Independent Re-review of Source Semantic Digest Repair)

**Bead**: vb-qdg7t
**Reviewer**: DeepSeek v4 Pro via OMP (independent; did not author)
**Workspace**: `/home/lewis/src/isoloated/velvet-ballistics-vb-qdg7t-digest-review`

## Frozen Source (reviewed)

| Role | SHA |
|------|-----|
| Frozen checkpoint (`@`) | `013189c5be8cac3dba616253db39465e0de3691b` |
| Parent (prior rejected candidate) | `1e7aeee2495f2cfca0e6c1e7719727dc7515ea0a` |
| Digest-framing origin (vb-a2l02) | `c789b1b35cb5d19a60cf2f786cbda5947109dcb1` |

## Gate Result

**STATUS: APPROVED** (repair-scoped tests), with one pre-existing stale-test finding **T-1**
(outside this repair's diff; discrete fix required for a green digest suite).

---

## Tests Run (actual, true-exit runner `python3 /tmp/vb-run-gate.py 420 <prefix> ...`)

| Invocation | Result | Evidence |
|-----------|--------|----------|
| `cargo test -p vb_compile --lib` | **496 passed, 0 failed** (2.36s) | `.beads/vb-qdg7t/evidence/compiler-lib.*` |
| `cargo test -p vb_compile --test digest_framing_collision` | **9 passed, 0 failed** | `.beads/vb-qdg7t/evidence/digest-collision.*` |
| `cargo test -p vb_compile --features test-util --test digest_duplicate_parity` (part of multi-test run) | **0 passed, 4 failed** | `.beads/vb-qdg7t/evidence/digest-integration.*` |
| 12 digest/pipeline/yaml integration binaries (`digest_yaml_e2e`, `digest_structural_fields`, `digest_ask_determinism`, `digest_ask_empty_prompt`, `digest_ask_explicit_arm`, `digest_ask_prompt_sensitivity`, `digest_ask_timeout_sensitivity`, `digest_set_finish_regression`, `finish_digest_integration`, `finish_digest_structural`, `together_digest_sensitivity`, `repeat_digest_integration`) | **all green** (5+4+17+6+6+12+11+7+14[+1 ignored]+3+10+8) | `.beads/vb-qdg7t/evidence/digest-remaining.*` |

Note: integration tests require `--features test-util` (shared `tests/common/mod.rs` imports
`WorkflowSourceParts`, gated `#[cfg(any(test, feature="test-util"))]` at `lib.rs:61-62`); running
without the feature fails at *compile* (`E0432`), not at test. `CARGO_BUILD_JOBS=2` enforced by the
runner; workspace-own `target/` used (no cross-agent lock).

Author claims "496 lib, 9 regressions" — both reproduced exactly.

## Non-weakening verification (repair-scoped)

- **`at_once`/`pages`/`items` `None == Some(1)`** — `assert_eq!` restoration in
  `foreach_digest_tests.rs` and the new collect tests (`mod_compile_lowering/tests.rs:
  direct_digest_collect_pages_none_equiv_some1`, `..._items_none_equiv_some1`) are backed by the
  verified lowering oracle (`part_02.rs:188`, `part_03.rs:218-219` `unwrap_or(1)`). The inverse
  boundary (`Some(0)` ≠ `None`, `Some(0)` ≠ `Some(1)`) is asserted. Not a weakened assertion.
- **`digest_unit_tests.rs` / `wait_digest_unit_tests.rs`** deltas update manual reference byte
  sequences to the *new framed* encoding (type tags `0x01`/`0x02`, `TAG_STRING` length prefix,
  `TAG_NONE`/`TAG_SOME`). These are exact-byte deterministic-encoding assertions, correctly
  re-pinned to the intentional encoding change — not assertions weakened to force green.
- **Collision fixture is real byte framing**, not a "nonzero/distinct IR" proxy:
  `nested_step_id_output_framing_collision_regression` builds step id `"a"`/output `"setb"` vs
  `"aset"`/`"b"` (the exact `/tmp/vb-um4oi-collision.rs` repro) and asserts both the *semantic* and
  *compiled* digests differ. Two new nested `AuthorValue` boundary tests
  (`author_value_text_vs_sequence_boundary`, `author_value_sequence_vs_mapping_boundary`) assert
  nested values collide-free across container type tags.
- **No proof weakening found** in any repair-scoped test file. Fail-closed behavior is verified at
  the source level (non-exhaustive enums, so a future variant cannot be constructed from a test;
  `digest_unit_tests.rs:400` documents this and asserts current variants do not fall through).

---

## T-1 [pre-existing, out of repair diff] `digest_duplicate_parity` — 4 red tests

**Location**: `crates/vb_compile/tests/digest_duplicate_parity.rs` (PO-UT-003).

**Observed**: `ask_prompt_some_timeout_parity`, `ask_prompt_none_timeout_parity`,
`ask_empty_prompt_parity`, `set_finish_parity` all fail — "public and private canonical_digest
diverge".

**Root cause (verified)**: the test carries a *local mirror* of the deleted dead-code
`compile/mod.rs` `canonical_digest` (raw unframed concatenation; `Together => "parallel"` vs live
`"together"`; `Aggregate => "aggregate"` vs live `"reduce"`; no domain separator/type tags/length
prefix). The live `canonical_digest` has used framed encoding since `c789b1b35` (vb-a2l02). The
"private" subject does not exist in the tree: no `mod compile;` in `lib.rs`, no `src/compile/`
directory (deleted in `ee7fca189` "delete orphan files"). Parity with a dead mirror of the
deliberately-replaced unframed encoding is a false contract.

**Attribution**: *not* introduced by the F-4/F-5/F-6 repair. `digest_duplicate_parity.rs` is
byte-identical between `c789b1b35` and `013189c5` (empty diff) and absent from the
`1e7aeee24..013189c5` vb_compile diff. It was invalidated by the framing change at `c789b1b35`.

**Impact**: `cargo test -p vb_compile --tests` is not green in the frozen checkpoint; the digest
suite's full-integration gate is red. This is a real governance issue (no red tests in a frozen
integration candidate), but it reflects a stale test, not a production defect — there is exactly one
live `canonical_digest`.

**Discrete fix**: delete `crates/vb_compile/tests/digest_duplicate_parity.rs` (it mirrors removed
dead code; repo rules prohibit copy/forwarding parity tests), and update `.beads/vb-xi2f.33/*`
PO-UT-003 tracking to record that the parity subject (`compile/mod.rs`) was removed. Do not re-pin
the mirror to the framed encoding — that would manufacture a redundant copy-comparison test.

## Verdict

**APPROVED** for the repair's own tests: non-weakened, deterministic-encoding, real byte-framing
collision coverage; all repair-scoped tests pass (496 lib + 9 collision + 14 integration binaries).
**T-1** (`digest_duplicate_parity`) is a pre-existing stale test outside this repair's diff and must
be removed by the parent before the digest suite is green — it does not block the F-4/F-5/F-6
acceptance of this bead.

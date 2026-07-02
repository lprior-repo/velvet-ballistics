# Hazard Analysis — Storage Trim/Snapshot Key Length Cap (vb-d9ml3)

This hazard analysis covers the contract surface for the three magic-17
sites at `crates/vb_storage/src/trimming/logic.rs:36, 77, 222` and the
two new aliases `MAX_TRIM_KEY_LEN` / `MAX_SNAPSHOT_KEY_LEN` to be added at
`crates/vb_storage/src/constants.rs:74`.

The hazard model is **purely numeric/cap-refinement** plus the parser/codec
and persistence surfaces triggered by the Fjall raw-key read. No
temporal, concurrency, unsafe/provenance, FFI, or hostile-input hazard
is in scope.

---

## Hazard Class Summary

| Class | Triggered? | Severity |
|-------|------------|----------|
| Temporal / recovery | YES — overlong key masquerading as durable snapshot would cause wrong-event trimming (Round 10 issue 7) | **P1** (root cause of the bead) |
| Parser / codec | YES — length check is the parser boundary | **P1** (root cause) |
| Numeric / cap refinement | YES — `MAX_*_KEY_LEN == JOURNAL_KEY_BYTES` must be compile-time | **P2** (the named-cap invariants) |
| Persistence (Fjall) | YES — raw-key read tolerates any length | **P2** (downstream effect) |
| Public API (error variant + code) | YES — `IncompleteTrim { deleted_count: u64 }` shape and `0x4102` code must be stable | **P2** (test-stability surface) |
| Error taxonomy | YES — choice between `IncompleteTrim` and `MalformedKeyspaceRow` | **P2** (test-pinning surface) |
| Bounded state (prefix scan termination) | YES — scanner must terminate on first `Err` | **P2** (workflow invariant WF-INV-1) |
| Concurrency | NO — synchronous scan over a snapshot | n/a |
| Unsafe / provenance | NO — `#![forbid(unsafe_code)]` is set on `vb_storage` | n/a |
| FFI | NO — no FFI surface | n/a |
| Network | NO — local Fjall backend | n/a |
| Time / clock | NO — no clock reads | n/a |
| Hostile input / fuzz | NO — pure type-level numeric/cap refinement | n/a (proptest covers arbitrary lengths; fuzz adds no coverage) |
| Performance | LOW — `key.len()` is `O(1)` | n/a |
| Release / API | LOW — `pub(crate)` aliases; no public API breakage | n/a |

---

## H-CAP-1 — Temporal / Recovery Hazard (Round 10 issue 7)

**Severity**: P1 (root cause of the bead).
**Trigger**: An overlong raw key under `PREFIX_RUN_SNAPSHOT` is iterated by
`latest_durable_snapshot_seq` at `logic.rs:36`. Pre-fix, the
`key.len() != 17` check returns `Err(IncompleteTrim)` — fail-closed. **The
bug is NOT that the check is missing; the bug is that the literal `17` is
magic-numbered and bypasses the alias chain.**

**Why this matters**: future maintainers could (a) accidentally change
`JOURNAL_KEY_BYTES` to a new value without updating the three call sites;
(b) accidentally use a different literal (e.g., `16`, `18`) at one of the
three sites; (c) introduce a new trim scanner that forgets the length
check entirely.

**Mitigation (contract)**:

1. Add the `MAX_TRIM_KEY_LEN` / `MAX_SNAPSHOT_KEY_LEN` aliases. Both are
   `const` references to `JOURNAL_KEY_BYTES`, so the compiler enforces
   equality.
2. Replace the literal `17` at all three sites with the named cap.
3. Co-locate all three constants in `constants.rs:74-79` so a single
   grep-able edit changes them in lockstep.
4. Add a doc-comment block to the new aliases explaining the journal key
   envelope (`[prefix][run_id:u64 BE][seq:u64 BE]`) so the meaning of
   "17" is preserved in the source.

**Verifier lane**: unit tests (existing at `snapshot_tests.rs:208-248`,
`trimming/tests.rs:875-987`); augmented with explicit overlong (length > 17)
cases. See `proof-seeds.jsonl` PS-CAP-UNIT-001..003.

---

## H-CAP-2 — Parser / Codec Hazard

**Severity**: P1.
**Trigger**: `decode_storage_key` (`keys.rs:346`) already enforces the
length contract via `KeyDecodeError::KeyLengthMismatch`. The trim scanner's
length check at the three sites is the **primary** parser boundary.

**Why this matters**: without the primary length check, the trim scanner
would feed an overlong key to `decode_storage_key`, which would correctly
reject it — but as `KeyDecodeError::KeyLengthMismatch`, not
`TrimError::IncompleteTrim`. The error-code path would break.

**Mitigation (contract)**:

1. Keep the primary length check at all three sites.
2. Keep the secondary `decode_storage_key` re-check at
   `latest_durable_snapshot_seq:43-46` as a prefix-collision safety net.
3. Document the two-layer check pattern in the doc-comment on
   `MAX_SNAPSHOT_KEY_LEN`.

**Verifier lane**: existing `keys/tests.rs` length tests; the existing
trim scanner tests continue to pass.

---

## H-CAP-3 — Numeric / Cap-Refinement Hazard

**Severity**: P2.
**Trigger**: `MAX_TRIM_KEY_LEN == JOURNAL_KEY_BYTES == 17` must hold at
compile time. If any of the three constants drifts, the trim scanners
silently accept overlong or reject canonical keys.

**Why this matters**: the alias chain (`const A = B`) makes drift
impossible in stable Rust — `A` is a compile-time reference to `B`, and
`B` is a literal `17`. The compiler enforces the equality.

**Mitigation (contract)**:

1. Both aliases are `const` references to `JOURNAL_KEY_BYTES`, NOT literal
   `17` re-definitions. This is the single most important contract clause.
2. The proof planner authors a `cargo check` invocation that fails if any
   of the three constants diverges from `17`. This is a meta-test that
   runs on every CI gate.

**Verifier lane**: `cargo check` + a Rust unit test that asserts
`assert_eq!(MAX_TRIM_KEY_LEN, 17)` and
`assert_eq!(MAX_SNAPSHOT_KEY_LEN, 17)`. See
`proof-seeds.jsonl` PS-CAP-CONST-001.

**Forbidden**: defining `MAX_TRIM_KEY_LEN = 17` directly at the alias
site. The compiler would accept it but the alias chain would be broken.

---

## H-CAP-4 — Persistence Hazard (Fjall Raw Key)

**Severity**: P2.
**Trigger**: `item.key()` returns a `fjall::UserKey` (= `Slice`) whose
`.len()` may be ANY value. The contract assumes Fjall does not perform
silent length-fixups; LSMtree can store arbitrary byte slices per key.

**Why this matters**: legacy rows, test artefacts left in the on-disk
journal, or LSMtree corruption can produce overlong raw keys under the
canonical prefix.

**Mitigation (contract)**:

1. The trim scanner tolerates any `key.len()`.
2. On `key.len() != MAX_*_KEY_LEN`, the scanner aborts with
   `IncompleteTrim`.
3. No truncation, padding, or skip-and-continue.

**Verifier lane**: integration test (`snapshot_tests.rs:208-248`,
`trimming/tests.rs:875-987`) using `journal.run_snapshot.insert(...)` and
`journal.events.insert(...)` with hand-crafted raw byte vectors of varying
lengths. See `proof-seeds.jsonl` PS-CAP-INTEG-001..003.

---

## H-CAP-5 — Public API / Error Variant Hazard

**Severity**: P2.
**Trigger**: `TrimError::IncompleteTrim { deleted_count: u64 }` is a
public variant (`pub` in `trimming/mod.rs:51-54`). Its shape is part of
the public API contract of `vb_storage`.

**Why this matters**: changing the variant shape (adding fields,
renaming, removing) would break downstream callers and require a
semver-major bump.

**Mitigation (contract)**:

1. The variant shape is **unchanged** by this bead.
2. The `deleted_count` field semantics are unchanged.
3. The diagnostic code `0x4102` is unchanged.
4. The symbolic code `JOURNAL_INCOMPLETE_TRIM` is unchanged.

**Verifier lane**: `error_code_tests.rs:~244` continues to pass without
modification.

---

## H-CAP-6 — Error Taxonomy Choice Hazard

**Severity**: P2.
**Trigger**: The bead text offers two typed-error targets:
`TrimError::IncompleteTrim (0x4102)` and
`JournalError::MalformedKeyspaceRow (0x4030)`.

**Why this matters**: choosing the wrong variant would (a) break the
existing structural test assertions, (b) require a code-map change,
(c) lose the `deleted_count` progress counter, or (d) lose the prefix +
expected + actual diagnostic context.

**Mitigation (contract)**:

1. **Decision**: reuse `TrimError::IncompleteTrim` because the existing
   tests at `snapshot_tests.rs:208-248` and `trimming/tests.rs:875-987` are
   structural assertions on `IncompleteTrim { deleted_count: 0 }`.
2. `MalformedKeyspaceRow` is documented as a precedent (`headers.rs:67-72`)
   but NOT introduced for the trim path. See `error-taxonomy.md` for the
   full rationale.

**Forbidden**: introducing a new `TrimError` variant for overlong keys.
This would force a code-map change and break the existing tests.

---

## H-CAP-7 — Bounded-State Hazard (Prefix Scan Termination)

**Severity**: P2.
**Trigger**: Each trim scanner iterates a prefix cursor and must
terminate either with `Ok` (iterator exhausted normally) or `Err` (first
non-canonical observation).

**Why this matters**: a misbehaving iterator (infinite loop, hot spin) or
a misplaced `continue` statement could leave the scan unbounded.

**Mitigation (contract)**:

1. The existing `return Err(...)` on the first non-canonical observation
   (`logic.rs:37-38`, `:78`, `:223`) is preserved verbatim.
2. The existing `for item in self.events.prefix(prefix_key)` loop has
   Fjall's bounded iteration (the LSMtree is finite).
3. `count_trimmable_events` uses `database.snapshot()` for a stable view;
   the contract preserves this.

**Forbidden**: replacing `return Err(...)` with `continue` in the loop
body. The fail-closed invariant requires abort-on-first-bad-key.

---

## H-CAP-8 — Performance Hazard (Cap Check is O(1))

**Severity**: LOW.
**Trigger**: `key.len()` on a `fjall::UserKey` is `O(1)` (the length is
stored alongside the byte view). The cap check adds one branch per key;
negligible.

**Mitigation**: none required. The performance impact is below the noise
floor of the trim pass (which is dominated by `batch.remove` and
`batch.commit`).

**Verifier lane**: none (no benchmark regression expected).

---

## H-CAP-9 — Test-Stability Hazard (Existing Tests Must Keep Passing)

**Severity**: P2.
**Trigger**: The bead explicitly requires
"snapshot_tests.rs:208-248, trimming/tests.rs:875-987 must keep passing".

**Why this matters**: a wrong implementation could easily break these
tests. For example:
- Converging on `MalformedKeyspaceRow` would break
  `snapshot_tests.rs:235` (asserts `TrimError::IncompleteTrim`).
- Adding a new variant would break `error_code_tests.rs:~244`.
- Changing the `deleted_count` semantics (e.g., returning `0` always)
  would break `trimming/tests.rs:875-932` (asserts `IncompleteTrim { .. }`,
  not specifically `deleted_count: 0`).

**Mitigation (contract)**:

1. The contract pins the `IncompleteTrim { deleted_count: u64 }` shape and
   `0x4102` code. See `error-taxonomy.md`.
2. The proof planner's primary obligation is to verify that
   `cargo test -p vb_storage` is GREEN post-fix, with the existing tests
   still passing and the new overlong cases added.
3. A regression-test sweep must run on `moon ci` (canonical gate).

**Verifier lane**: integration tests + the moon-ci gate.

---

## Hazard Risk Matrix

| Hazard | Severity | Probability | Risk | Mitigation |
|--------|----------|-------------|------|------------|
| H-CAP-1 (temporal) | P1 | Low (mitigated by length check) | Medium | Named caps + co-location |
| H-CAP-2 (parser) | P1 | Low (existing decoder check) | Medium | Two-layer check preserved |
| H-CAP-3 (numeric) | P2 | Low (compile-time enforced) | Low | Alias chain + const-equal test |
| H-CAP-4 (persistence) | P2 | Low (Fjall tolerates any length) | Medium | Lenient tolerance + fail-closed |
| H-CAP-5 (public API) | P2 | Low (variant frozen) | Low | Shape preservation + tests |
| H-CAP-6 (error taxonomy) | P2 | Medium (choice error) | Medium | Pin to `IncompleteTrim` |
| H-CAP-7 (bounded state) | P2 | Low (Fjall-bounded) | Low | Existing `return Err` preserved |
| H-CAP-8 (performance) | LOW | Very Low | Very Low | None required |
| H-CAP-9 (test stability) | P2 | Medium (easy to break) | Medium | Contract pins + regression sweep |

The overall risk profile is **medium-low**. The bead is a focused
internal fix; the contract commits to preserving all existing tests,
adding new ones, and making zero cross-crate changes.

---

## Forbidden Patterns (consolidated)

| Pattern | Hazard class |
|---------|--------------|
| Magic literal `17` at the three sites | H-CAP-1, H-CAP-3 |
| Defining `MAX_*_KEY_LEN = 17` directly | H-CAP-3 |
| Introducing a new `TrimError` variant | H-CAP-5, H-CAP-6 |
| Converging on `MalformedKeyspaceRow` | H-CAP-6, H-CAP-9 |
| Returning `Ok(None)` to mask a malformed key | H-CAP-1 |
| Returning `Ok(0)` to mask a malformed key | H-CAP-1 |
| Replacing `return Err` with `continue` | H-CAP-7 |
| Truncating a long key | H-CAP-1, H-CAP-7 |
| Padding a short key | H-CAP-1, H-CAP-7 |
| `panic!` / `unwrap()` / `expect()` | H-CAP-2 (forbidden by Holzmann-Rust) |
| `unsafe { ... }` | n/a (`#![forbid(unsafe_code)]`) |

---

## Verifier Lane Profile (for the proof planner)

The contract classifies the bead's hazards into the following lane profiles
per `proof-pipeline-contract.md`:

| Profile | Hazards | Verifier lanes |
|---------|---------|----------------|
| Rust-local implementation | H-CAP-3, H-CAP-5, H-CAP-6 | unit tests + proptest |
| Parser / codec | H-CAP-2 | unit tests (existing `keys/tests.rs`) |
| Persistence | H-CAP-4 | integration tests + proptest |
| Bounded-state workflow | H-CAP-7 | integration tests |
| Temporal workflow | H-CAP-1 | not in scope (no TLA+ for sync single-thread flow) |
| Hostile input | n/a | not in scope (proptest covers arbitrary lengths) |
| Concurrency | n/a | not in scope (synchronous scan) |
| Unsafe / provenance | n/a | not in scope (`forbid(unsafe_code)`) |
| Performance | H-CAP-8 | not in scope (no regression expected) |
| Release / API | n/a | not in scope (no public API change) |

The proof planner's `proof-obligations.planned.jsonl` should therefore
cover the four "Rust-local" lanes (unit, proptest, integration, fuzz-omit
with rationale) and explicitly mark the others as `not_required` with
`not_applicability_evidence_refs`.

END OF HAZARD ANALYSIS.
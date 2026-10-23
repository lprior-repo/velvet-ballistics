# vb-8eocj.4.1 acceptance plan

Status: 13 direct unit tests are split into `source_metadata/tests.rs` and passed
through the standalone workspace target at V95; see `native-v95/README.md`.
The wider acceptance cases below remain proposed. The new Rust module is intentionally
unhooked and there is no cloud cargo, rustc, or rustfmt. Existing crate checks
cannot count as evidence for code they do not compile. Master must first approve
and explicitly include the production file in a native validation target.
Do not substitute a hand-copied test model. Existing test target ownership is
unchanged; this document grants no test-file edit fence.

## A. Module-only acceptance, after standalone API approval

1. Empty construction and `empty()` are equal, with zero count/bytes, no
   requirements, and checked lookup returning None for 0 and u32::MAX.
2. Unique symbols retain input order, including a non-lexical ordering;
   `symbol(id)` resolves exact bytes; count consistency accepts exactly the
   derived count. Out-of-range IDs never panic or alias another payload.
3. Duplicate bytes at distant positions are rejected with the correct original
   IDs; two empty symbols are duplicates, but one empty text symbol is legal.
   Unicode byte-equivalent values deduplicate; no Unicode normalization is
   silently applied, and visually similar distinct bytes remain distinct.
4. A 4,096-byte symbol is accepted; a 4,097-byte symbol is rejected. Include
   multibyte UTF-8 values so the test distinguishes bytes from characters.
   Checked aggregate accounting reports the exact sum without a 1 MiB cutoff.
   A valid table exceeding 1 MiB aggregate must not fail solely for that size.
5. Table count equal to MAX_VALUES_PER_RUN passes the structural bound when
   values are unique and short; count one greater fails. This is a cold resource
   test with an explicitly budgeted native fixture, not a hot-loop benchmark.
   Failure of scratch reservation returns ValidationAllocationFailed where a
   controlled allocator-failure facility is available; do not change the global
   allocator or production behavior merely to make this test convenient.
6. A binding `token: TOKEN` resolves alias and requirement separately and
   retains both; swapping either changes the resulting metadata. Both sides
   must be in range and nonempty. Distinct aliases may share one requirement.
   A duplicate alias or descending alias ID is rejected regardless of target.
   Requirement count larger than the symbol table is rejected before traversal.
7. Constructor failure returns no accepted metadata; successful metadata has
   only shared getters. Compile-fail/API review confirms no mutable getter or
   direct public fields can invalidate it. A clone remains logically equal.
8. Boundary/property generators vary table order, counts, UTF-8 sizes, aliases,
   and malformed IDs. No fixed dummy workflow is used as a Kani proof claim.
   Focused mutation checks should detect removing length, uniqueness, count,
   alias ordering, reference bounds, or empty-name rejection.

Native acceptance must record exact commands and results for formatting,
compilation, source clippy, behavior tests, and review on the delivered bytes.
The canonical repository gate remains moon ci, but this slice is not a whole
repository readiness claim. No formal or performance claim is made.

## B. Compiler and source acceptance after separate hook approval

1. Text constants, object construction keys, and accessor field keys share one
   interner. Reusing bytes yields the same ID; distinct bytes never collide.
   Checked overflow/resource failure becomes a typed compiler diagnostic.
2. Source text flows through Set, Choose branches, field/index access, action
   result binding, and top-level result mapping. Verify the actual returned
   UTF-8 and structured payloads, not only handle values or node counts.
3. Repeated compilation of semantically equivalent source yields identical
   table, binding pairs, canonical artifact bytes, and digest. Canonical map
   reordering rules remain consistent with existing source identity; object
   field insertion order remains observable where specified.
4. An explicit text/key change with unchanged symbol count changes the digest.
   Alias change and requirement-name change each change the digest. No secret
   value exists in the source fixture, artifact, diagnostic, or test logs.
5. Until Master approves host semantics, any secrets.* value expression fails
   explicitly. Accepting requirement metadata must not turn it into a clean
   ordinary input, fabricated placeholder, or runtime credential lookup.

## C. Wire/persistence compatibility after coordinated schema approval

1. New-version round-trip preserves all bytes, ID order, alias/requirement pairs,
   and digest. Reconstruct through the validated type, not unchecked deserialization.
2. Pinned old-schema fixtures exercise each approved outcome: explicit legacy
   decode and migration, or typed migration-required rejection. Empty tables do
   not make positional Postcard trailing fields backwards-compatible. Reject
   truncated tables, trailing data, invalid UTF-8, huge declared counts/lengths,
   duplicate symbols, invalid requirement references, and unknown versions.
3. Every compiler, storage, runtime, CLI, and packed-artifact decode route in
   the migration manifest enforces the same version and validation policy.
   A valid outer byte bound alone must not permit oversized allocation from a
   malicious nested length prefix.
4. Payload tampering, changing only a requirement target, or substituting an
   equal-sized table invalidates digest/evidence. The verifier fingerprint
   includes source_metadata.rs when its validator becomes an admission gate.
5. Persist, close, reopen, load, verify, and execute the same workflow. Test raw
   compiled IR and accepted-artifact envelopes separately. Preserve transactional
   acceptance and existing durability behavior; do not bypass evidence gates.

## D. Runtime, recovery, and secrets after independent decisions

1. Fresh runs seed the complete compiled symbol table before external inputs or
   actions create values. Required capacity is admitted before run exposure.
   Every source handle resolves to the expected bytes and object key.
2. Nonempty/mismatched stores fail closed; no silent append at shifted IDs.
   External structured values undergo bounded handle remapping, including
   nested object keys and lists. Compare literal versus external equal text and
   object key access to catch the current non-deduplicating-store/handle-equality
   mismatch. Exercise two concurrent workflows with different tables.
3. Checkpoint restoration preserves and verifies the exact source prefix,
   appends no duplicate seed, and restores dynamic values after it. Corrupt or
   truncated prefixes and wrong artifact bindings fail. Noncheckpoint recovery
   uses the same initialization semantics as fresh admission. Repeat a recovery
   to prove no accumulating duplicate prefix or ID drift.
4. Presence-only secret admission uses a synthetic availability provider, never
   process credentials or real environment secret values. Missing requirements
   reject before RunAccepted. Multiple aliases to one requirement retain their
   relationship. Ordinary input cannot satisfy a secret by name collision.
5. If Master later approves opaque handles, add explicit tests for action scope,
   capability denial, taint propagation, revocation, retry, checkpoint/recovery,
   and absence of raw values from all persisted/logged/public results. No such
   behavior is asserted by this module or test plan today.

## Stopping conditions

Module delivery requires the approved API, production-file native compilation,
focused passing tests, source review, and test review. End-to-end F1 completion
additionally requires B/C/D and the coordinated file fences. A passing
metadata-only test does not close source compilation or runtime support.

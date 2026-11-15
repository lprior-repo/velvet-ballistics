# Findings

No findings within the two authored property-oracle changes.

**STATUS: APPROVED**

Scope: independent TEST review of `vb-eirmk.75.8.25.77`, frozen candidate `c0cfa765b046b1a902ed1caee9e6930fe319cbcd`, base `f366003714753937573e04650bf63cc4e5beb8ea`. Contract SHA256 `f2a08141e89f75c281c87195c20b02308bea790f8090506bd4c553a3d668834d`. Bead was observed in_progress; this review does not close it or approve main landing/global engine acceptance.

Source bindings:

- `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs`: blob `91e045c201a596d7bc0eb35934d04d733884df7b`, SHA256 `db568d2bdc8b00c4fcd97a234100a1ae448718f6e5639f22c44364fafbd8bbdb`.
- `crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs`: blob `15dc11be42f00e81540d69661c5b665bb3fd7f69`, SHA256 `233781fbad394ee4770259a926022a5660e93bd28e4666d1e3f08bf3b525f88f`.

# Contract and oracle review

PS001 keeps the existing positive-run RunAccepted generator and actual public encode_record invocation. Encoding errors are converted to TestCaseError::fail and propagated through the proptest body, so an unexpected encoder error cannot skip the length assertion. The header length conversion uses usize::try_from with failure propagation. The lower-bound assertion remains intact. Counterfactual: an encoder that always returns Err passed the previous if-let guarded check; this property now fails. A successful zero/short buffer also fails the preserved minimum-length assertion.

PS008 keeps the actual fresh FjallJournal/JournalWriteBatch fixture, valid first append and existing success assertion, then requires batch.len()==1. No mock substitutes for the store or mutation. Counterfactual: append_event returning Ok without staging a write passed the former success-only assertion; the new exact pending-length oracle fails. Staging two writes also fails. The existing fresh-batch-zero and persisted/replay cases remain in the complete selected targets, so a constant-one length implementation is not protected by this single assertion alone.

Diff inspection confirms exactly these two test files changed. No generator, case, existing assertion, API, persisted format, dependency or gate was changed. No new panic/suppression/ignore was introduced. Review is limited to these improvements; unrelated existing arithmetic-smoke properties are not newly endorsed as runtime evidence.

# Executed and recorded evidence

Reviewer command `node .evidence/verify-storage77-test-review.mjs` ran in the reviewer's isolated Git/JJ workspace and exited0. It copied and checked both candidate Git blobs against the ready manifest, checked the ignored external contract hash, verified the exact two-path diff, and checked raw native log SHA256 `61f28dc8c1447da2b53350670ae5d8d554b09b9691e43606deed3063474a25d2`. No reviewer production/test edit, mutation execution or Rust build occurred.

Recorded author proving command, read from the source-bound ready manifest:

```text
CI=1 TMPDIR=/home/lewis/src/isoloated/vb-reconcile-r24/.evidence/storage-properties77/scratch CARGO_BUILD_JOBS=2 RUSTC_WRAPPER= RUSTFLAGS=-Dwarnings timeout 10m rustup run nightly-2026-04-28 cargo nextest run --cargo-quiet -p vb_storage --all-features --test proptest_vb_vzcuf_PS_001 --test proptest_vb_vzcuf_PS_008 --test-threads 4
```

The copied raw log shows Nextest run `60e7b481-fedc-46a2-834d-8c5acba7475d`, both complete target binaries, PASS lines for both changed properties, and summary `12 tests run: 12 passed (6 slow), 0 skipped`; native.exit contains0. Elapsed native summary283.359s is execution provenance, not performance evidence. format.exit contains0 and format.log is empty. These are recorded runs inspected against frozen source, not independently rerun native commands. The command respects two Cargo jobs, four test threads, ten-minute bound and persistent isolated scratch. Broad joined CI/main landing remain central integration work.

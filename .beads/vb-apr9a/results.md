# PS007 checked input repair

Seven unchecked operations replaced with checked conversion plus split_first. Result roundtrip asserted directly, no conversion-error path silently skips oracle. Local3090 exact JSON generated, mechanically applied and targeted rustfmt. Initial contract call incorrectly returned Rust instead of contract, rejected; v2 prose accepted. Initial implementation iflet silently skipped impossible failure, rejected; v2 compares Result directly.

Scoped clippy target with warnings/indexing_slicing/as_conversions denied exits0. Bounded cargo-fuzz default address sanitizer build, x86_64-unknown-linux-gnu,1000runs maxlen32 timeout2s/input rss4096MB outer180s,45nonempty boundary seeds, exits0. Actual cov14 ft15 rss67MB. Existing scalar oracle weak, no actual core/storage crash assurance claim, deeper oracle tracked vb-mvvrr. Raw fullbuild/run log retained. Seed corpus is smoke input only. Independent local5090 code review and local3090 test review approved lint-only repair.

Canonical lint-src now advances to two literal boolean assertions in kind_validation, separately tracked; no canonical source lint/CI green claim.

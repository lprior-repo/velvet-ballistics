# Recover strict storage property success assertions

Given the existing valid RunAccepted property generator in proptest_vb_vzcuf_PS_001.rs, encoding must succeed and its returned envelope must contain at least RECORD_HEADER_LEN bytes. Encoding failure must fail the property; it must not skip the assertion. Given a fresh actual Fjall JournalWriteBatch in PS008, its first valid append must succeed and its observable pending length must become exactly 1.

Recover only these two authored oracle improvements. Preserve all generators, cases, existing assertions, production code, error types, public APIs, dependencies, persisted formats, gates and intentional fuzz deletions. Convert header length with a checked conversion and propagate encoding failure as TestCaseError; add no panic, suppression or skipped case. Reuse the current proptest runner and real storage implementation.

Run pinned-nightly nextest for the two complete integration test targets with all features, warnings denied, two Cargo jobs, four test threads and persistent isolated TMPDIR. Run the relevant formatting check. Independent CODE and TEST review must bind exact source hashes and raw command evidence. Benchmarks are waived. Central integration owns broad joined CI and main landing; keep this bead open until that landing.

# Independent bounded TEST review

STATUS: APPROVED

Fixture verdict: PASS for current-main real issuance, retained exact archive bytes and private-copy replay at c92826fb216079584b5a4ee390abf68867027d8f. Runtime verdict: PASS for the bounded unsafe retry refusal contract at ae8ab1a7440cd65e01e52b32b18d5295bd3539e1. No actionable test findings.

Both candidates descend directly from production base f366003714753937573e04650bf63cc4e5beb8ea. Reviewer Git and JJ roots were verified as /home/lewis/src/isoloated/vb-taxonomy60-test-review-r24. Review workspace uses a new change above the frozen fixture commit. Parent retains live vb-eirmk.60 ownership; live bd show reported IN_PROGRESS. No production source or tests were edited by this reviewer.

## Scope and source receipts

Read every hunk of both diffs: fixture root test adds module inclusion and retained_taxonomy.rs; runtime action/mod.rs adds contract-based refusal and action_resume_contract.rs adds three public API tests. Read the complete new test files and the exercised catalog admission/grant/receipt and runtime registered reconciliation paths, existing scoped grant helper, retry classification helper, bounded contract, retained README, baseline/producer/replay logs, runtime red/green/caller logs, and master section 65.

- Fixture root test SHA256: 94245d6952ac5d9bb2f2c6a14350bf47be55699b5df2d44c5378499eef92fd96.
- Fixture retained_taxonomy.rs SHA256: 7f0183d120abeebb9f4421dc9f737ad297de0c5a50898cbeca1faafd2ba929e9.
- Frozen runtime action/mod.rs SHA256: 0e5595ca10b06f72f57a2d794060c1a89ebcf4979cc35d543047cc274f3fcdee.
- Frozen runtime action_resume_contract.rs SHA256: 900344c441d7ec8c315a6a6c6e99fe505df64766c567a64d18426b0b3453be62.
- Archive manifest SHA256: 4730cca9ffe0b73ccf6a8aae13ccbd623a82ec88fa0224ab6f4ec71edb505ee0; wc -l measured 284 entries. Every entry passed before and after replay; cmp of complete checksum outputs exited 0.

## Test judgment

Fixture production APIs issue accepted artifacts, schedule tickets, execute the concrete register effect, and obtain its opaque receipt. Replay operates only on a recursively copied private archive; artifact bytes and journal event values are compared against retained exports before runtime recovery. Manifest checks cover all exact retained file bytes, including pack, pre-dispatch events, raw receipt rows, and storage internals. Replay asserts identical pending ticket and reconciled receipt digest/value/ordinal with one committed effect and register value 42. Explicit absent archive cannot fall back to fresh generation. Cold Sends/Creates/Destroys artifacts receive Journaled admission checks only. Current archive captures no old retry-checkpoint fixture.

Runtime tests use public resume_action_outcome and exact expected typed errors. Every current nonpure side effect with Unsafe refuses before issuing another ticket, both with available capacity and with exhausted capacity; the exhausted case distinguishes refusal precedence from RetryExhausted. Permitted classification controls assert AwaitingAction and preserved run/step/action/run_attempt/capacity plus incremented sequence/attempt and reset fencing token. Pure None bypass covers every current retry-safety class. Nonretryable failures assert exact refusal across the whole current classification product. Removing the new contract predicate makes the named unsafe regression test fail, as recorded in red.log: Writes/capacity3 returned AwaitingAction attempt2 instead of the exact nonretryable error. The implementation helper and controls make inverse predicates, lost pure bypass, unconditional refusal, and reordered exhaustion observable.

## Independent execution

To avoid a duplicate heavy compile, private reflink-capable copies of the existing native test executables were executed in this review workspace. This is independent execution of supplied binaries, not an independent source rebuild; parent is responsible for the combined frozen-source build. All commands set TMPDIR to reviewer .scratch/tmp, use timeout 60s, and leave evidence inside this workspace. Native suites use --test-threads=4. No Cargo compilation or GPU execution occurred in this review.

- bin/registered-executor --test-threads=4 with VB_TAXONOMY_ARCHIVE pointing to reviewer retained-v3: 12 passed, 0 failed, 0 ignored (retained-native.log).
- bin/pending-recovery --test-threads=4: 22 passed, 0 failed, 0 ignored (pending-native.log).
- bin/registered-executor --exact retained_taxonomy::retained_taxonomy_authority_characterization_archive with archive variable absent: 1 passed, 11 filtered (fresh-native.log).
- Same exact fixture test with VB_TAXONOMY_ARCHIVE naming absent-archive: expected exit 101, 1 failed, error `retained archive must already exist` (missing-native.log).
- bin/action-resume-contract --test-threads=4: 3 passed, 0 failed, 0 ignored (runtime-native.log).
- sha256sum -c .evidence/taxonomy60/test-source.sha256: both source entries OK.
- From .evidence/taxonomy60, sha256sum -c retained-v3.sha256 before and after native replay: all entries OK (archive-before.log, archive-after.log); cmp exited 0.

Executable hashes are recorded in binary-source-receipt.txt. Binaries came from fixture-r24/target/debug/deps and behavior-r24/target/debug/deps; their owning source files matched frozen hashes at review time. Existing producer source-gate evidence was read: baseline 33/33; final producer/replay 34/34; runtime red 2 pass/1 fail; green 3/3; final callers 42 pass with explicit filter.

## Limits

These verdicts do not approve canonical taxonomy, key ingredient validation, durable retry-checkpoint compatibility, full phase38 acceptance, formal closure, or repository-wide readiness. Key formula and persisted formats were unchanged. Retained recovery remains subject to current bounded run lifetime using real clock data. Full moon ci was not run. Proof projection scope is delegated to CODE review; no proof claim is made here.

Applied jj, beads, test-reviewer, functional-rust, holzman-rust; read rules/workflow.md, subagents.md, code-review.md, testing.md; test-reviewer references/behavior-test-rubric.md and determinism-and-mutation.md; holzman-rust references/nasa-jpl-standards.md and runtime-performance-architecture.md. Power-of-Ten review relevant to tests: fixed classification loops and depth-capped copying terminate; fallible I/O propagates errors; assertions remain test-only. Heap-backed fixture serialization/storage copying is cold test setup, with no production allocation or performance claim.

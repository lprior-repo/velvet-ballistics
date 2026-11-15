# Independent TEST re-review: overflow controls and retained retry checkpoint

STATUS: APPROVED for both bounded scopes. No open findings. Prior overflow-addendum.md P2 disposition: fixed_with_evidence.

## Overflow controls

Reviewed complete diff eb3c2a81cfb615f7d340576b1259383ebcd41fa6..a61caaf4652f7757ab129412b15476c970bafd77. Only crates/vb_runtime/tests/action_resume_contract.rs changed, adding three tests. Exact frozen test SHA256: e86e2ad8c7a4561f7e48755ce5d677d3010383c7503ecf08750e03bb918315f3. Production SHA256 remains 0e5595ca10b06f72f57a2d794060c1a89ebcf4979cc35d543047cc274f3fcdee. Git diff --exit-code over production source and verification/verus exited 0 between these freezes.

The new public API tests cover seq=u64::MAX for permitted nonpure Safe and KeyRequired, and pure Unsafe bypass, asserting exact Core(InternalInvariantViolation { reason: seq_overflow_on_retry }). Nonpure Unsafe asserts exact nonretryable refusal with remaining and exhausted capacity. Permitted Safe and KeyRequired at exhausted capacity assert exact RetryExhausted { action:5, attempts:1 }. These distinguish all requested refusal/exhaustion/overflow orderings and would catch replacing the overflow branch with exhaustion or moving overflow before refusal/capacity. No impossible below-capacity u16 attempt-overflow input is introduced.

Independent private native binary execution: TMPDIR=$PWD/.scratch/tmp timeout 60s .evidence/taxonomy60-test-review/bin/action-resume-overflow --test-threads=4; 6 passed, 0 failed, 0 ignored, 0 filtered (overflow-native.log). This resolves the previous expanded overflow-coverage rejection. It does not approve proof artifacts.

## Retained retry fixture

Reviewed complete fixture source diff c92826fb216079584b5a4ee390abf68867027d8f..b0e249de044d1a8ec2a34aec5f2e7115d11cbd5f: new retained_retry.rs and retained_taxonomy.rs module inclusion. Frozen SHA256 retained_retry.rs: e4bb1abe9e0a112c263ef773e7cb65223658fab63e7a8e389444fbe8762ce013; retained_taxonomy.rs: 5ee141df8f502beb724147e75c673cbe0bb17cddfe492e402e2fefeb2f309928. Both owning worker source files matched frozen Git blobs at review time. Also read retry-README.md, source receipt, producer and same-byte replay logs, Runtime::fail_action, Runtime::recover_retry durable-chain validation boundary, and prior shared fixture helpers.

The producer obtains canonical scheduled tickets through real runtime admission, then invokes public fail_action/tick_all to issue RetryCheckpoint events. It extracts and checks real checkpoint payload digest, decoder version3 and exact postcard encode/decode identity. The checkpoint DTO is decoded from actual events; no replacement authority DTO is synthesized. Replay verifies checkpoint payload bytes and complete checkpoint event bytes, validates against the independently loaded accepted artifact, checks full journal prefix and exact stored accepted artifact bytes, and restores the exact action registry/artifact binding through public recover_retry. The side-effecting fixture commits one real register effect under a catalog-issued grant at the provider-committed/runtime-uncompleted boundary before the runtime failure. Original receipt reconciliation matches the retained digest. The explicitly requested manual retry restores run_attempt2; reconciliation for that new ticket returns exact ExecutorError::ReceiptMissing and leaves journal events, register value42 and effect_count1 unchanged. This supports original receipt identity and reconciliation without duplicate effects. It does not establish that executing the new manual invocation would be blocked; the test intentionally does not perform that effect and README states the limitation.

The reviewer privately copied the complete retry-retained-v1 archive and checksum manifest into its own persistent workspace. Manifest SHA256: b931713b960084afeaa36f7b5965134ea6f47ddb8df2a9f72f0a85f65c0efe2b. All131 retry entries passed sha256sum -c before and after native replay; cmp of complete outputs exited0. All284 original retained-v3 entries also passed after this review run, and cmp against the prior full checksum output exited0. Native fixture code reopens copies under reviewer scratch, so archive input bytes remain unchanged.

Independent native runs, all with TMPDIR=$PWD/.scratch/tmp and timeout60s:

- bin/retry-checkpoint --test-threads=4 with VB_TAXONOMY_ARCHIVE and VB_TAXONOMY_RETRY_ARCHIVE selecting reviewer copies: 13 passed, 0 failed, 0 ignored, 0 filtered (retry-native.log).
- bin/retry-checkpoint --exact retained_taxonomy::retained_retry::retained_retry_checkpoint_keeps_exact_action_contract_authority with archive/freeze variables absent: 1 passed, 12 filtered (retry-fresh-native.log).
- Same exact test with VB_TAXONOMY_RETRY_ARCHIVE naming absent-retry-archive: expected exit101, exact error retained retry archive must already exist (retry-missing-native.log). Missing replay input cannot silently regenerate.

The two existing worker pinned-nextest logs were read: retry-producer-v1.log and retry-same-bytes-replay.log each report41/41 passed,0 skipped. These are supplied source-build evidence, separate from reviewer native runs.

## Execution provenance and limits

Both Git and JJ roots were verified as /home/lewis/src/isoloated/vb-taxonomy60-test-review-r24. Native executables were copied into reviewer bin directory from behavior-r24 and fixture-r24 target/debug/deps; executable and frozen-source hashes are in followup-binary-receipt.txt. This review independently executes supplied binaries; it does not claim an independent source rebuild. Parent owns the combined-source build and integration check. No production source, tests, proof artifacts, dependencies, or gates were edited. Full moon ci was not run.

The prior skills/rules reference set remains applicable. This re-review grants no proof approval, canonical taxonomy acceptance, exhaustive MASTER65 acceptance, or whole-bead closure. vb-eirmk.60 remains blocked per parent coordination. Current real-clock fixture replay remains subject to bounded run-lifetime policy.

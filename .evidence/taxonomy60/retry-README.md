# Current retained retry checkpoint authority

Observed verification: retry-producer-v1.log reports 41/41 passed, 0 skipped; retry-same-bytes-replay.log reports 41/41 passed, 0 skipped. Both original and retry manifests were checked after replay, covering all 284 original archive files and all 131 retry archive files. rustfmt --check and git diff --check passed; the two test source files still match frozen producer source b0e249de exactly.

Exact producer test source was frozen before archive generation at b0e249de044d1a8ec2a34aec5f2e7115d11cbd5f, bookmark codex/taxonomy60-retry-source-r24, atop the unchanged c92826fb216079584b5a4ee390abf68867027d8f baseline fixture. Production source remains f366003714753937573e04650bf63cc4e5beb8ea, tree d6c13f925804b58f98260e012c6f2c0c5f4afd21. The producer uses the existing artifact issuer, runtime admission, fail_action, tick_all and recover_retry APIs. No checkpoint DTO authority was constructed, and no timer implementation or production code changed.

retry-retained-v1 preserves two actual Strict failure boundaries: None/Safe/DeterministicPure and registered Writes/KeyRequired/IdempotentExternal. Both contain complete raw Fjall journals, original accepted artifact bytes, original contract and ticket bytes, complete issued event vectors, exact RetryCheckpoint event bytes and its original checkpoint payload. The checkpoint decoder checks current version 3, original payload digest and exact encode/decode identity. Both checkpoint registries are compared against the independently loaded accepted artifact and the restored runtime registry.

The Writes fixture additionally preserves a real register effect and receipt committed under an actual catalog-issued grant before runtime completion, followed by genuine runtime failure. Replay copies all raw bytes, invokes durable-chain-bound recover_retry, confirms the original journal prefix, and restores run attempt 2. The original receipt reconciles identically and effect_count remains 1. The new run-attempt ticket has no receipt: its reconciliation returns exact ReceiptMissing, with no journal change and effect_count still 1. This does not claim that an explicitly requested manual retry can never execute a new invocation: current recover_state execution_mode and its existing action_recovery_tests require Live mode for failed-step retry; the checkpoint fixture does not perform that new effect.

Generate a new archive only from the exact frozen source, with an absent destination:

```sh
TMPDIR="$PWD/.evidence/taxonomy60/tmp" CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=2 NEXTEST_TEST_THREADS=4 RUSTC_WRAPPER= RUSTFLAGS=-Dwarnings VB_TAXONOMY_RETRY_FREEZE="$PWD/.evidence/taxonomy60/retry-retained-v1" timeout 10m rustup run nightly-2026-04-28 cargo nextest run --cargo-quiet --all-features -p velvet-ballistics-workspace-tests --test runtime_fjall_pending_action_recovery --test vb_7x0ol_probe_registered_executor --test vb_79ffw_ticket_authority_probe
```

Replay the same archive without permitting replacement generation:

```sh
(cd .evidence/taxonomy60 && sha256sum -c retry-retained-v1.sha256)
TMPDIR="$PWD/.evidence/taxonomy60/tmp" CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=2 NEXTEST_TEST_THREADS=4 RUSTC_WRAPPER= RUSTFLAGS=-Dwarnings VB_TAXONOMY_ARCHIVE="$PWD/.evidence/taxonomy60/retained-v3" VB_TAXONOMY_RETRY_ARCHIVE="$PWD/.evidence/taxonomy60/retry-retained-v1" timeout 10m rustup run nightly-2026-04-28 cargo nextest run --cargo-quiet --all-features -p velvet-ballistics-workspace-tests --test runtime_fjall_pending_action_recovery --test vb_7x0ol_probe_registered_executor --test vb_79ffw_ticket_authority_probe
(cd .evidence/taxonomy60 && sha256sum -c retry-retained-v1.sha256 && sha256sum -c retained-v3.sha256)
```

Raw run origins are real issuance clock data; replay remains subject to the current run-lifetime policy. Native verification uses the pinned repository nextest gate restricted to its three owning targets. moon ci, formal proof closure and canonical taxonomy acceptance remain outside this evidence scope. The owning vb-eirmk.60 remains blocked as coordinated by the parent task.

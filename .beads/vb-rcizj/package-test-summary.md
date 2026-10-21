# Complete runtime package execution

Source candidate: 5effa861576285586ecd1c9a1d4812f1c79bded3.
Command: env RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 timeout 300 cargo test -p vb_runtime --tests --no-fail-fast
Working directory: /home/lewis/src/isoloated/velvet-ballistics-vb-3l40a-runtime-5090
Exit: 101. Raw log: evidence/runtime-package-all-results.log.

Actual combined results: 2437 passed, 2 failed, 1 ignored. Library1903 passed; integration534 passed and2 failed. The ignored crash_child_marker is deliberately invoked by a parent crash/replay process test; this is not a blanket package-zero-ignore claim. Both failures are snapshot run identity error classification and are tracked in vb-90e4k. The repaired retry/attempt integrations9+15 pass. This is package regression evidence, not full moon CI, review approval or v1 acceptance.

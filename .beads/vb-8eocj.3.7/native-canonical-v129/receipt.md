# Canonical CI V129: preserved failure

Frozen source/receipt c2b0298d3071f535a828654fa273d97bb9647f1e, source SHA256 76a6ba18675eb0b7a7a3ff77af7e627fe06d7f50492aec96fc3a35fe4da50d1c. Exact source maps unchanged before/after.

Moon panic-surface exited 2 on three assertions in test-only nested_entries/support.rs. The module is loaded exclusively beneath cfg(test), but the generic helper filename misses the existing test filename exclusion. No script changes or assertion weakening are proposed.

After observing this concrete failure, the coordinator deliberately terminated only its own Moon process group. Wrapper recorded exit 1, 410.974 seconds, timed_out=false. This is a failed/cancelled canonical run, not a complete gate pass. Kani task completed before cancellation; no claim of whole pipeline success. Other fuzz builds were incomplete at cancellation. Local3090 proposal renames support.rs to support_tests.rs and updates the test-only path attribute.

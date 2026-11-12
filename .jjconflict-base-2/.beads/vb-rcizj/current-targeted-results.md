# Current ticket test repair checkpoint

Bounded actual command: `env RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 timeout 300 cargo test -p vb_runtime --test ticket_authority_matrix --test durable_retry_red_phase --test vb_jggy_lifecycle_tests`.

Result: 26 passed, 0 failed, 0 ignored. Raw output: evidence/inspected-matrix.log. The two matrix tests each exercise ten field corruptions using actual public API tickets, exact error variants, complete journal-vector equality and pending-ticket/frame invariance. These are twenty subcases, not twenty separate Rust tests. Genuine stale attempt2/current3 tests remain in vb_jggy_lifecycle_tests. Mandatory suspended error-handler PC/error-slot assertions pass after the red vacuity finding and local fixture repair. No independent approval or full package/CI acceptance is claimed for this checkpoint. Provenance distinguishes local-generated named-function patches from limited coordinator-authored matrix glue.

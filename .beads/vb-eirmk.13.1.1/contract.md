# vb-eirmk.13.1.1 contract

Add ONE new proptest to `crates/vb_ipc/src/queue/tests/array_queue_tests.rs`
using the existing unused `arb_ingress_frame` strategy. Generate 0..=16 frames,
submit all (cloned), recv all in same order with exact `IngressFrame` equality
(run/workflow/payload), then verify empty state (`Ok(None)`, `is_empty()`,
`len()==0`). No new deps, no API changes, no panic/expect/unwrap/as in new code.
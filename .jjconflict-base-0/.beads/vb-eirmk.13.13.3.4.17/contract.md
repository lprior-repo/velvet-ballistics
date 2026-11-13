# IPC ingress exact-content strategy

Bead: vb-eirmk.13.13.3.4.17. Exclusive source fence: crates/vb_ipc/src/queue/tests/array_queue_tests.rs minimal helper registration/import and crates/vb_ipc/src/queue/tests/ingress_frame_strategy.rs new private strategy. No registry13/14, socket helper, production ingress/protocol, gate, dependency, or schema edits.

Preserve ingress_queue_preserves_exact_frame_content_through_fifo_cycle and frames vec0..=16 plus every existing oracle/case byte-for-byte. Historical f971 strategy fixed RunId1 and zero digest and used expect; current source has only its call. Existing arb_capacity is not a reusable frame strategy.

IngressFrame actually contains RunId, WorkflowDigest, BoundedPayload only. Generate arbitrary u64 run IDs, all32 digest bytes, and arbitrary payload0..1024 as before. Include an additional bounded opaque wire-header payload family using the real IpcFrameHeader encoder with all current known/unknown commands, u16 flags and u64 correlations plus variable byte bodies. These are opaque queue bytes, not claims that invalid flags/unknown commands are accepted by protocol decoding.

Public IngressFrame construction remains checked and fallible. If a proptest optional-value bridge is required, mandatory exact Ok-unit assertions precede any Option conversion, so every unexpected constructor/encoder error fails the test rather than rejecting a sample. No ignore/unwrap/expect/panic macros, no fake constructors, no default-only frames or field-domain shrink.

All generated functions<=25 lines including signatures/braces, <=5params, newfile<=300. Preserve all unrelated legacy test source, including its existing callbacks and cloning semantics; no broad refactor. Parent runs full IPC native and independent CODE/TEST after guarded freeze. Source parse/model RESULT alone is not acceptance.

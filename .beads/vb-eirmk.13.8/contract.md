# Contract: express the FIFO empty-state invariant literally

- In `ingress_queue_preserves_exact_frame_content_through_fifo_cycle`, express the existing empty predicate obligation as `prop_assert_eq!(ingress.is_empty(), true, "queue must be empty")`.
- Preserve the property name, input generator domain `0..=16`, queue capacity 16, submitted and received payload order, exact payload comparison, final `None` assertion, and zero-length assertion.
- This is an equivalent spelling of the existing required boolean condition; do not change behavior, cases, gates, dependencies, or any other source.
- Verify with pinned file-only rustfmt and the scoped `vb_ipc` nextest filter; report the native command exit status.

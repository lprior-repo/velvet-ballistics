# Single-event append preflight

The single-event journal append validates the checked successor sequence and bounded per-run sequence capacity before invoking the adapter. Overflow returns storage SequenceOverflow; new-run capacity refusal returns ActiveRunCapacityExceeded. Neither calls the adapter or changes local authority. Adapter refusal preserves local sequence. A successful append publishes one event at the prior sequence and advances local sequence exactly once. Batch behavior remains unchanged.

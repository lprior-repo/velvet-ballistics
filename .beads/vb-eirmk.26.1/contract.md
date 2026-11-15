# Contract: admitted live timer storage

Delivery for vb-eirmk.26.1 after the explicit 2026-11-15 approval, “Land fixed tables and update phase 55 (Recommended).” The approved source is a8dae78aabf8322a21f43f3c35168900d03f4f26. The original design contract and all earlier candidates remain preserved in their original workspaces and refs. This amendment changes the live representation to match the approved MASTER55 correction; it grants no public API or persisted-format change.

RunAggregate owns one AdmittedTimers table, with entry and step storage reserved fallibly at cold shard construction for ShardConfig::max_active_runs. Live insertion, replacement, selection, removal, cancellation and clear reuse that storage. Full-capacity replacement reuses its existing slot. Public TimerEntry and PendingTimer values, utility TimerWheel APIs, snapshots, journal records and checkpoint encodings retain their existing representations.

Each live timer has one run and one step, with exact kind, deadline and generation. At most one timer belongs to a run. Pure preparation checks run membership, capacity, positions and generation before durable writes; failed preparation or append leaves existing ownership unchanged. Commit performs bounded in-place updates after the required durable append. Existing typed capacity, missing-run and invalid-timer outcomes remain authoritative.

An idle tick selects at most one owned due timer by deadline and then RunId. A no-due tick still drives existing pending work. Validated firing preserves current step, sequence and generation authority; stale, early, foreign or cancelled tokens cannot consume a different timer. Answer, cancellation, terminal cleanup, shutdown and cold restore retain their existing ownership rules. Clear and restore retain the reserved storage.

Bounded snapshots preserve the original prefix membership and sidecar iteration order before sorting. Cold recovery preserves existing timeout-unit and checked-generation policies; an Instant is not persisted and no cross-reopen monotonic-generation guarantee is introduced.

Acceptance uses registered firing, cancellation, capacity/replacement, append-denial, pending-work, authority and cold-recovery cases; independent CODE and TEST review of the frozen source; and joined checks on current main. The captured allocation observation covers attributed live helper frames during 64 capacity-two reuse cycles, not total-process allocation, universal OOM behavior, fairness, latency or a performance improvement. Formal parent and full MASTER acceptance remain separate.

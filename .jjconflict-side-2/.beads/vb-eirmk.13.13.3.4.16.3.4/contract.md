# Continue pending shard runs on idle ticks

This bounded repair owns only the private idle dispatcher in `crates/vb_runtime/src/shard/impl_parts/chunk_001/tick_drive.rs`. The existing `.16.3.4` child is claimed; the `.3.2` input fixture and `.3.3` secret/checkpoint scopes remain separate.

The unchanged two-node SetConst→Finish regression in `shard/tests/chunk_011.rs` must observe steps/completed of 1/0, 2/1, 2/1 across three ticks with a step budget of one. The unchanged sibling must count cumulative observations 1, 1, 3 as totals 1, 1, 3. Neither fixture nor assertion may change.

The r591 native failure reports tick-two steps 1 instead of 2. Source shows empty-queue `tick_inner` drives only due timers, while `drive_one_pending_run` is otherwise reached only on an already-running Resume. SetConst already increments the frame counter when advancing; Finish already increments once, and terminal cleanup records the cumulative count before pool release and clearing accounting. The defect is pending-work starvation, not an arithmetic or core-frame contract defect.

Preserve command-pop ordering, shutdown behavior, timer-wheel ordered non-consuming peek, timer error propagation, timer generation/deadline validation, commit points, and the explicit `drive_one_due_timer_at` seam. On an idle tick, a due timer has priority; otherwise drive at most one existing Running run whose current step is Pending using the existing selector and round-robin ordering. Do not drive a second run after handling a timer, even when the timer handler returns an error.

The minimal wrapper may check due presence at one captured Instant and delegate to the unchanged timer-at helper using that same Instant; both peeks are non-consuming and no mutation occurs between them. No extra clock read is added to command ticks. No new public API, allocation, scheduler state, deadline policy, journal format, or core behavior is introduced. No throughput or latency improvement is claimed.

Import only the actual parent `tick_drive.rs` preimage, guarded by SHA-256 `372a2f88a6e8e412cf1a9266d883d65468ec3b1c91bb4a585cd25e48f5370911`; the own-root difference is only an obsolete unused ready-run helper. Preserve all other parent bytes and existing tests. The source author is the single leased local3090 request; integration independently checks its exact diff, syntax, function/file bounds, and forbidden constructs.

Run pinned file-only rustfmt and the original regression plus accounting/timer/idle focused controls with an explicit own target, empty wrappers, jobs two, warnings denied, and bounded native resources where this inherited composition permits. Preserve raw commands, actual exits, and failures; a compile/preflight failure is not a test result. Parent composition, full native, CODE, and independent TEST acceptance remain separate.

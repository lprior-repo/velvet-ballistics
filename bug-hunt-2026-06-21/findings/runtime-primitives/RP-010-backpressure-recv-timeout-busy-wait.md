# RP-010: `BackpressureReceiver::recv_timeout` busy-waits on a 1 ms poll

- **Severity**: Low
- **Category**: perf
- **Location**: `crates/vb_runtime/src/action_queue/types.rs:99-127`
- **Confidence**: confirmed

## Description

`recv_timeout` polls `ArrayQueue::pop` in a tight loop with a `thread::sleep(remaining.min(Duration::from_millis(1)))` back-off. This is the classic "poll-sleep" anti-pattern: it introduces up to 1 ms of latency on each event and burns a core on each waiting consumer thread.

## Evidence

`crates/vb_runtime/src/action_queue/types.rs:113-127`:

```rust
loop {
    if let Some(w) = self.queue.pop() {
        return Ok(w);
    }
    let now = std::time::Instant::now();
    if now >= deadline {
        return Err(BackpressureRecvTimeoutError::Timeout);
    }
    let remaining = match deadline.checked_duration_since(now) {
        Some(r) => r,
        None => return Err(BackpressureRecvTimeoutError::Timeout),
    };
    std::thread::sleep(remaining.min(Duration::from_millis(1)));
}
```

The module doc (types.rs:77-83) and the surrounding code base explicitly prefer lock-free MPMC primitives (crossbeam) over `std::sync::mpsc` for hot paths. But crossbeam's `ArrayQueue` is non-blocking only — there is no `pop_blocking` — so `recv_timeout` reverts to polling. The comment does not justify the choice.

Per-call cost:
- Up to 1 ms of wall-clock latency waiting for the next warning (vs. immediate wake on push for a condvar/Notify-based queue).
- 1 000 thread wake-ups per second per receiver thread while idle.

## Adversarial Check

The backpressure channel is not the runtime's hottest path — the *enqueue* path is what matters for action-completion throughput. But the receiver is the half that the spec actually relies on for backpressure signals; if it sleeps for 1 ms while the queue fills, the operator learns about backpressure late.

Why not `tokio::sync::Notify` or `std::sync::Condvar`? The code base already mixes `crossbeam_queue`, `rtrb`, `Mutex<Vec>`, and `parking_lot` elsewhere — there is no architectural bar to a condvar.

Severity Low because this is a backpressure monitor, not the data path; the data path uses `enqueue`/`dequeue` directly without blocking.

## Suggested Fix

Either:

(a) Pair the `ArrayQueue<BackpressureWarning>` with a `parking_lot::Condvar` (or `std::sync::Condvar`) and notify on every successful `try_send`. `recv_timeout` then waits `condvar.wait_timeout_while(...)` instead of sleeping.

(b) Switch the backpressure channel to `crossbeam_channel::bounded` (MPMC, supports `recv_timeout` natively) and drop the `ArrayQueue` for this specific sub-channel. The action data queue can remain `ArrayQueue`.

(c) Document that 1 ms latency on backpressure signals is acceptable for this design, and add a comment explaining why polling was chosen over a condvar.

# Proof Repair Guide — vb-hs9m

**For proof-writer attention. Status: REJECTED — blocking State 6 delivery.**

---

## Critical Path (must fix before re-review)

### 1. Wire kani_trace_ring.rs into vb_runtime module tree

**File:** `crates/vb_runtime/src/lib.rs`
**Current state:** `kani_trace_ring.rs` exists at `crates/vb_runtime/src/kani_trace_ring.rs` but is not declared in `lib.rs`.
**Required change:** Add `#[cfg(kani)] pub mod kani_trace_ring;` alongside the other `#[cfg(kani)]` module declarations at lines 61–70.

```rust
// In lib.rs, after line 70 (kani_vt2f_shard_lower_semantics):
#[cfg(kani)]
pub mod kani_trace_ring;
```

**Why this is blocking:** `cargo kani --harness <name>` searches the crate module tree for `#[kani::proof]` functions. An unwired file is invisible to Kani's harness discovery.

**Verification after fix:**
```bash
cargo kani --harness verify_trace_ring_bounds --tests 2>&1 | grep -E "Running|proof|success|0 failures"
```

---

### 2. Fix Kani tooling (CBMC targets)

**Current state:** `cargo kani --version` → `error: No supported targets were found.`
**Required action:** Install Kani with proper CBMC target support. On the current host:

```bash
# Option A: Use Kani's built-in target setup
cargo kani setup

# Option B: Verify kani is pointing at correct CBMC installation
# The underlying issue is that CBMC's goto-cc target compiler is not configured
# for the host platform (x86_64-unknown-linux-gnu)

# Check:
cargo kani --version
kani --version  # direct CBMC binary

# If no targets, you may need:
rustup target add x86_64-unknown-linux-gnu
cargo kani --harness verify_trace_ring_bounds --tests
```

**Verification after fix:**
```bash
cargo kani --version  # Must not output "No supported targets were found"
cargo kani --harness verify_trace_ring_bounds --tests
# Must produce kani-trace-ring-report.html with 0 model-checking failures
```

---

### 3. Fix Miri tooling (rust-src)

**Current state:** `cargo +nightly miri test --package vb_runtime -- trace` → `fatal error: given Rust source directory ... does not exist`
**Required action:**

```bash
rustup component add rust-src --toolchain nightly
cargo +nightly miri test --package vb_runtime -- trace
cargo +nightly miri test --test bundle_tests
```

**Verification after fix:**
```bash
cargo +nightly miri test --package vb_runtime -- trace
# Must report 0 UB violations
```

---

## Secondary Fixes

### 4. Add unit tests for Kani-exclusive claims (compensating evidence)

While fixing Kani, also add unit tests for the Kani-exclusive claims that currently have no compensating evidence. This provides defense-in-depth.

**OBL-TRC-003 (drain_for_run correctness):** Add to `crates/vb_runtime/src/trace.rs`:
```rust
#[test]
fn drain_for_run_filters_correctly() {
    let mut ring = TraceRing::new(4);
    let run_a = RunId::new_v4();
    let run_b = RunId::new_v4();

    ring.push(TraceEvent::StepStarted { run: run_a, step: StepIdx::new(0) });
    ring.push(TraceEvent::StepStarted { run: run_b, step: StepIdx::new(0) });
    ring.push(TraceEvent::StepEnded { run: run_a, step: StepIdx::new(0) });
    ring.push(TraceEvent::RunFinished { run: run_a });

    let drained = ring.drain_for_run(run_a, 10);
    assert!(drained.iter().all(|e| e.run_id() == run_a));
    // Verify insertion order preserved
    let run_a_events: Vec<_> = drained.iter().map(|e| e.run_id()).collect();
    assert_eq!(run_a_events, vec![run_a, run_a, run_a]); // StepStarted, StepEnded, RunFinished
}
```

**OBL-TRC-004 (terminal event detection):** Add to `crates/vb_runtime/src/trace.rs`:
```rust
#[test]
fn has_terminal_event_for_run_true_and_false() {
    let mut ring = TraceRing::new(4);
    let run_a = RunId::new_v4();
    let run_b = RunId::new_v4();

    ring.push(TraceEvent::RunSubmitted { run: run_a });
    ring.push(TraceEvent::RunSubmitted { run: run_b });

    assert!(!ring.has_terminal_event_for_run(run_a));
    ring.push(TraceEvent::RunFinished { run: run_a });
    assert!(ring.has_terminal_event_for_run(run_a));
    assert!(!ring.has_terminal_event_for_run(run_b));
}
```

---

### 5. Execute OBL-EVN-003 integration test

```bash
cargo test --package xtask evidence::persistence::integration -- --test-threads=1
```

---

## Re-run Targets

After all fixes, re-run all blocked obligations:

```bash
# Kani TraceRing (after lib.rs fix + Kani setup)
cargo kani --harness verify_trace_ring_bounds --tests
cargo kani --harness verify_trace_ring_dropped_monotonic --tests
cargo kani --harness verify_drain_for_run_correctness --tests
cargo kani --harness verify_terminal_event_detection --tests

# Kani EvidenceBundle (after Kani setup)
cargo kani --harness schema_version_parse_non_panic
cargo kani --harness validator_correctness
cargo kani --harness write_read_non_panic

# Miri (after rust-src install)
cargo +nightly miri test --package vb_runtime -- trace
cargo +nightly miri test --test bundle_tests

# Integration
cargo test --package xtask evidence::persistence::integration -- --test-threads=1
```

---

## Waiver Status (do not re-evaluate)

These waivers are valid and do not need repair:
- **WAIVED-TLA-001**: TraceRing is SPSC local data structure; no temporal behavior in scope
- **WAIVED-LEAN-001**: No algebraic theorem kernel required for this bead scope
- **WAIVED-CONC-001**: SPSC lock-free ring; rtrb crate guarantees single-producer/single-consumer

---

## Priority Order

1. **lib.rs module wiring** (LETHAL-1) — zero-cost, immediate
2. **rust-src for Miri** (MAJOR-1) — `rustup component add rust-src --toolchain nightly`
3. **Kani CBMC targets** (LETHAL-2) — CI environment fix
4. **Unit tests for Kani-exclusive claims** (MAJOR-2 through MAJOR-5) — defense-in-depth
5. **OBL-EVN-003 execution** (MINOR-1) — one command

Once steps 1–3 are complete and the Kani/Miri runs produce reports with 0 failures, re-run proof-reviewer.

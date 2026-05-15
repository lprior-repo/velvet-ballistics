# Machine Gate Report: vb-0253.2

bead_id: vb-0253.2
phase: 11 (formal-verifier)
updated_at: 2026-05-15T00:00:00Z

## Machine Gates Executed

### TEST-001 — cargo test -p vb_ipc

```
$ cargo test -p vb_ipc
  Compiling vb_ipc v0.1.0
   Finished test [unoptimized + debuginfo] target(s) in 0.17s
    Running unittests src/lib.rs
    Running tests/tests.rs
      407 passed (2 suites, 0.20s)
```

**Result:** PASS — 407 tests, 0 failures, 0 flaky

---

### LINT-001 — cargo clippy -p vb_ipc

```
$ cargo clippy -p vb_ipc
    Checking vb_ipc v0.1.0
     Finished `dev` [unoptimized + debuginfo] target(s) in 0.11s

No issues found.
```

**Result:** PASS — 0 warnings, 0 errors

---

### BUILD-001 — cargo build -p vb_ipc

```
$ cargo build -p vb_ipc
   Compiling vb_ipc v0.1.0
    Finished `dev` [unoptimized + debuginfo] target(s) in 0.25s
```

**Result:** PASS — exit 0

---

### BUILD-002 — cargo build -p velvet_ballastics

```
$ cargo build -p velvet_ballastics
   Compiling velvet_ballastics v0.1.0
    Finished `dev` [unoptimized + debuginfo] target(s) in 1.57s
```

**Result:** PASS — exit 0

---

### LINT-001 (unsafe check) — rg for unsafe code

```
$ rg 'unsafe_code' crates/vb_ipc/src/*.rs
crates/vb_ipc/src/action_output.rs:1:#![forbid(unsafe_code)]
crates/vb_ipc/src/bounded.rs:1:#![forbid(unsafe_code)]
[... 15 files total]
```

**Result:** PASS — only `#![forbid(unsafe_code)]` declarations, no `unsafe` blocks

---

## Summary

| Gate | Command | Result |
|---|---|---|
| TEST-001 | `cargo test -p vb_ipc` | PASS (407/407) |
| LINT-001 | `cargo clippy -p vb_ipc` | PASS (0 warnings) |
| BUILD-001 | `cargo build -p vb_ipc` | PASS (exit 0) |
| BUILD-002 | `cargo build -p velvet_ballastics` | PASS (exit 0) |
| LINT-001 (unsafe) | `rg 'unsafe_code' crates/vb_ipc/src/*.rs` | PASS |

**STATUS: ALL PASS**

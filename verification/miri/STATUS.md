# Miri Coverage Plan

## 1. Current Status

Zero Miri coverage exists. No `cargo miri` has ever been run in this repository. No Miri CI step is configured.

## 2. Unsafe Code Scan (First-Party Crates)

| Crate | Has `unsafe` blocks? | `#![forbid(unsafe_code)]`? |
|---|---|---|
| `vb_ajc40_flux` | No | Yes |
| `vb_benchmark` | No | Yes |
| `vb_boundary_inventory` | No | Yes |
| `vb_cli` | No | Yes (all source files) |
| `vb_compile` | No | Yes (all source files) |
| `vb_core` | No | Yes |
| `vb_expr` | No | Yes |
| `vb_ipc` | No | Yes |
| `vb_proof_kernels` | No | Yes |
| `vb_queue_semantics` | No | Yes |
| `vb_runtime` | No | Yes |
| `vb_storage` | No | Yes |
| `vb_validate` | No | Yes |
| `vb_verification` | No | Yes |
| `vb_yaml` | No | Yes |
| `workspace_tests` | No | No |

**Scan results:** 990+ source files carry `#![forbid(unsafe_code)]`. Zero occurrences of `unsafe {`, `unsafe fn`, `unsafe trait`, `unsafe impl`, `transmute`, `MaybeUninit`, or `unsafe impl Send/Sync` exist in first-party code.

## 3. Recommended Priority Order

Since no first-party unsafe code exists, Miri's value is in detecting **UB triggered through safe code against third-party dependencies**. Priority order:

1. **`vb_core`** — highest risk area: complex logic, data structures, and serialization via `postcard`/`serde` (both transitively unsafe)
2. **`vb_storage`** — storage engine with I/O and internal data layout
3. **`vb_runtime`** — runtime execution with compilation artifacts and scheduling
4. **`vb_cli`** — CLI with IPC, file I/O, and process management
5. **`vb_compile`** — compiler with AST manipulation and bytecode emission
6. **`vb_ipc`** — inter-process communication layer
7. **`vb_queue_semantics`** — queue data structures with concurrency surface area
8. **Remaining crates** — lower risk, mostly pure logic with no I/O or FFI

## 4. Miri Command Template

```bash
# Test a single crate with Miri (default features)
cargo miri test -p <crate-name>

# Test with specific features
cargo miri test -p <crate-name> --features <features>

# Test with Miri flags for stricter checking
MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-symbolic-alignment-check" \
  cargo miri test -p <crate-name>

# Test a specific test
cargo miri test -p <crate-name> -- <test-name>

# Test with tree borrows (stricter than stack borrows)
MIRIFLAGS="-Zmiri-tree-borrows" cargo miri test -p <crate-name>

# Check for data race in concurrent tests
MIRIFLAGS="-Zmiri-check-number-validity" cargo miri test -p <crate-name>

# Run all workspace crate tests through Miri
for crate in vb_core vb_storage vb_runtime vb_cli vb_compile vb_ipc vb_queue_semantics; do
  cargo miri test -p "velvet-ballistics-$crate"
done
```

## 5. Known Blockers

| Blocker | Impact | Notes |
|---|---|---|
| **FFI/system calls** | Crates using `rustix` or `postcard` may hit platform-specific syscalls Miri cannot model (epoll, file I/O, networking) | `vb_cli`, `vb_storage`, `vb_runtime` likely affected. Miri supports basic file I/O (`-Zmiri-disable-isolation`), but not all syscalls. |
| **Time/clock dependencies** | Tests involving timestamps, durations, or timeouts may fail under Miri | Need `-Zmiri-disable-isolation` and possibly test exclusion. |
| **Performance / test timeouts** | Miri is 50-100x slower than native execution | Large test suites may need per-test Miri timeouts (`MIRI_TEST_TIMEOUT`), or selective test exclusion via `#[cfg(miri)]`. |
| **Generated code / proc macros** | `vb_compile` generates code at runtime; Miri may struggle with dynamic behavior | Likely requires focusing on unit tests rather than integration tests for this crate. |
| **Concurrent tests** | Miri's weak memory model checking is valuable but slow | Data race detection (`-Zmiri-check-number-validity`) is experimental; start without it. |
| **`-Zmiri-strict-provenance`** | Exposes provenance issues in deps (e.g., `postcard`, `serde_json`, `zerocopy`) | May cause false positives in dependencies; start with default provenance checking. |

## 6. Recommendation

File a bead to implement the initial Miri CI pass:

1. **Target:** Start with `vb_core` — purest logic crate, least I/O, best Miri compatibility
2. **Scope:** `cargo miri test -p vb_core --lib` (skip integration tests initially)
3. **Integration:** Add a `moon` task (e.g., `miri-test`) analogous to the existing Kani tasks, gated separately from `moon ci` to avoid CI timeout
4. **Exclusions:** Any test that hits filesystem, networking, or FFI should be `#[cfg(not(miri))]` or behind a `#[cfg(miri)]` test skip
5. **Evidence:** Save Miri pass/fail logs under `.evidence/miri/`

The `ws-batch-10` evidence archive and `fuzz/` directory do not need Miri coverage at this time.

---

*Document generated: 2026-06-14*

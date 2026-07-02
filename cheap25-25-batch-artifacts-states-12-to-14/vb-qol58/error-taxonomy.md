# Error Taxonomy — vb-qol58

> Bead: `vb-qol58` — Lint: fix source slicing and indexing issues in IPC and test utilities (P0 bug)
> Stage: `rust-contract` (State 3)
> Lane: Rust-local (canonicalization refactor; zero behavior change)

This is a **lint-canonicalization** bead. It introduces **zero** new error
variants. The error taxonomy below documents the **pre-existing** error
surface that the refactor must NOT perturb, plus the **lint-class error
vocabulary** that the refactor targets (these are clippy diagnostic codes,
not Rust `Result::Err` values).

## 1. Rust `Result::Err` Variants (must NOT change)

### 1.1 `IpcError` (used by `IpcFrameHeader::encode`)

From `crates/vb_ipc/src/error.rs`:

| Variant | Used by `encode`? | Behavior preserved by refactor? |
|---|---|---|
| `IpcError::Full` | No | N/A |
| `IpcError::Disconnected` | No | N/A |
| `IpcError::PayloadTooLarge { actual, limit }` | No | N/A |
| `IpcError::InvalidMagic { actual }` | No | N/A |
| `IpcError::UnsupportedVersion { actual }` | No | N/A |
| `IpcError::UnknownCommand(u16)` | No | N/A |
| `IpcError::ReservedNonZero { actual }` | No | N/A |
| `IpcError::PayloadLengthMismatch { header, actual }` | No | N/A |
| **`IpcError::HeaderEncodeFailed`** | **Yes** (line 44, 47, 50, 53, 56, 59, 62) | **Yes** — refactor preserves all 7 emit sites. |
| `IpcError::HeaderDecodeFailed` | No | N/A |
| `IpcError::PayloadLengthOutOfRange { actual }` | No | N/A |
| `IpcError::PayloadEncodeFailed` | No | N/A |
| `IpcError::PayloadDecodeFailed` | No | N/A |
| `IpcError::ResponseDecodeFailed` | No | N/A |

**Refactor invariant**: `encode` continues to return
`Err(IpcError::HeaderEncodeFailed)` for any cursor `write_*` failure, with
the **same** trigger conditions and the **same** call-site coverage.

### 1.2 `Option::None` (used by `SeededBytes::<N>::new`)

| Variant | Used? | Behavior preserved by refactor? |
|---|---|---|
| `None` | **Yes** — short-circuits when `N == 0` | **Yes** — `N == 0 → None` is BEFORE the RNG fill, refactor does not move it. |
| `Some(SeededBytes<N>)` | **Yes** — populated after RNG fill | **Yes** — `bytes` field is populated identically. |

**Refactor invariant**: `SeededBytes::<0>::new(_)` continues to return
`None`. `SeededBytes::<N>::new(_)` for `N > 0` continues to return
`Some(Self { bytes })` with the same byte sequence (RNG is deterministic
from `seed`).

### 1.3 (No error return for `FixtureBuilder::build_bytes`)

`build_bytes` is infallible by construction. It returns a `Vec<u8>` of
length `self.capacity.value` directly, with no `Result`. The capacity
itself is bounded by `FixtureCapacity::new` (returns
`Err(TestSetupError::InvalidCapacity(_))` for `0` or `> MAX_CAPACITY`),
but that is checked at the **constructor** site, not in `build_bytes`.

**Refactor invariant**: `build_bytes` continues to return `Vec<u8>` of
length `self.capacity.value` with the same RNG-driven byte sequence.

## 2. Lint-class Error Vocabulary (clippy diagnostic codes)

These are not Rust errors; they are `cargo clippy` diagnostic codes that
the workspace's `lint-src` gate (`.moon/tasks/all.yml:46-53`) denies with
`-D`. The refactor is a **preventive** tightening against these.

### 2.1 Lint classes defended by this bead (production)

| Lint class | Site | Current status | Post-refactor status |
|---|---|---|---|
| `clippy::indexing_slicing` | `frame_types.rs:41` | Clean (literal full-slice on fixed array doesn't currently trip nightly-2026-04-28) | **Clean** — canonical verb used |
| `clippy::indexing_slicing` | `seed.rs:23` | Clean | **Clean** — canonical verb used |
| `clippy::indexing_slicing` | `fixture.rs:58` | Clean (omitted-bound full-slice is currently clean) | **Clean** — canonical verb used |
| `clippy::get_unwrap` | (none of 3 sites) | N/A | N/A |
| `clippy::unwrap_used` | (none of 3 sites) | N/A | N/A |
| `clippy::string_slice` | (none of 3 sites) | N/A | N/A |

**Pre-existing lint-class errors (out of scope, not in lint-src)**: the
test-side `crates/vb_ipc/src/tests.rs` and `*_tests.rs` modules contain
many `&buf[..N].try_into().unwrap()` and `bad_header[..4].copy_from_slice(…)`
patterns; these are **out of default scope** for this bead (delivery-scope
row 14).

### 2.2 Lint classes that must NOT regress

| Lint class | Where monitored |
|---|---|
| `clippy::all` | `.moon/tasks/all.yml:51` |
| `clippy::unwrap_used` | `.moon/tasks/all.yml:51` |
| `clippy::expect_used` | `.moon/tasks/all.yml:51` |
| `clippy::panic` | `.moon/tasks/all.yml:51` |
| `clippy::panic_in_result_fn` | `.moon/tasks/all.yml:51` |
| `clippy::todo` | `.moon/tasks/all.yml:51` |
| `clippy::unimplemented` | `.moon/tasks/all.yml:51` |
| `clippy::dbg_macro` | `.moon/tasks/all.yml:51` |
| `clippy::indexing_slicing` | `.moon/tasks/all.yml:51` |
| `clippy::string_slice` | `.moon/tasks/all.yml:51` |
| `clippy::get_unwrap` | `.moon/tasks/all.yml:51` |
| `clippy::arithmetic_side_effects` | `.moon/tasks/all.yml:51` |
| `clippy::as_conversions` | `.moon/tasks/all.yml:51` |
| `clippy::let_underscore_must_use` | `.moon/tasks/all.yml:51` |
| `clippy::await_holding_lock` | `.moon/tasks/all.yml:51` |
| `clippy::print_stdout` | `.moon/tasks/all.yml:51` |
| `clippy::print_stderr` | `.moon/tasks/all.yml:51` |
| `unsafe_code` | `.moon/tasks/all.yml:51` |

**GOD RULE from AGENTS.md**: the bead MUST NOT weaken this gate. The
refactor must produce zero new warnings or the gate fails closed.

### 2.3 Panic / `unwrap` / `expect` audit (per `.moon/tasks/all.yml`)

The three refactor sites do **not** introduce any new `unwrap`/`expect`/
`panic`/`todo`/`unimplemented`/`dbg!` macros. The refactor is purely a
spelling change. The pre-existing surface at each site:

| Site | `unwrap` | `expect` | `panic` | `todo` |
|---|---|---|---|---|
| `frame_types.rs:41` | 0 | 0 | 0 | 0 |
| `seed.rs:23` | 0 | 0 | 0 | 0 |
| `fixture.rs:58` | 0 | 0 | 0 | 0 |

## 3. Forbidden Error Patterns (for this refactor)

| Pattern | Why forbidden | Replacement |
|---|---|---|
| Replacing `&mut bytes[..]` with `&mut bytes[0..bytes.len()]` | Same `clippy::indexing_slicing` violation, more verbose. | `bytes.as_mut_slice()` |
| Replacing `&mut bytes[..]` with `bytes.as_mut_ptr_range()` | Wrong: that returns `Range<*mut u8>`, not `&mut [u8]`. Type mismatch. | `bytes.as_mut_slice()` |
| Replacing `rng.fill(&mut bytes[..])` with `bytes.fill_with(|| rng.gen())` | Wrong: doesn't use the same deterministic RNG draw path; behavior may change. | `rng.fill(bytes.as_mut_slice())` |
| Adding `.unwrap()` / `.expect()` to the canonical accessor | Adds `clippy::unwrap_used` / `clippy::expect_used` deny violations. | Trust the type; `as_mut_slice()` is infallible. |

## 4. Error-Recovery Semantics (none introduced)

This refactor introduces **zero** recovery paths, retries, fallback
values, or compensating actions. The `Option::None` short-circuit in
`SeededBytes::<0>` is preserved verbatim.

## 5. Anti-Hallucination Markers

- All `IpcError` variants listed in §1.1 are taken verbatim from
  `crates/vb_ipc/src/error.rs` as read in this isolated workspace.
- The 7 `HeaderEncodeFailed` emit sites correspond exactly to lines 44,
  47, 50, 53, 56, 59, 62 of `frame_types.rs`.
- The `None` short-circuit at `seed.rs:18-20` is unchanged.
- No new error variants are invented.
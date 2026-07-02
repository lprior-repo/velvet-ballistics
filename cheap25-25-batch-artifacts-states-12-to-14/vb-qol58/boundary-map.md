# Boundary Map — vb-qol58

> Bead: `vb-qol58` — Lint: fix source slicing and indexing issues in IPC and test utilities (P0 bug)
> Stage: `rust-contract` (State 3)
> Lane: Rust-local (canonicalization refactor; zero behavior change)

This boundary map partitions the three refactor sites between the
**pure core** (typed-byte-container constructors) and the **imperative
shell** (writer application). It also identifies every "external
dependency surface" the refactor touches, and asserts that the refactor
introduces **zero** new boundary crossings.

## 1. Pure Core vs Imperative Shell

### 1.1 Pure Core (no I/O, no time, no network, no RNG-state-mutation-from-outside)

| Function | Body | Side effects |
|---|---|---|
| `IpcFrameHeader::encode` | Allocate `[u8; IPC_HEADER_LEN]` on stack, fill via Cursor. | None — writes to a stack-local array only. |
| `SeededBytes::<N>::new(seed)` | Construct `StdRng::seed_from_u64(seed)`, fill local `[u8; N]`. | None visible to caller. The RNG is a *pure* deterministic state machine. |
| `FixtureBuilder::build_bytes(self, seed)` | Construct `StdRng::seed_from_u64(seed)`, fill local `Vec<u8>`. | None visible to caller. Same RNG purity. |

### 1.2 Imperative Shell

There is **no** imperative shell for any of the three sites — none of them
talks to a socket, a file, a clock, a thread pool, or an async executor.
All three are pure synchronous functions whose only "imperative" act is
the in-place buffer fill.

### 1.3 Async Shell

None. The IPC transport layer (where async lives, in
`crates/vb_ipc/src/{client,server,ingress}.rs`) is **not** touched by this
refactor. The three sites are strictly synchronous.

## 2. Boundary Surface Inventory

| Boundary | Touched by this refactor? | Notes |
|---|---|---|
| **Storage** (Fjall, disk, sled, etc.) | **No** | None of the three sites touch storage. |
| **Network** (Unix socket, TCP, etc.) | **No** | The IPC transport is in `crates/vb_ipc/src/{client,server,ingress}.rs`; the three sites here are encode-only and decode-only helpers. |
| **Time** (`Instant::now`, `Duration`, etc.) | **No** | None of the three sites touch time. |
| **FFI / `unsafe`** | **No** | All three sites are `#![forbid(unsafe_code)]` (the `vb_ipc` crate). |
| **External process / subprocess** | **No** | N/A |
| **Filesystem** | **No** | N/A |
| **Async runtime / `tokio::spawn`** | **No** | N/A |
| **Channel / queue / MPSC** | **No** | N/A |
| **Atomic / lock / mutex** | **No** | N/A |
| **Parser boundary** (reading external bytes into a typed struct) | **Indirectly via `decode`** | `decode` is the parser boundary; this refactor only changes `encode`. The `decode` path at `frame_types.rs:71` uses `bytes.as_slice()` which is already canonical. |
| **RNG source** (`StdRng::seed_from_u64`) | **Yes — preserved** | The refactor does not change the RNG seed constructor; only the fill-borrow expression. |

## 3. Typed-Byte-Container Boundary (the only one this refactor touches)

The refactor's entire boundary surface is the **typed-byte-container
borrow boundary** — the place where a local byte container is borrowed
mutably and handed to a writer.

### 3.1 Before (forbidden surface)

```rust
// frame_types.rs:41
let mut cursor = std::io::Cursor::new(&mut bytes[..]);
// seed.rs:23
rng.fill(&mut bytes[..]);
// fixture.rs:58
rng.fill(&mut vec[..]);
```

The expression `&mut bytes[..]` is a **full-slice range expression** on
a typed container. The `..` range is redundant because the type already
encodes the length; the expression carries no information beyond what the
type system already provides, but it DOES make the borrow subject to
`clippy::indexing_slicing` under widened lint gates.

### 3.2 After (canonical surface)

```rust
// frame_types.rs:41
let mut cursor = std::io::Cursor::new(bytes.as_mut_slice());
// seed.rs:23
rng.fill(bytes.as_mut_slice());
// fixture.rs:58
rng.fill(vec.as_mut_slice());
```

The accessor `as_mut_slice()` returns a `&mut [u8]` whose length is
**statically equal** to the container's length by construction. There is
no range expression, no opportunity for off-by-one, and no `clippy`
deny violation.

### 3.3 Alternative canonical surface (auto-deref)

```rust
// frame_types.rs:41
let mut cursor = std::io::Cursor::new(&mut bytes);
// seed.rs:23
rng.fill(&mut bytes);
```

Auto-deref of `&mut [u8; N]` to `&mut [u8]` is equivalent to
`as_mut_slice()` and equally canonical. Either form is acceptable; the
delivery-scope row 1 fix-shape recommends `bytes.as_mut_slice()` for
symmetry with `decode`'s `bytes.as_slice()` at line 71.

## 4. Functional Core / Imperative Shell Map

```
+-----------------------------------------------------------------+
|  Pure Core (functional)                                         |
|                                                                 |
|  [u8; IPC_HEADER_LEN]   bytes.as_mut_slice()  ->  &mut [u8]     |
|  [u8; N]                bytes.as_mut_slice()  ->  &mut [u8]     |
|  Vec<u8>                vec.as_mut_slice()    ->  &mut [u8]     |
|                                                                 |
+-----------------------------------------------------------------+
                              |
                              v  (immutable-borrow contract)
+-----------------------------------------------------------------+
|  Imperative Shell (writer application)                          |
|                                                                 |
|  Cursor::new(&mut [u8])   ->  write_u32, write_u16, write_u64   |
|  Rng::fill(&mut [u8])    ->  populates bytes deterministically  |
|                                                                 |
+-----------------------------------------------------------------+
```

The pure core owns the **type**; the imperative shell owns the **effect**.
The refactor lives entirely in the pure core's accessor choice.

## 5. Out-of-Scope Boundary Surfaces (Explicit)

These boundary surfaces appear in nearby code but are NOT touched by this
refactor:

- **Unix-socket pair construction** in `crates/vb_ipc/src/client/tests.rs:252-255` (test-only).
- **Server bind / `serve_ipc`** lifecycle in `crates/vb_ipc/src/server/impl_tests.rs`.
- **Postcard payload codec** in `crates/vb_ipc/src/payloads.rs` and `codec.rs`.
- **Queue / ingress channels** in `crates/vb_ipc/src/{ingress,queue}/`.
- **Bounded payload newtype** in `crates/vb_ipc/src/bounded.rs`.
- **Storage round-trips** in `crates/workspace_tests/tests/*` (out-of-scope per delivery-scope row 18).

## 6. Cross-Boundary Invariants (preserved by the refactor)

| Invariant | Holds because |
|---|---|
| `encode` does not perform I/O | The Cursor writes only to the stack-local `bytes` array; no socket, file, or pipe is touched. |
| `encode`'s returned `[u8; 24]` has the same bytes for the same input | The 7 `write_*` calls are in the same order; the only change is the borrow expression. |
| `SeededBytes::<N>::new(seed)` produces the same byte sequence | `StdRng::seed_from_u64(seed)` is deterministic; the RNG's `fill` writes the same bytes regardless of how the destination borrow is expressed. |
| `FixtureBuilder::build_bytes(self, seed)` produces the same `Vec<u8>` | Same RNG purity; same fill length (`vec.capacity.value` equals `vec.len()`). |
| `Vec<u8>` length before/after fill | `vec![0u8; cap]` initializes the vec to exactly `cap` elements; `fill` writes into all of them; the returned vec has `cap` elements. |

## 7. Anti-Hallucination Markers

- All three sites are read from this isolated workspace.
- No new boundary crossing is introduced.
- The "before" / "after" surfaces are literal diffs (3 lines changed
  total) — not a refactor across multiple functions or modules.
- The cross-boundary invariants are all preserved because the RNG seed
  and the writer call sequences are unchanged.
# Domain Model — vb-qol58

> Bead: `vb-qol58` — Lint: fix source slicing and indexing issues in IPC and test utilities (P0 bug)
> Stage: `rust-contract` (State 3)
> Lane: Rust-local (canonicalization refactor; zero behavior change)
> Workspace: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`

This is **not** a feature bead. There is no new domain, no new entity, no new
command. The domain modeled here is the **byte-buffer canonicalization
surface** already exercised by the production code in
`crates/vb_ipc/src/frame_types.rs` and the test utilities in
`crates/workspace_tests/src/test_util/{seed,fixture}.rs`. The contract's job
is to make the **illegal slice patterns unrepresentable** and the **legal
typed-byte-container access canonical**.

## 1. Ubiquitous Language

| Term | Definition | Where it lives |
|---|---|---|
| **Typed byte container** | A Rust value whose type statically fixes its length and element kind: `[u8; N]`, `Vec<u8>`, `&mut [u8]`, `&[u8]`. | Rust core type system |
| **Full-slice expression** | An omitted-bound range (`&mut bytes[..]`, `&buf[..]`) that lowers to the entire slice of the source. Distinguishable from partial-range slicing (`bytes[..4]`) which IS `clippy::indexing_slicing`. | Rust surface syntax |
| **Canonical verb** | The blessed accessor for full-slice access on a typed byte container: `bytes.as_mut_slice()` for `[u8; N]`, `vec.as_mut_slice()` for `Vec<u8>`, auto-deref `&mut bytes` for `[u8; N]`, `bytes.as_slice()` for `&[u8]`. | Project-wide idiom |
| **Cursor writer target** | The buffer that `std::io::Cursor::new` borrows mutably to perform little-endian field writes during IPC header encoding. Must be a `&mut [u8]` whose length equals the encoded wire layout. | `crates/vb_ipc/src/frame_types.rs:39-64` |
| **Deterministic byte producer** | A `StdRng` seeded from a `u64` (via `seed_from_u64`) that fills a fixed-size byte buffer reproducibly for test fixtures. | `crates/workspace_tests/src/test_util/seed.rs:21-23` and `fixture.rs:56-58` |
| **Fixed-size encoded wire header** | The 24-byte little-endian IPC header (`magic[4] version[2] command[2] flags[2] reserved[2] correlation[8] payload_len[4] = 24`). Length is enforced by the constant `IPC_HEADER_LEN`. | `crates/vb_ipc/src/constants.rs` |
| **Surgical mutation** | Mutating a single field of a typed byte container via a typed accessor (e.g., `header_bytes(...).with_bad_magic(0xDEAD_BEEF)`) rather than rewriting the container with a literal range slice. | Refactor target for test sites |

## 2. Entities (none introduced)

This refactor adds **zero** new entities. The pre-existing entities stay:

- `IpcFrameHeader` — owned, fixed-layout IPC header struct.
- `IpcFrame` — owned, bounded-payload frame struct.
- `SeededBytes<N>` — owned, deterministically-seeded fixed-size byte struct.
- `FixtureBuilder` — owned builder for `Vec<u8>` test fixtures.
- `FixtureCapacity` — owned capacity newtype (max 1 MiB).

## 3. Value Objects (newtypes, smart constructors, accessors)

This refactor does **not** introduce new value objects. The contract is about
*canonical accessors on existing value objects*:

| Existing value object | Canonical accessor (replace literal-range slicing with) |
|---|---|
| `[u8; IPC_HEADER_LEN]` (e.g., `bytes` in `frame_types.rs:40`) | `bytes.as_mut_slice()` for `Cursor::new` mutation target; auto-deref `&mut bytes` also acceptable; matches the existing `decode` path at `frame_types.rs:71` (`Cursor::new(bytes.as_slice())`). |
| `[u8; N]` (e.g., `bytes` in `seed.rs:22`) | `rng.fill(bytes.as_mut_slice())` or `rng.fill(&mut bytes)` for full-array fill; both are equivalent and lint-clean. |
| `Vec<u8>` (e.g., `vec` in `fixture.rs:57`) | `rng.fill(vec.as_mut_slice())` or `rng.fill(&mut vec)` for full-vec fill; both are lint-clean. |

The key property: **the length is statically known from the type**, so any
`[0..N]`, `[..N]`, or `[N..]` literal-range slicing is *redundant* — the
range expression carries no information beyond what the type already
provides. The canonical accessor makes this redundancy visible to the
reviewer.

## 4. Commands (none introduced)

This refactor introduces **zero** new commands. The pre-existing commands stay:

- `IpcFrameHeader::encode(&self) -> Result<[u8; IPC_HEADER_LEN], IpcError>`
- `IpcFrameHeader::decode(bytes: &[u8; IPC_HEADER_LEN], max: MaxPayloadBytes) -> Result<Self, IpcError>`
- `SeededBytes::<N>::new(seed: u64) -> Option<Self>`
- `FixtureBuilder::build_bytes(self, seed: u64) -> Vec<u8>`

What changes is only the **internal buffer-borrow expression** used to
produce or consume the underlying bytes.

## 5. Events (none introduced)

This refactor emits **zero** new events.

## 6. Policies / Invariants

The contract adds the following **lint-canonicalization invariants** at the
three production sites, plus a **clippy-deny** invariant across the
workspace `lint-src` gate:

| ID | Invariant | Site |
|---|---|---|
| `INV-LINT-CURSOR-TARGET` | The mutable borrow passed to `Cursor::new(...)` in `IpcFrameHeader::encode` is the result of `bytes.as_mut_slice()` (or auto-deref `&mut bytes`); it is **never** a literal-range slice expression. | `crates/vb_ipc/src/frame_types.rs:41` |
| `INV-LINT-RNG-FILL-ARRAY` | The mutable borrow passed to `rng.fill(...)` for a fixed-size `[u8; N]` array is the result of `bytes.as_mut_slice()` (or auto-deref `&mut bytes`); it is **never** `&mut bytes[..]`. | `crates/workspace_tests/src/test_util/seed.rs:23` |
| `INV-LINT-RNG-FILL-VEC` | The mutable borrow passed to `rng.fill(...)` for a `Vec<u8>` is the result of `vec.as_mut_slice()`; it is **never** `&mut vec[..]`. | `crates/workspace_tests/src/test_util/fixture.rs:58` |
| `INV-LINT-DENY-GATE` | The workspace `lint-src` moon task (`.moon/tasks/all.yml:46`) continues to deny `clippy::indexing_slicing`, `clippy::get_unwrap`, `clippy::unwrap_used`, etc. The bead must NOT weaken this gate. | `.moon/tasks/all.yml:46-53` |

## 7. Forbidden States

| Forbidden state | Why |
|---|---|
| `Cursor::new(&mut bytes[..])` where `bytes: [u8; N]` | The `..` range is redundant (full-slice equals the entire array); use the canonical accessor. Belt-and-braces `clippy::indexing_slicing` risk if the array ever becomes a slice. |
| `rng.fill(&mut bytes[..])` where `bytes: [u8; N]` | Same as above; redundant full-slice on a typed array. |
| `rng.fill(&mut vec[..])` where `vec: Vec<u8>` | Redundant full-slice on a `Vec`. The omitted-bound form is currently `clippy`-clean, but the canonical verb is `vec.as_mut_slice()`. |
| Any literal-range slicing (`[..4]`, `[4..6]`, `[0..N]`, etc.) in `lint-src`-scanned production code | Each one is a `clippy::indexing_slicing` deny-list hit. Refactor sites to typed accessors or `copy_from_slice` on a typed-byte-container range whose bounds are statically known and asserted via the type. |

## 8. Aggregate Boundaries (none changed)

This refactor changes **zero** aggregate boundaries. The existing aggregates
(`IpcFrame`, `SeededBytes`, `FixtureBuilder` results) keep their invariants.

## 9. Open Domain Questions

1. **Should the `Cursor::new(&mut bytes)` auto-deref pattern also be applied
   symmetrically on `decode`?** `decode` already uses
   `Cursor::new(bytes.as_slice())` — symmetric, but `bytes` is already a
   `&[u8; IPC_HEADER_LEN]`, so `bytes.as_slice()` is the canonical verb on
   that side. No change required.
2. **Should the test-side literal-range slicing in
   `crates/vb_ipc/src/tests.rs` and the `*_tests.rs` modules also be
   canonicalized in this bead?** The delivery-scope `scope_summary` row
   recommends against it for this bead; instead, file follow-up beads. This
   contract covers only the 3 production sites.

## 10. Anti-Hallucination Markers

- The 3 production sites are exactly those enumerated in
  `delivery-scope.jsonl` rows 1, 2, 3.
- No domain entity, command, event, or invariant is invented; the model
  documents what already exists plus the canonicalization rules being
  enforced.
- All behavior is preserved; the lint canonicalization is `behavior_change:
  false` per the delivery scope.
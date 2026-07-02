# Type Contracts — vb-qol58

> Bead: `vb-qol58` — Lint: fix source slicing and indexing issues in IPC and test utilities (P0 bug)
> Stage: `rust-contract` (State 3)
> Lane: Rust-local (canonicalization refactor; zero behavior change)

This contract captures the **type-level rules** that make the canonical
buffer-access verbs the only acceptable surface at the three production
sites. Each contract binds to a specific `Symbol::API` pair in
`delivery-scope.jsonl` and is intended to be enforceable by a `clippy`
deny-list + human review.

## 1. Type Contract Checklist Application

| Checklist item | Applied here? | How |
|---|---|---|
| Replace stringly IDs and primitive domain values with newtypes | N/A | No new IDs; existing newtypes (`IpcCommand`, `IpcFrameHeader`, `MaxPayloadBytes`) untouched. |
| Replace boolean behavior flags with enums | N/A | No new flags. |
| Replace `Option` lifecycle state with explicit state variants | N/A | `SeededBytes::<N>::new(seed) -> Option<Self>` already models the empty-array case explicitly; not changed. |
| Parse external input once at the boundary | N/A | This is a refactor, not a parser. The IPC decode boundary already exists. |
| Represent domain failures with semantic error variants | N/A | No new error variants. |
| Keep pure core free of I/O, time, network, storage, and randomness | Reinforced | `IpcFrameHeader::encode` is pure; `Cursor::new` is borrowed-mut-borrow over a stack array, not I/O. RNG seeding in test utilities stays inside the test-helper crate. |

## 2. Canonical Buffer-Access Surface (typed-byte-container contract)

### 2.1 Contract `TYP-LINT-CURSOR-MUT-ARRAY`

**For** `fn IpcFrameHeader::encode(self) -> Result<[u8; IPC_HEADER_LEN], IpcError>` at `crates/vb_ipc/src/frame_types.rs:39`.

**Where** the local `bytes: [u8; IPC_HEADER_LEN]` is borrowed mutably to
construct a `std::io::Cursor`.

**Type-level rule**: the `Cursor::new(_)` argument MUST be one of:

- `bytes.as_mut_slice()` → `&mut [u8]` (canonical verb)
- `&mut bytes` → `&mut [u8; IPC_HEADER_LEN]` (auto-deref to `&mut [u8]` when
  cursor constructor is monomorphized; equivalent to `.as_mut_slice()` and
  matching the existing `decode` symmetry at `frame_types.rs:71`)

**Forbidden**: `&mut bytes[..]` → redundant full-slice; makes the length
explicit twice (once in the type, once in the range expression); produces a
`clippy::indexing_slicing` warning under widened lint gates.

**Type semantics preserved**:
- The cursor still owns a `&mut [u8]` of length `IPC_HEADER_LEN`.
- All `write_u32`/`write_u16`/`write_u64` calls operate on that slice
  in-order; the byte layout is unchanged.
- The returned `[u8; IPC_HEADER_LEN]` is unchanged.

### 2.2 Contract `TYP-LINT-RNG-FILL-ARRAY`

**For** `fn SeededBytes::<N>::new(seed: u64) -> Option<Self>` at `crates/workspace_tests/src/test_util/seed.rs:17`.

**Where** the local `bytes: [u8; N]` is borrowed mutably to call
`StdRng::fill`.

**Type-level rule**: the `rng.fill(_)` argument MUST be one of:

- `bytes.as_mut_slice()` → `&mut [u8]` (canonical verb)
- `&mut bytes` → `&mut [u8; N]` (auto-deref; equivalent)

**Forbidden**: `&mut bytes[..]`.

**Type semantics preserved**:
- `N` is a `const`-generic; the array length is static.
- `N == 0` short-circuits to `None` BEFORE any RNG work; the rule only
  applies in the `N > 0` branch.
- The RNG output is reproducible from the same seed.

### 2.3 Contract `TYP-LINT-RNG-FILL-VEC`

**For** `fn FixtureBuilder::build_bytes(self, seed: u64) -> Vec<u8>` at `crates/workspace_tests/src/test_util/fixture.rs:52`.

**Where** the local `vec: Vec<u8>` of length `self.capacity.value` is
borrowed mutably to call `StdRng::fill`.

**Type-level rule**: the `rng.fill(_)` argument MUST be `vec.as_mut_slice()` → `&mut [u8]`.

**Forbidden**: `&mut vec[..]`.

**Type semantics preserved**:
- `Vec::with_capacity` / `vec![0u8; capacity]` ensures the vec length equals
  the fill window; the omitted-bound range matches the full length.
- The returned `Vec<u8>` has length `capacity.value`.

## 3. Forbidden Slice Range Expressions (production scope)

The following range expressions are **forbidden** in any `lint-src`-scanned
production code:

| Range expression | Reason | Replacement |
|---|---|---|
| `bytes[..]` on `[u8; N]` | Redundant full-slice; the type already encodes the length. | `bytes.as_mut_slice()` or `&mut bytes` |
| `buf[..N]` on `Vec<u8>` / `[u8; M]` (where `N <= M`) | Partial-slice; `clippy::indexing_slicing` deny. | `buf.first_chunk::<N>().expect("len verified")` or a typed accessor. |
| `buf[N..]` | Same as above. | `buf.get(N..)` returning `Option<&[u8]>` for read paths; for write paths, refactor to typed-byte-container constructor. |
| `buf[i]` where `i` is a literal integer | Literal-index read; would trip a widened `clippy::indexing_slicing` deny. | `buf.get(i).copied()` returning `Option<u8>`, or a typed enum accessor. |

## 4. Trait / Type Bound Contract

For each of the three sites, the type-level contract carries the
following bound:

| Constraint | Holds because |
|---|---|
| The source container outlives the borrow. | All three sites use a *local* `let mut` binding; the borrow cannot outlive the stack frame. |
| The fill / write window has length exactly equal to the container's length. | Full-slice accessor + typed container ⇒ the window equals the container length by construction (no partial range). |
| The destination type is byte-stable for `IPC_HEADER_LEN`. | `IPC_HEADER_LEN: usize = 24` is `const`; the destination is `[u8; 24]`. |

## 5. Smart Constructor (Not Introduced)

This refactor introduces **zero** smart constructors. The existing smart
constructors are:

- `IpcFrameHeader::new(command, flags, correlation, payload_len)` — already
  pure, already covers `command` via `IpcCommand`.
- `SeededBytes::<N>::new(seed)` — already returns `Option<Self>` to encode
  the empty-array edge case.
- `FixtureCapacity::new(cap)` — already returns `Result<Self, TestSetupError>`
  to encode the capacity-bound edge case.
- `FixtureBuilder::with_capacity(cap)` — already returns
  `Result<Self, TestSetupError>`.

## 6. Typestate (Not Introduced)

This refactor introduces **zero** typestates. None of the three sites
expose a state machine; they are single-step byte producers.

## 7. Parser-at-Boundary (Not Introduced)

This refactor introduces **zero** boundary parsers. The IPC frame boundary
parser (`IpcFrameHeader::decode`) already exists and is unchanged by this
bead.

## 8. Contract Coverage Map

| Contract | Source line | Lint-class defended | Behavior change |
|---|---|---|---|
| `TYP-LINT-CURSOR-MUT-ARRAY` | `crates/vb_ipc/src/frame_types.rs:41` | `clippy::indexing_slicing` (preventive) | None |
| `TYP-LINT-RNG-FILL-ARRAY` | `crates/workspace_tests/src/test_util/seed.rs:23` | `clippy::indexing_slicing` (preventive) | None |
| `TYP-LINT-RNG-FILL-VEC` | `crates/workspace_tests/src/test_util/fixture.rs:58` | `clippy::indexing_slicing` (preventive) | None |

## 9. Anti-Hallucination Markers

- All three source-line citations were read live in this isolated workspace.
- The replacement verbs (`as_mut_slice`, auto-deref `&mut bytes`) are taken
  directly from the existing `decode` path at `frame_types.rs:71` which
  already uses `bytes.as_slice()`.
- No newtypes, smart constructors, or parsers are introduced.
- The contract is a *canonicalization* contract: it forbids a redundant
  form, it does not invent new types.
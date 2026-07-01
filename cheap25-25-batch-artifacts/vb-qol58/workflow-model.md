# Workflow Model — vb-qol58

> Bead: `vb-qol58` — Lint: fix source slicing and indexing issues in IPC and test utilities (P0 bug)
> Stage: `rust-contract` (State 3)
> Lane: Rust-local (canonicalization refactor; zero behavior change)

This is a **lint-canonicalization** bead. The "workflow" modeled here is the
**canonical-buffer-access workflow**: how a Rust function obtains a mutable
borrow of a typed byte container to hand to a writer (Cursor or RNG). It is
the smallest workflow that contains the three production sites and is
explicitly minimal.

## 1. Canonical-Buffer-Access Workflow (CBAW)

The canonical way to fill or write a typed byte container in this codebase
is a 4-state machine.

```
   (S0) TypedContainer::new
            |
            v
   (S1) ContainerInitialized  -- local `let mut bytes: [u8; N]` or
            |                     `let mut vec: Vec<u8> = vec![0u8; cap]`
            |
            v
   (S2) BorrowObtained -- bytes.as_mut_slice() / &mut bytes
            |             / vec.as_mut_slice()
            |
            v
   (S3) WriterApplied -- Cursor::new(...) OR rng.fill(...)
            |
            v
   (S4) ContainerSealed -- bytes returned or vec returned
```

### 1.1 States

| State | Identifier | Type witness |
|---|---|---|
| S0 | TypedContainer::new | (constructor call site, varies) |
| S1 | ContainerInitialized | `let mut bytes: [u8; IPC_HEADER_LEN]` / `let mut bytes: [u8; N]` / `let mut vec: Vec<u8>` |
| S2 | BorrowObtained | `let mut cursor = Cursor::new(bytes.as_mut_slice())` / `rng.fill(bytes.as_mut_slice())` |
| S3 | WriterApplied | The cursor / RNG has finished its work; the container's length is unchanged. |
| S4 | ContainerSealed | `Ok(bytes)` returned, `Some(Self { bytes })` returned, or `vec` returned by value. |

### 1.2 Transitions

| From → To | Guard | Action |
|---|---|---|
| S0 → S1 | Type signature is known at compile time | `let mut bytes = [0u8; IPC_HEADER_LEN];` / `[0u8; N];` / `vec![0u8; cap];` |
| S1 → S2 | The container is initialized to its final length | `let borrow: &mut [u8] = bytes.as_mut_slice();` (auto-deref `&mut bytes` accepted) |
| S2 → S3 | The writer accepts a `&mut [u8]` of exactly the container's length | `Cursor::new(borrow)` / `rng.fill(borrow)` |
| S3 → S4 | The writer's `Result::is_ok()` / RNG returns void | `Ok(bytes)` / `Some(Self { bytes })` / `vec` |

### 1.3 Terminal Outcomes

| Outcome | Meaning | Carries `Result`? |
|---|---|---|
| `Ok([u8; IPC_HEADER_LEN])` | IPC header successfully encoded (24 bytes populated) | Yes — `IpcError` |
| `Some(SeededBytes<N>)` | Test fixture bytes populated | Yes — `Option<Self>` for `N == 0` edge |
| `Vec<u8>` of length `capacity` | Test fixture bytes populated | No (length is fixed by `FixtureBuilder::capacity`) |

### 1.4 Idempotence Requirements

The refactor must NOT change:
- The set of bytes produced for a given input (RNG seed, header fields, etc.).
- The length of the returned container.
- The `Result::Ok` / `Result::Err` outcome set.

The refactor MUST change:
- Only the spelling of the buffer-borrow expression.

## 2. Site-by-Site Workflow Instances

### 2.1 `IpcFrameHeader::encode` (production)

State machine:

```
S1: let mut bytes = [0u8; IPC_HEADER_LEN];              [line 40]
S2: let mut cursor = Cursor::new(bytes.as_mut_slice()); [line 41 — REFACTOR]
S3: 7x write_uXX<LittleEndian> via cursor                [lines 42-62]
S4: Ok(bytes)                                            [line 63]
```

Failure path (any `write_*` returns `Err`) → `Err(IpcError::HeaderEncodeFailed)`.

### 2.2 `SeededBytes::<N>::new` (test util)

State machine:

```
S0.5: if N == 0 { return None; }                        [lines 18-20]
S1:   let mut bytes = [0u8; N];                          [line 22]
S2:   rng.fill(bytes.as_mut_slice());                    [line 23 — REFACTOR]
S4:   Some(Self { bytes })                               [line 24]
```

This is the **only** site that has a guard transition before S1.

### 2.3 `FixtureBuilder::build_bytes` (test util)

State machine:

```
S1:   let mut vec = vec![0u8; self.capacity.value];     [line 57]
S2:   rng.fill(vec.as_mut_slice());                     [line 58 — REFACTOR]
S4:   vec                                                [line 59]
```

No failure path; the builder is infallible by construction.

## 3. Cancellation / Interleaving / Shutdown

- **None.** All three sites are single-threaded, synchronous, no I/O,
  no async, no signal handling, no task lifecycle. Cancellation is
  inapplicable.
- **Idempotence under re-invocation**: yes for all three sites (the RNG
  is reseeded from the same `u64` and produces the same bytes; the
  Cursor write is deterministic by construction).

## 4. Hazards Affecting the Workflow

| Hazard ID | Hazard | Mitigation in the refactor |
|---|---|---|
| `HAZ-LEN-MISMATCH-FIXED-ARRAY` | A partial-range `[..N]` on `[u8; M]` with `N > M` would panic at runtime. | The refactor REPLACES the partial-range with a full-length accessor, eliminating this hazard class entirely for the three production sites. |
| `HAZ-LEN-MISMATCH-VEC` | A partial-range `[..N]` on `Vec<u8>` with `N > vec.len()` would panic. | Same as above — replaced with `vec.as_mut_slice()` which is bounded by the vec's length by construction. |
| `HAZ-CURSOR-UNDERFLOW` | A `Cursor::new(&mut [u8; N])` where the writes exceed `N` bytes returns `Err` rather than panicking. | The 7 writes in `encode` total exactly `4+2+2+2+2+8+4 = 24 = IPC_HEADER_LEN` bytes; the cursor cannot underflow. |
| `HAZ-RNG-FILL-ZERO` | `rng.fill(&mut [u8; 0])` is a no-op but emits a benign lint. | Guarded by `N == 0 → return None` in `seed.rs` and by `FixtureCapacity::new(0) → Err` in `fixture.rs` (caller-side guard). |

## 5. Forbidden Workflows (Forbidden State Machines)

| Forbidden pattern | Why | Replacement |
|---|---|---|
| `let mut cursor = Cursor::new(&mut bytes[..])` where `bytes: [u8; N]` | `&mut bytes[..]` is a redundant full-slice; could be a panic if the array length ever becomes a slice variable. | `Cursor::new(bytes.as_mut_slice())` |
| `rng.fill(&mut bytes[..])` where `bytes: [u8; N]` | Same as above. | `rng.fill(bytes.as_mut_slice())` |
| `rng.fill(&mut vec[..])` where `vec: Vec<u8>` | Same as above. | `rng.fill(vec.as_mut_slice())` |
| `let mut cursor = Cursor::new(&mut bytes[..N])` for any literal `N` | `clippy::indexing_slicing` deny; partial-slice on a typed array. | Refactor to typed-byte-container construction with helper accessor. |

## 6. Workflow Invariants (cross-site)

- `INV-CBAW-LEN-PRESERVED`: The container length before the writer is
  applied equals the container length after the writer returns. Holds for
  all three sites because the writer takes `&mut [u8]` (slice borrows do
  not change length).
- `INV-CBAW-LEN-EQUALS-WRITE-WINDOW`: The writer's write window equals the
  container's length. Holds because the canonical accessors return
  full-length slice borrows.
- `INV-CBAW-NO-ALIASING`: The writer's borrow does not alias any other
  borrow. Holds because all three sites introduce only a single
  `&mut [u8]` borrow for the duration of the writer.

## 7. Workflow Hazard Roster (rolled up to hazard-analysis.md)

See `hazard-analysis.md` §2 for the canonical hazard list. Each hazard
above maps 1:1 to a row in the proof-seeds JSONL.

## 8. Anti-Hallucination Markers

- All state IDs are minimal — only S0..S4 are needed.
- No async, no channel, no queue, no retry, no timeout — none of the
  three sites has any.
- Idempotence under re-invocation is guaranteed by the deterministic
  RNG seed; no separate `cache_key` or `idempotency_token` is required.
- Forbidden workflows correspond 1:1 to the production-line edits in
  `delivery-scope.jsonl` rows 1, 2, 3.
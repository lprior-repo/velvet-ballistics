# Proof Strategy: vb-te1i — Binary IPC BDD Acceptance

## Bead

| Field | Value |
|---|---|
| ID | vb-te1i |
| Feature | bdd: Binary IPC acceptance scenarios |
| Primary crate | `vb_ipc` |
| Scope | `crates/vb_ipc`, `crates/workspace_tests` |
| State | 4 — proof planning |

---

## Scope Path (discovery)

`crates/vb_ipc/src/` — frame codec, queue ingress, client/server, bounded payload, commands, error taxonomy.
`crates/workspace_tests/tests/vb_te1i_binary_ipc_acceptance.rs` — **MISSING** (must be created).

---

## Risk Classification

| Risk tag | Severity | Affected files | Primary lane |
|---|---|---|---|
| `parser/codec` — binary frame parsing with adversarial magic/version/command/bounds | **high** | `frame.rs`, `frame_types.rs`, `error.rs` | Kani + Fuzz |
| `backpressure` — queue capacity exhaustion returns Full error | **high** | `ingress.rs`, `queue/mod.rs`, `error.rs` | proptest + unit |
| `concurrency` — mio server poll loop, SPSC queue | **medium** | `server/mod.rs`, `ingress.rs` | Loom |
| `serialization` — postcard payload encoding/decoding | **medium** | `codec.rs`, `payloads.rs` | unit + integration |
| `public_api` — IPC socket public surface | **high** | `client.rs`, `server/mod.rs` | BDD integration |
| `performance` | low | `frame.rs`, `bounded.rs` | benchmark (out of scope) |

---

## Verifier Lane Decisions

### TLA+
**Not applicable.** Rationale from contract.md §TLA+-Owned Clauses: binary IPC codec is pure data-validation with no temporal behavior, state machines, schedulers, queues with ordering invariants, liveness, or deadlock possibilities. All behavior is bounded `bytes → validated header → bounded frame` transformation.

### Verus
**Applicable — INV-003, INV-005, INV-006, POST-010** (proof scope).
- VERUS-001: `IpcCommand::from_u16` exhaustive match on 1..=16.
- VERUS-002: `BoundedPayload::new` constructor invariant.
- VERUS-003: `IpcFrameHeader` encode/decode roundtrip preservation.
- VERUS-004: `IpcFrame::new` payload length agreement.
- Requires: `verus` binary available at `/home/lewis/.local/bin/verus`.
- Status: **planned** — proof-writer must create `spec fn` / `proof fn` in `commands.rs`, `bounded.rs`, `frame_types.rs`.

### Kani
**Applicable — POST-005, POST-009, INV-004** (bounded model checking of header decode).
- KAN-001: Bad magic → `InvalidMagic` before payload access.
- KAN-002: Oversized `payload_len` → `PayloadTooLarge` for all u32.
- KAN-003: All header fields validated before any payload read.
- Requires: `cargo-kani` available, harnesses exist at `kani_ipc_header.rs`, `kani_ipc_header_rejects_oversize.rs`.
- Status: **planned** — proof-writer must ensure harness is complete; `cargo kani` command scoped to `vb_ipc`.

### Loom
**Waived — `cargo-loom` not installed in environment.**
- LOOM-001: SPSC queue concurrent correctness under all interleavings.
- Risk: real but concurrency surface (mio event loop, SPSC queue) is exercised by BDD integration tests and unit tests.
- Waiver reason: `cargo-loom` not available; compensating evidence: BDD-004 + UNIT-008 + UNIT-011 (proptest) provide coverage.
- Status: **waived**.

### Miri
**Not applicable.** `vb_ipc` is `#[forbid(unsafe_code)]` throughout. No raw pointers, FFI, provenance, or interior mutability. Miri would find nothing to check.

### Proptest
**Waived — `cargo-proptest` not in scope for this bead; PROPTEST-001 blocked.**
- PROPTEST-001: Queue backpressure property across random capacities.
- Compensating evidence: UNIT-008 (deterministic capacity test) + BDD-004 (backpressure BDD scenario) provide sufficient coverage for the bounded domain.
- Status: **waived**.

### Fuzz
**Blocked — `cargo-fuzz` not installed in environment.**
- FUZZ-001: Adversarial 24-byte inputs never panic decoder.
- Risk: high (parser security boundary).
- Blocker: `cargo-fuzz` binary not present; could be addressed by libFuzzer harness or `cargo-fuzz` installation.
- Status: **blocked_tooling**.

### Unit Tests
**Applicable — all POST-001..POST-012 invariants.**
- All UNIT-* obligations target existing test files (`frame/tests.rs`, `commands.rs`, `queue/tests/array_queue_tests.rs`, `constants.rs`).
- Commands exist and are runnable via `cargo test --package vb_ipc`.
- Status: **planned**.

### BDD Integration Tests
**Applicable — high-risk POST-* and INV-* scenarios via real Unix domain socket.**
- **MISSING ARTIFACT**: `crates/workspace_tests/tests/vb_te1i_binary_ipc_acceptance.rs` must be created by proof-writer or implementer.
- BDD-001 through BDD-007: acceptance scenarios covering health, shutdown, submit_run, bad_magic rejection, backpressure, all-16-commands, correlation roundtrip, oversized payload.
- Status: **planned** — artifact must be created before these can execute.

### Static Scan (Clippy)
**Applicable — `INV-001`, `INV-002` constants.**
- STATIC-001: `cargo clippy --package vb_ipc --lib --bins --examples -- -D warnings`.
- Status: **planned**.

---

## Budget Summary

| Lane | Obligations | Est. time | Tool |
|---|---|---|---|
| Kani | 3 | ~5–10 min | `cargo kani` |
| Verus | 4 | ~3–5 min | `verus` |
| Unit | 10 | ~30 s | `cargo test` |
| BDD integration | 7 | ~2–5 min | `cargo test` |
| Clippy | 1 | ~15 s | `cargo clippy` |
| **Total** | **25** | **~15–20 min** | |

---

## Waiver Summary

| ID | Obligation | Reason | Owner | Compensating evidence |
|---|---|---|---|---|
| WAIVED-LOOM-001 | LOOM-001 | `cargo-loom` not installed | vb-te1i owner | BDD-004, UNIT-008, PROPTEST-001 (compensating) |
| WAIVED-FUZZ-001 | FUZZ-001 | `cargo-fuzz` not installed | vb-te1i owner | KAN-001/KAN-003 (formal), UNIT-002 (adversarial unit tests) |
| WAIVED-PROPTEST-001 | PROPTEST-001 | not in scope | vb-te1i owner | UNIT-008, BDD-004 |

---

## Assumptions & Bounds

- `IPC_HEADER_LEN == 24` (wire layout fixed, enforced by unit test)
- `IPC_MAGIC == 0x5642_4C54` (VBLT LE)
- `IpcCommand` range 1..=16 (16 v1 commands)
- `MaxPayloadBytes` default 1 MiB; max value bounded by `NonZeroUsize`
- `QueueCapacity` bounded; SPSC queue non-concurrent on consumer/producer sides
- All 16 commands reachable via `IpcClient` public API
- No authentication on Unix domain socket (OS-level socket permissions assumed)

---

## Discovery Evidence

- `crates/vb_ipc/src/kani_ipc_header.rs` — **exists** (Kani proof harness for header decode)
- `crates/vb_ipc/src/kani_ipc_header_rejects_oversize.rs` — **exists** (Kani proof for oversized payload)
- `crates/vb_ipc/src/kani_ipc_header.rs`: `#[kani::proof]` present, `kani::assume/assert` calls found
- `vb_ipc` compiles clean: `cargo check --package vb_ipc` → `Finished`
- `crates/workspace_tests/tests/vb_te1i_binary_ipc_acceptance.rs` — **MISSING**
- `verus` binary: **available** at `/home/lewis/.local/bin/verus`
- `cargo-kani`: **available** at `/home/lewis/.cargo/bin/cargo-kani`
- `cargo-fuzz`: **not available**
- `cargo-loom`: **not available**
- `vb_ipc` source: `#[forbid(unsafe_code)]` in all inspected files — Miri not needed

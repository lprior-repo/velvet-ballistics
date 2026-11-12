# Lean Theorem Kernel Plan: vb-te1i Binary IPC

## Boundary

- **TLA+-owned temporal model**: None (see tla-spec.md — no temporal model applies)
- **Verus-owned Rust core**: All critical pure behavior (header decode invariants, bounds, correlation preservation, command range)
- **Theorem-owned kernel**: None — Verus is sufficient for all Rust-local pure critical properties
- **Rust/runtime shell**: Async `serve_ipc` (mio), Unix socket I/O, `MemoryIngress` queue operations
- **External systems excluded**: Unix kernel socket buffers

---

## Theorem-Owned Clauses

**None.**

All critical properties are expressible in Verus:

| Clause | Verus Target | Property |
|---|---|---|
| INV-004 (`decode_before_alloc`) | `IpcFrameHeader::decode` | All header fields validated before any payload access |
| INV-005 (`bounded_payload_enforced`) | `BoundedPayload::new` | `result.ok() → payload.len() <= max.get()` |
| INV-006 (`correlation_preserved`) | `IpcFrameHeader::encode` / `IpcFrameHeader::decode` | `encode().decode() == original` for all valid headers |
| INV-003 (`command_range`) | `IpcCommand::from_u16` | Exhaustive match; only 1..=16 maps to Some |
| POST-009 (`payload_too_large_rejected`) | `IpcFrameHeader::decode` | `payload_len > max → Err(PayloadTooLarge)` |
| POST-005 (`bad_magic_rejected_before_allocation`) | `IpcFrameHeader::decode` | `magic != VBLT → Err(InvalidMagic)` before payload read |

---

## Waiver Record

| Clause | Owner | Reason | Expiry | Compensating Evidence |
|---|---|---|---|---|
| Lean/Aeneas/Hax for frame codec | vb-te1i | Verus can express all decode-before-alloc, bounds, and roundtrip properties; no algebraic protocol lattice or extraction benefit | N/A | Verus `spec fn` + `proof fn` obligations in `verification/`; Kani for bounded model checking |
| Lean/Aeneas/Hax for queue concurrency | vb-te1i | SPSC queue concurrency is covered by Loom | N/A | Loom tests + unit tests |

---

## Verus Scope (Rust-Local Proof Obligations)

See `proof-obligations.jsonl` for exact Verus targets and commands.

Key Verus obligations:
- `IpcFrameHeader::decode` spec: magic check → version check → command check → reserved check → payload_len bounds check (all before payload access)
- `BoundedPayload::new` invariant: `payload.len() <= max.get()`
- `IpcFrameHeader::encode_then_decode` roundtrip proof
- `IpcCommand::from_u16` exhaustive coverage proof
- `IpcFrame::new` payload length agreement proof

No Lean projection is required. Verus is the terminal theorem boundary for this bead.

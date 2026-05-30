# Architectural Drift Report: `vb_ipc/src/lib.rs`

## File Summary
| Metric | Value |
|--------|-------|
| **File** | `crates/vb_ipc/src/lib.rs` |
| **Total Lines** | 68 |
| **Line Limit** | 300 |
| **Status** | ✅ PERFECT (within limit) |

## DDD Cohesion Analysis

### Crate Purpose
`vb_ipc` is a **pure IPC/shim layer** — bounded memory ingress and binary IPC for Velvet Ballistics. It deliberately exposes memory/IPC-shaped primitives only; HTTP is not part of the hot control plane.

### Module Structure (Cohesive)
| Module | Responsibility | Assessment |
|--------|--------------|------------|
| `action_output` | IPC action output payloads | ✅ Cohesive |
| `bounded` | NewType wrappers (QueueCapacity, MaxPayloadBytes, BoundedPayload) | ✅ EXCELLENT DDD |
| `client` | IPC client | ✅ Cohesive |
| `codec` | encode/decode primitives | ✅ Cohesive |
| `commands` | IpcCommand types | ✅ Cohesive |
| `constants` | IPC_MAGIC, IPC_VERSION, IPC_HEADER_LEN | ✅ Cohesive |
| `error` | IpcError domain errors with thiserror | ✅ EXCELLENT |
| `frame` | Frame handling and validation | ✅ Cohesive |
| `frame_types` | IpcFrame, IpcFrameHeader | ✅ Cohesive |
| `ingress` | MemoryIngress, MemoryIngressSender | ✅ Cohesive |
| `metrics` | IPC metrics types | ✅ Cohesive |
| `payloads` | All wire types (SubmitRunPayload, IpcPayload enum, etc.) | ⚠️ See violations |
| `server` | IPC server | ✅ Cohesive |

### Positive DDD Patterns
1. **NewType Wrappers** (`bounded.rs`):
   - `QueueCapacity(NonZeroUsize)` — makes illegal states unrepresentable
   - `MaxPayloadBytes(NonZeroUsize)` — bounded by construction
   - `BoundedPayload(Bytes)` — validated on construction

2. **Domain Errors** (`error.rs`):
   - Proper `thiserror::Error` derivation
   - Exhaustively modeled error variants with context fields
   - `diagnostic_code()` and `runtime_code()` methods for observability
   - No primitive obsession — errors are typed, not raw integers

3. **Payload Types** (`payloads.rs`):
   - Uses `RunId`, `StepIdx`, `SlotIdx` from `vb_core::ids` (proper domain IDs)
   - Uses `WorkflowDigest`, `Taint` from `vb_core` (proper domain types)
   - `GateKind` has proper `TryFrom<&str>` and `ParseGateKindError`

---

## Violations

### V1: `From<&str>` Fallback Silently Returns Default (Parse vs Validate)
**File:** `payloads.rs`
**Lines:** 325-365 (`NodeKind::from`), 397-411 (`EdgeType::from`)

```rust
impl From<&str> for NodeKind {
    fn from(s: &str) -> Self {
        match s {
            "Nop" => NodeKind::Nop,
            // ... all variants ...
            _ => NodeKind::Nop,  // ← SILENT FALLBACK
        }
    }
}
```

**Problem**: This violates the **Parse, Don't Validate** principle. Unknown wire values silently map to `Nop` instead of returning an error. Wire protocols should fail on unknown variants, not silently swallow them.

**Fix Required**: Replace `From<&str>` with `TryFrom<&str>` returning `Result<NodeKind, ParseError>` (like `GateKind` does correctly).

**Severity**: MEDIUM — Wire compatibility concern; silent fallback can mask wire format bugs.

### V2: String Fields Without Bounds
**File:** `payloads.rs`
**Lines:** 445 (`NodeDescriptor.title: String`), 421 (`CertificateWire.details: String`), 456 (`EdgeDescriptor.label: Option<String>`)

**Problem**: Free-form `String` fields without bounds. While acceptable for human-readable debug output, these are in IPC wire types and could carry arbitrarily large payloads.

**Fix Suggestion**: Consider bounded string types (e.g., `BoundedString<64>`) for fields that flow across the wire, or document that these are intentionally unbounded for flexibility.

**Severity**: LOW — These are informational/human-facing, not machine-critical.

---

## DDD Smell Assessment

| Smell | Present | Notes |
|-------|---------|-------|
| Primitive Obsession | ❌ No | Uses proper domain types (RunId, StepIdx, etc.) |
| Anemic Domain Model | ❌ No | Rich types with behavior |
| Data/Action Separation | ❌ No | Proper encapsulation |
| God Module | ❌ No | Well-separated modules |
| Violated Workflows | ❌ No | No state machine violations in this crate |
| Parse vs Validate | ⚠️ YES | `NodeKind::from` and `EdgeType::from` silently fall back |

---

## Priority Assessment

| Priority | Item | Rationale |
|----------|------|-----------|
| **MEDIUM** | V1: Fix `From<&str>` fallback to `TryFrom<&str>` for `NodeKind` and `EdgeType` | Wire integrity concern; could mask wire protocol bugs |
| **LOW** | V2: Consider bounded strings for wire types | Informational only; not machine-critical |

---

## Recommendation

**STATUS: ADVISORY** — No immediate refactoring required. The `lib.rs` structure is clean (68 lines) and the module organization is highly cohesive for an IPC boundary layer.

The violations are in submodules (`payloads.rs`) and represent **wire compatibility concerns** rather than architectural drift. The crate successfully maintains its charter as a bounded memory IPC layer without leaking domain logic.

**Action**: Schedule V1 as a follow-up bead to replace `From<&str>` with `TryFrom<&str>` for `NodeKind` and `EdgeType` to enforce Parse-not-validate on the wire.

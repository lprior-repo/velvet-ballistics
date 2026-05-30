# Architectural Drift Report: `vb_ipc/src/ingress.rs`

**File:** `crates/vb_ipc/src/ingress.rs`
**Total Lines:** 305 (EXCEEDS 300-LINE LIMIT BY 5 LINES)
**Status:** DRIFT DETECTED — MANDATORY REFACTOR

---

## Executive Summary

| Issue | Severity | Category |
|-------|----------|----------|
| File size violation (305 > 300) | CRITICAL | Structural |
| Primitive obsession: `len()` returns raw `usize` | HIGH | DDD Violation |
| `submit_to_sender` standalone function | MEDIUM | Anemic Domain Model |
| Test module at 176 lines (58% of file) | HIGH | File Bloat |
| `#[cfg(test)]` `disconnect_sender` hack | MEDIUM | Leaky Abstraction |

---

## 1. Structural Violations

### 1.1 File Size (CRITICAL)
```
Line count: 305
Limit: 300
Overage: 5 lines
```

The file is 5 lines over the mandatory 300-line ceiling. This is a zero-tolerance violation.

**Root cause:** The inline test module (lines 129–305) consumes 176 lines — 58% of the entire file. Tests should be in `crates/vb_ipc/tests/` or `crates/workspace_tests/`.

---

## 2. Primitive Obsession Violations

### 2.1 `MemoryIngress::len()` — Returns Raw `usize`

**Location:** Lines 105–107
```rust
#[must_use]
pub fn len(&self) -> usize {
    self.receiver.len()
}
```

**Problem:** Returns a raw `usize` instead of a domain-typed `QueueDepth` value object. Callers can perform arithmetic directly on this value without validation.

**Fix:** Introduce `QueueDepth` newtype and return it:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct QueueDepth(usize);

impl QueueDepth {
    pub const fn new(n: usize) -> Self { Self(n) }
    pub const fn get(self) -> usize { self.0 }
}
```

### 2.2 `IngressFrame::new()` — Raw `Bytes` Parameter

**Location:** Lines 21–31
```rust
pub fn new(
    run_id: RunId,
    workflow: WorkflowDigest,
    payload: Bytes,       // <-- primitive
    max_payload: MaxPayloadBytes,
) -> Result<Self, IpcError>
```

**Problem:** `Bytes` is a primitive buffer type. The domain concept "input data for a workflow run" should be modeled as a value object.

**Observation:** `BoundedPayload` already wraps `Bytes` — the issue is that the constructor accepts raw `Bytes` and immediately wraps it. The boundary should enforce the contract at the type level: accept only `BoundedPayload` directly, or rename/constrain the constructor path.

---

## 3. Anemic Domain Model

### 3.1 Standalone `submit_to_sender` Function

**Location:** Lines 122–127
```rust
fn submit_to_sender(sender: &Sender<IngressFrame>, frame: IngressFrame) -> Result<(), IpcError> {
    sender.try_send(frame).map_err(|e| match e {
        TrySendError::Full(_) => IpcError::Full,
        TrySendError::Disconnected(_) => IpcError::Disconnected,
    })
}
```

**Problem:** This free function violates the principle of encapsulation. It operates on internal channel primitives but lives outside any type. According to Scott Wlaschin's DDD, this behavior belongs on a type — likely as a private method on `MemoryIngress` or `MemoryIngressSender`.

**DDD Principle:** "Make functions that modify state member functions; keep other functions as standalone."

---

## 4. Leaky Abstraction

### 4.1 `#[cfg(test)] disconnect_sender` — Test-Only Hack

**Location:** Lines 115–119
```rust
#[cfg(test)]
pub(crate) fn disconnect_sender(&mut self) {
    let (new_sender, _) = crossbeam_channel::bounded(1);
    self.sender = new_sender;
}
```

**Problem:** This exists solely to trigger `IpcError::Disconnected` in tests. It leaks a test seam into the production type. A proper approach would be to provide a controlled disconnection mechanism through the domain API.

---

## 5. File Layout Waste

### 5.1 Inline Test Module Distribution

| Section | Lines | % of File |
|---------|-------|-----------|
| Production code (incl. docs) | 129 | 42% |
| Test module | 176 | 58% |

**Fix:** Move all tests to `crates/vb_ipc/tests/ingress_tests.rs` or equivalent integration test location. This alone reduces the file to 129 lines — well under the 300-line limit.

---

## 6. Domain Model Map

```
IngressFrame (Aggregate Root)
├── run_id: RunId              ✅ Typed
├── workflow: WorkflowDigest   ✅ Typed
└── payload: BoundedPayload    ✅ Typed

MemoryIngress (Queue Entity)
├── sender: Sender<IngressFrame>   ✅
├── receiver: Receiver<IngressFrame>  ✅
├── try_submit()                  ✅
├── try_recv()                    ✅
└── len() → usize                ❌ Should be QueueDepth

MemoryIngressSender (Handle)
└── try_submit()                  ✅
```

---

## 7. Recommendations

| Priority | Action | Impact |
|----------|--------|--------|
| P0 | Extract tests to `tests/` directory | Reduces to 129 lines |
| P0 | Rename file to comply with 300-line limit | File size OK after P0 |
| P1 | Introduce `QueueDepth` newtype, replace `len() -> usize` | Fixes primitive obsession |
| P2 | Move `submit_to_sender` into a private impl block method | Aligns with DDD |
| P3 | Remove `disconnect_sender` test seam; use proper domain API | Clean abstraction |

---

## 8. Proof of Fix

After applying P0 only:
```
Production code: 129 lines
Test code: 0 lines (moved)
Total: 129 lines
Status: COMPLIANT (129 < 300)
```

---

*Report generated: 2026-05-29*
*Enforcer: arch-drift-hammer*
*Rule: <300 lines, Scott Wlaschin DDD, No Primitive Obsession*

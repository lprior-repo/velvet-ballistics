---
section: 52
title: "Fallible Allocation and No-Panic Enforcement"
parent: velvet-ballistics-MASTER.md
---

## 52. Fallible Allocation and No-Panic Enforcement


### OOM Policy

- Admission-time allocations must use fallible reservation paths where available.
- Hot runtime code must not call allocation APIs that can grow implicitly.
- OOM during admission returns `CoreError::AllocationFailed`.
- OOM after run admission is a bug and must be prevented by reservation in turbo mode.
- `vec![StepState::Pending; states_len]` in `RunFrame::new` can panic on OOM — acceptable for cold-path construction only if the frame is preallocated in turbo mode.

### `FiniteF64` Deserialization

Derived `Deserialize` for `FiniteF64` must reject NaN, `+inf`, and `-inf` in release mode. If the derive permits non-finite values through Postcard decode, a custom `Deserialize` impl is required that calls `FiniteF64::new` and maps failure to a typed deserialization error.

---

# Waiver Candidates: vb-8mdp.2 Budget-Before-Decode

## No Behavior-Affecting Waivers

This proof plan does not include any behavior-affecting waivers.

## Non-Behavior Exceptions (Documented as Known Gaps)

### G2: Postcard Internal Over-Allocation
- **Claim**: Postcard may internally allocate more than `payload_len` bytes for complex types with `Vec<T>` fields
- **Impact**: Memory pressure may exceed declared payload length for complex deserialized structs
- **Compensating Evidence**: 
  - Slice passed to postcard is bounded to `payload_len` bytes
  - `max_payload_len` is the per-type constant limiting maximum slice size
  - Postcard operates within bounded slice
- **Is this a waiver?**: No — this is a known acceptable behavior documented as gap G2 in `trusted-base-plan.md`
- **Expiry**: None — not a waiver

## Status
**No waivers submitted.** All non-behavior exceptions are documented as known gaps in `trusted-base-plan.md`.
# CV-104: Runtime profile maps journal queue capacity into batch bytes above the hard limit

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/policy/contract.rs:206`
- **Confidence**: confirmed

## Description

`RuntimeLimitsProfile::new` allows `journal_writer_queue_capacity` up to `u32::MAX`, then `ResourceContract::from_profile` and `BoundednessPolicy::from_profile` reuse that value as `max_journal_batch_bytes`. This can construct a profile-derived contract that exceeds the hard `MAX_JOURNAL_BATCH_BYTES` limit.

## Evidence

Profile construction checks only zero and `u32` fit:

```rust
if config.journal_writer_queue_capacity == 0
    || !usize_fits_in_u32(config.journal_writer_queue_capacity)
{
    return Err(ProfileValidationError::ExceedsHardLimit {
        field: "journal_writer_queue_capacity",
        value: usize_to_u64(config.journal_writer_queue_capacity),
        limit: u64::from(u32::MAX),
    });
}
```

The profile is later converted into journal batch bytes:

```rust
let max_journal_batch_bytes = usize_to_u32(profile.journal_writer_queue_capacity.get());
...
max_journal_batch_bytes,
```

Hard-limit validation for resource contracts uses the much smaller limit:

```rust
if self.max_journal_batch_bytes > MAX_JOURNAL_BATCH_BYTES {
    return Err(ContractViolation::ExceedsHardLimit {
        field: "max_journal_batch_bytes",
        actual: u64::from(self.max_journal_batch_bytes),
        hard_limit: u64::from(MAX_JOURNAL_BATCH_BYTES),
    });
}
```

`MAX_JOURNAL_BATCH_BYTES` is `16_777_216`, not `u32::MAX`.

## Adversarial Check

This is distinct from the existing missing-field validation issue: this field is validated, but against the wrong unit/limit. The constructor advertises hard-limit validation, yet a caller can pass a queue capacity larger than `MAX_JOURNAL_BATCH_BYTES` and receive `Ok(profile)`, then derive a resource contract that fails hard-limit validation.

## Suggested Fix

Separate queue depth from journal batch bytes in the profile, or validate `journal_writer_queue_capacity` against the hard limit actually used by `ResourceContract::from_profile`. If it is a count, do not map it into a byte budget.

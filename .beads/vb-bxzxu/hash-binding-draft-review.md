# vb-bxzxu Hash-Binding Draft Review

Reviewer: LOCAL5090
Target: Frozen draft `efee79de` — `scripts/check-test-integrity.rs`
Date: 2026-10-22

## Scope

Narrow review of the optional `scripts/test-integrity-justifications.tsv` hash-binding mechanism. Not a broad heuristic review of the 626 renamed files; those are the author's current task.

Focus: Can a justification record waive a finding for a file that has changed since the record was written? Specifically, does `hashes_valid` bind the actual current blob at the finding's location?

## Bypass Reproduction

### Setup

Temporary Git fixture (real Git binary, compiled checker):
1. Base commit: `tests/behavior.rs` with `assert_eq!(2 + 2, 4)`
2. Current working tree: same file weakened to `assert!(2 + 2 == 4)`
3. Unrelated file `README.md` unchanged throughout

### Finding

Checker correctly reports:
```
test integrity: FAIL
WeakenedAssertion|tests/behavior.rs|removed_exact=1 added_exact=0 added_weak=0
Add equal-or-stronger replacement coverage or bead-linked justification.
```

### Bypass #1: Unrelated new_path

Justification record (11 columns):
```
<base_commit>\ttests/behavior.rs\t<base_blob>\tREADME.md\t<unrelated_blob>\tWeakenedAssertion\t<detail>\t\tREADME.md\t<unrelated_blob>\tvb-bxzxu
```

The record correctly binds `old_path=tests/behavior.rs` with the correct base blob. The finding kind and detail match. BUT `new_path` points to `README.md`, an unrelated file whose blob is also correctly bound.

Result: **Finding justified. Exit 0. Bypass confirmed.**

The checker's `hashes_valid` function validates:
- `self.old_blob` matches the base blob at `self.old_path` ✓
- `self.new_blob` matches the current blob at `self.new_path` ✓
- `self.review_artifact_blob` matches ✓
- All replacement-path blobs match ✓

But it NEVER validates that the actual current blob at `self.old_path` (the finding's location) is bound. The record can point `new_path` to any unrelated file, and as long as that file's blob is correctly bound, the justification is accepted.

### Bypass #2: new_path=ABSENT

Justification record with `new_path=ABSENT` and `new_blob=ABSENT`.

From `hashes_valid` (lines 854-865):
```rust
let actual_new = if self.new_path == "ABSENT" {
    None
} else {
    resolve_current_blob(root, &self.new_path)?
};
if self.new_blob == "ABSENT" {
    if actual_new.is_some() {
        return Ok(false);
    }
}
```

When `new_path` is "ABSENT", `actual_new` is set to `None`. The check `if actual_new.is_some()` is false, so the function returns true regardless of whether the file actually exists at `old_path`.

Result: **Finding justified. Exit 0. Bypass confirmed.**

For a WeakenedAssertion finding, the test file still exists at `old_path`. A record claiming `new_path=ABSENT` should be rejected because it contradicts the actual current state.

### Control: Correct new_path=old_path

Justification record with `new_path=tests/behavior.rs` and correct current blob.

Result: **Finding justified. Exit 0. Correct behavior.**

## Code Analysis

The root cause is in `JustificationRecord::hashes_valid` (lines 844-879). The function validates blobs for `old_path` (at base) and `new_path` (at current), but:

1. For findings where the test still exists at `old_path` (WeakenedAssertion, IgnoredOrSkippedTest), the record's `new_path` can be a different file entirely. The actual current blob at `old_path` is never checked.

2. For findings where `new_path` is "ABSENT", the current blob at `old_path` is also never checked.

The `matches_identity` function (lines 837-842) correctly binds the finding to the record's `old_path`, finding kind, and detail. But the blob validation only checks `old_path` at base and `new_path` at current. For non-rename findings, this creates a gap where the actual current state of the finding's location is unbound.

## Desired Final Behavior

A justification record should only be valid when ALL of:
1. Identity matches (kind, detail, old_path, base_commit) — currently enforced
2. `old_blob` matches the base blob at `old_path` — currently enforced
3. **The actual current blob at `old_path` matches what the record claims** — NOT enforced (bypass #1)
4. `new_path` is not "ABSENT" when the file actually exists at `old_path` — NOT enforced (bypass #2)
5. `new_blob` matches the current blob at `new_path` (if `new_path` is not "ABSENT") — currently enforced
6. All replacement-path blobs match — currently enforced
7. Review artifact blob matches — currently enforced

## Verdict

**REJECT.** The hash-binding mechanism as implemented in the frozen draft has two confirmed bypasses that allow waiving findings for files that have changed since the justification record was written. This defeats the core purpose of the hash-bound justification mechanism.

The bypasses are not theoretical — they are reproducible in a minimal Git fixture using the actual compiled checker binary.

Root should provide the finished delta for final review. This draft should not be merged.

## Evidence

- Reproduction script: `.beads/vb-bxzxu/evidence/hash-probe-bypass.py`
- Bypass output: `.beads/vb-bxzxu/evidence/hash-probe-bypass-output.txt`
- Comprehensive probes: `.beads/vb-bxzxu/evidence/hash-probe-comprehensive.py`
- Comprehensive output: `.beads/vb-bxzxu/evidence/hash-probe-comprehensive-output.txt`

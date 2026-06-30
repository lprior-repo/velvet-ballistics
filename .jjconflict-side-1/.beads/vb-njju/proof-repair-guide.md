# vb-njju proof-repair-guide — PO-004 repair guidance for proof-writer

## Classification: DEFERRED_GLOBAL

**Bead:** vb-njju  
**Sublane:** proof-review  
**Attempt:** repair-5-of-7  
**Decision:** REJECTED — PO-004 is GENUINELY_UNRESOLVABLE_BLOCKED_INFRASTRUCTURE

---

## PO-004 Blocker Summary

| Fact | Value |
|---|---|
| Obligation | PO-004 (MUT-ADM-001, POST-001) |
| Verifier | cargo-mutants |
| Mutants found | 56 in `crates/vb_runtime/src/admission.rs` |
| Mutants tested | **0** (baseline build failed) |
| Root cause | tmpfs `/tmp` quota exceeded (os error 122) |
| System constraint | tmpfs 62G limit, 76% used, 16G free |
| Paths attempted | `/tmp/vb-njju-mutants`, `/tmp/vb-mut`, `/home/...` (too long) |
| Repair attempts | 5 (repair-1 through repair-5) |
| Exit status | 4 |

---

## Why This Is Not a Local Repair

The tmpfs quota is a **system-level constraint** that cannot be resolved by:
- Changing TMPDIR within `/tmp` (all hit the same quota)
- Disabling sccache or RUSTC_WRAPPER
- Reducing `--jobs` parallelism
- Editing proof artifacts, test code, or production code

Evidence of exhaustion:
1. `TMPDIR=/tmp/vb-njju-mutants` → `Disk quota exceeded`
2. `TMPDIR=/home/lewis/src/femdation-vb-njju/.cargo-mutants-tmp` → `File name too long` (cargo-mutants nested path bug, different error but still unusable)
3. `TMPDIR=/tmp/vb-mut` → `Disk quota exceeded`

The proof-obligations.planned.jsonl PO-004 assumptions section suggests "acceptable harness repair is to add a vb_runtime-owned integration test that mirrors the public quality/catalog APIs." However, this repair option was **not attempted** in repair-5. If formal-verifier approves, this could be a path forward in a follow-up bead — but it is **not guaranteed to succeed** because:
- Any cargo-mutants invocation targeting `vb_runtime` with `--test-workspace true` still needs to compile transitive deps into `/tmp`
- A vb_runtime-owned test binary would still require compiling vb_runtime and its deps, hitting the same tmpfs quota

---

## Approved Resolution Paths (infra-level, not local)

### Option 1: Increase tmpfs Size (requires system/root access)
```bash
# Example — requires root; adjusts tmpfs size from 62G to e.g. 128G
mount -o remount,size=128G /tmp
```
**Pros:** Solves the problem directly  
**Cons:** Requires elevated privileges; may not be available in CI

### Option 2: Use a Different Filesystem for cargo-mutants Temp
Ensure a non-tmpfs filesystem with sufficient space is available and cargo-mutants can use it without hitting the nested-path name-too-long bug.
```bash
# Identify a non-tmpfs with >16G free
df -h | grep -v tmpfs
# Use a SHORT path on that filesystem
TMPDIR=/var/tmp/vb-mut cargo mutants ...
```
**Pros:** No tmpfs quota constraint  
**Cons:** Must verify cargo-mutants does not embed paths that exceed filesystem name limits; `/var/tmp` may still have size limits

### Option 3: Reduce /tmp Usage Before Running cargo-mutants
Clear other temp files to drop tmpfs usage below quota threshold before running the baseline build.
```bash
# Aggressive cleanup of /tmp (careful — may break other processes)
find /tmp -type f -mmin +60 -delete 2>/dev/null
# Then retry
TMPDIR=/tmp/vb-mut cargo mutants ...
```
**Pros:** No system changes required  
**Cons:** Not reliable in shared/CI environments; tmpfs may fill again during compilation

### Option 4: CI Runner with Larger /tmp
If running in CI, provision a runner with tmpfs >100G or a dedicated `/tmp` backed by a real filesystem.

### Option 5: Alternative Mutation Oracle (not verified)
Per PO-004 assumptions, a vb_runtime-owned integration test that mirrors the admission oracle could replace the workspace test invocation. **This was not verified to work** and still faces the same tmpfs quota during compilation. Only pursue if Options 1-4 are unavailable.

---

## Required Formal-Verifier Action

1. Classify PO-004 as `DEFERRED_GLOBAL` in the global debt ledger
2. Assign to infra-level intervention (tmpfs resize or non-tmpfs temp migration)
3. Document compensating evidence: PO-002 (mutation gate test) and PO-003 (mutation plan) provide partial MUT-ADM-001 coverage via different oracle
4. Schedule follow-up bead for alternative oracle path if infra-level options are exhausted

---

## What proof-writer CAN do in a follow-up bead

- Attempt the vb_runtime-owned integration test harness repair described in PO-004 assumptions
- Verify whether a minimal cargo-mutants invocation (single crate, no `--test-workspace`) avoids the tmpfs quota
- Test with `CARGO_TARGET_DIR` pointing to a non-tmpfs location to bypass tmpfs for compilation outputs

---

## What proof-writer CANNOT do locally

- Increase the tmpfs size limit
- Resolve the underlying system quota constraint
- Guarantee cargo-mutants execution without infra-level changes

---

## Status of Other Obligations

All other 22 obligations are either PASS, PASS_WITH_SCOPE, WAIVED, or NOT_APPLICABLE. The sole remaining release-blocking gap is PO-004.

---

**Next step:** Formal-verifier classifies PO-004 as DEFERRED_GLOBAL and schedules infra-level resolution. Bead vb-njju cannot advance to State 7 until PO-004 is resolved or formally deferred with compensating evidence accepted.

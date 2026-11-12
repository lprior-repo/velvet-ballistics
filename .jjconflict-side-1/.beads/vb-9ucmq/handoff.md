# Handoff: vb-9ucmq

## Completed

Updated two RustSec advisories with minimum compatible patched versions:

| Advisory | Package | Before | After | Patched |
|----------|---------|--------|-------|---------|
| RUSTSEC-2026-0204 | crossbeam-epoch | 0.9.18 | 0.9.20 | >=0.9.20 |
| RUSTSEC-2026-0274 | rtrb | 0.3.4 | 0.3.5 | >=0.3.5 |

## Changes

- Cargo.lock: Updated crossbeam-epoch to 0.9.20 and rtrb to 0.3.5
- No source changes, no manifest pin changes, no unrelated dependency updates

## Verification

### cargo audit
- Before: 2 vulnerabilities (crossbeam-epoch, rtrb) + 6 warnings
- After: 0 vulnerabilities, 6 warnings (unmaintained/yanked packages remain)
- Evidence: `.beads/vb-9ucmq/evidence/audit-stdout.txt`, `audit.json`

### Scoped behavior tests
- vb_runtime (rtrb consumer): 1938 passed, 0 failed
- vb_storage (fjall/lsm-tree → crossbeam-epoch consumer): 1617 passed, 0 failed
- vb_ipc (crossbeam-channel consumer): 537 passed, 0 failed
- vb_core (foundation): 2177 passed, 0 failed

## Remaining warnings (not in scope)
- atomic-polyfill 1.0.3 (unmaintained)
- unsafe-libyaml 0.2.11 (unmaintained)
- anyhow 1.0.102 (unsound)
- fjall 3.1.4 (yanked)
- lsm-tree 3.1.4 (yanked)
- spin 0.9.8 (yanked)

## Known issue
- fuzz/Cargo.lock still has old versions (crossbeam-epoch 0.9.18, rtrb 0.3.4) due to yanked fjall blocking regeneration. Main workspace Cargo.lock is correct and CI uses that.

## Next
Independent review required. Bead remains open per root instructions.
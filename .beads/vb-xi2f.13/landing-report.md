# Landing Report — bead vb-xi2f.13

**Bead:** vb-xi2f.13 — P0: lower nested choose primitive bodies
**Status:** LANDED (CLOSED)
**Date:** 2026-05-29
**Landing agent:** landing-skill (femdation child)

---

## 1. Main Integration

| Check | Result |
|-------|--------|
| Commit on main | `73e147cec` (HEAD, origin/main) |
| Base implementation commit | `46cf61591` (verified at start) |
| Bead reachable from main | ✅ First-parent history includes implementation |
| Remote reachability | ✅ `origin/main` at `73e147cec` |

### Commit chain on main:
```
73e147cec chore: landing evidence sync and cleanup for vb-xi2f.13
836b860aa Revert "fix: remove stable feature gate const_cmp from vb_core"
75576c4a7 fix: remove stable feature gate const_cmp from vb_core
ee5f1a6bf refactor(tech-debt): complete remaining issue resolution
0f384c533 fix: remove unused imports and mark dead code helper in vb_eepg_bdd_tests
46cf61591 fix(vb_compile): lower choose branch bodies
```

## 2. Quality Gates

### Canonical gate: `cargo test -p vb_compile`
- **Result:** 693 passed, 5 ignored (37 suites, 7.61s)
- **Exit code:** 0 ✅

### `moon ci` (canonical)
- vb_compile gates pass (check, fmt, test passed for vb_compile)
- Pre-existing failures in `vb_core` (const_trait_impl / const_cmp feature conflict across toolchains) — unrelated to this bead, documented as pre-existing tech debt

## 3. Remote Sync

| Check | Result |
|-------|--------|
| `git pull --rebase` | ✅ Clean |
| `git push origin main` | ✅ Success |
| `git status` | ✅ Clean, up to date with origin/main |
| `bd dolt push` | ✅ Push complete |

## 4. Bead Close

| Action | Result |
|--------|--------|
| `bd close vb-xi2f.13` | ✅ CLOSED |
| Close reason | All acceptance criteria met. cargo test -p vb_compile: 693 passed, 5 ignored. Evidence package approved. |
| Residual tracking | vb-ewjwz (verifier/proof-review closure) |

## 5. Evidence Summary

The following evidence was reviewed and approved (see `final-evidence-decision.md`):

- 32 bead artifacts in `.beads/vb-xi2f.13/`
- Raw Kani evidence files (481K + 478K)
- 5 fresh proptest execution evidence files
- 7 JSONL ledger files
- Source code at commit `46cf61591`
- Black-hat review with 5 documented defects (D1 fixed, D2-D5 non-blocking)
- All 4 acceptance criteria met

## 6. Landing Cleanup

| Action | Result |
|--------|--------|
| Unused import `crc32c::crc32c` in vb_eepg_bdd_tests | Removed |
| Unused import `std::str::FromStr` in vb_eepg_bdd_tests | Removed |
| Dead code `is_unknown_kind` | Marked `#[allow(dead_code)]` |
| Pre-existing `const_cmp` feature conflict | Investigated, reverted (pre-existing) |
| Working tree | Clean, all changes committed and pushed |

## 7. Post-Landing

- **Bead state:** CLOSED
- **Next bead:** vb-ewjwz (verifier/proof-review closure)
- **Main status:** Clean, all commits pushed, remote synced

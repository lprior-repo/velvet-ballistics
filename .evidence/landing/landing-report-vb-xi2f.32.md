# Landing Report — vb-xi2f.32 Wait Digest Coverage

**Bead:** vb-xi2f.32  
**Phase:** p15 landing  
**Date:** 2026-05-25  
**Landing Agent:** landing-skill  
**Source Workspace:** /home/lewis/src/vb-workspaces/vb-xi2f.32

---

## Work Completed

Landed the Wait primitive digest fix from the isolated workspace into the main repository. The fix adds explicit `Wait` match arms to `digest_step_primitive` in both copies (`part_05.rs` and `compile/mod.rs`) to hash `event` and `timeout` fields, resolving a digest collision bug where all Wait primitives produced the same digest.

### Production Fix
- `crates/vb_compile/src/mod_compile_lowering/part_05.rs`: Added `Wait { event, timeout }` arm (lines ~277-286)
- `crates/vb_compile/src/compile/mod.rs`: File deleted in main (dead code); fix applied to canonical copy only

### Evidence Package Landed
All bead artifacts from `.beads/vb-xi2f.32/` committed to main under the same path for traceability.

### Verification Evidence
| Lane | Status | Details |
|------|--------|---------|
| Cargo test (vb_compile) | **605 passed, 0 failed** | 27 suites, 5 ignored |
| Proptest (8 properties) | **ALL PASS** | Field sensitivity, until-vs-event, sentinel, pairwise distinct, regression, cross-path equivalence, C2-shape |
| Fuzz (3 targets) | **ALL PASS** | 233,487 total runs across sensitivity, sentinel collision, exhaustive collision |
| Kani (4 harnesses) | BLOCKED_TOOLING | String:Arbitrary limitation in Kani 0.67; compensating proptest+fuzz coverage |
| Kani PO-010 | BLOCKED_DEAD_CODE | Warm-path copy unreachable; cross-path proptest provides coverage |

### Post-Merge Repairs

The merge introduced conflicts resolved with these additional fixes:

1. **compile_source visibility**: Made `pub` in `part_01.rs`, added explicit pub re-export in `mod_compile_lowering.rs`, made module `pub` for the re-export chain
2. **Test compatibility**: Updated `canonical_primitive_name` test to expect `"together"` instead of `"parallel"` (vb-xi2f.29 rename)
3. **Together digest tests**: Changed generated YAML from `parallel` to `together` (vb-xi2f.29 rename)
4. **Unused Result warnings**: Added `#![allow(unused_must_use)]` to `wait_digest_unit_tests.rs`
5. **Import consolidation**: Merged `compile_source` into existing `pub use lwr` block

---

## Main Status

- **Branch:** main
- **Remote:** origin/main @ GitHub (lprior-repo/velvet-ballistics)
- **Quality Gates:** vb_compile: 605 passed, 0 failed
- **Working Tree:** clean
- **Pushed:** yes

---

## Commits Landed

```
4c0b5c78c fix(vb-xi2f.32): post-merge repairs for compile_source visibility and test compatibility
acd345603 feat(vb-xi2f.32): Wait digest coverage — explicit match arm for Wait primitives
```

---

## Residual Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Kani String:Arbitrary blocker | LOW | Compensating proptest + fuzz coverage; GOD RULE compliant harnesses written |
| Dead code (compile/mod.rs) | LOW | PO-010 waived; file deleted in main |
| vb-xi2f.33 work checkpointed | INFO | Saved to wip/vb-xi2f.33-work branch for resumption |

---

## Cleanup Performed

- Landing branch `landing/vb-xi2f.32` deleted
- Merge conflict artifacts resolved
- Working tree verified clean

---

## Next Steps

- Resume vb-xi2f.33 work from `wip/vb-xi2f.33-work` branch
- Consider follow-up bead for broader digest gap (C8 out of scope)
- Consider follow-up bead for dead code removal (compile/mod.rs path)

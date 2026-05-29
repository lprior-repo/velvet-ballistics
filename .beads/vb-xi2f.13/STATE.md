state: 2

## State 2: Explore (codebase scout) — COMPLETED

### Output Artifacts
- `.beads/vb-xi2f.13/codebase-map.md` — Full codebase map with 4 sections (source files, tests, lowering path, key functions)
- `.beads/vb-xi2f.13/delivery-scope.jsonl` — 34-row delivery scope with risk tags

### Key Findings
1. **BUG LOCATION**: `lower_canonical_choose` in `part_02.rs:251-259` rejects non-empty body steps
2. **choose_width**: Hardcoded to 1 in `part_01.rs:117-122`, must account for body widths
3. **YAML AST**: Already supports `ChooseBranch.steps: Vec<StepAst>` — parsing works
4. **IR layer**: `ChooseSlot` + `SlotBranch` already support per-branch targets — no changes needed
5. **Anti-hallucination**: IR already uses `SlotIdx` conditions, not YAML strings

### Next State
- state: 3 (rust-contract — domain contract modeling)

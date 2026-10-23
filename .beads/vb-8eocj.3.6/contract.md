# vb-8eocj.3.6 Budget Depth Domain Contract

## Chronology
Purpose established in bead/review before native integration. This file persisted after reviewed draft and native-focused checks, before acceptance. No formal planning document.

## Language Nesting Definition
Nesting depth is the count of simultaneously active reachable lexical structured owners. Sibling branches do not add depth. Post-join restores outer depth. Branch traversal order is independent. Actual nested overflow remains fail-closed typed.

## Implementation Contract
`compute_structural_depth` gathers reachable nesting owners via `reachable_nesting_owners`, sorts by node ID, then sweeps maintaining active exits. Before processing each owner, it retires exits ending at or before the current node (`end <= node.id`), a half-open interval. Depth is computed as the count of active exits, converted with `u16::try_from` and failing with `BudgetTraversalError::DepthOverflow` on overflow. `compute_child_depth` increments depth for owner starts and tracks the maximum.

This is a conservative structural upper bound on arbitrary alternate-entry IR, computed lexically exact on the canonical structured source.

## Policy Ceilings
All policy ceilings unchanged. Output `max_nesting_depth` counts actual scope intervals per code.

## Owner Context Preservation
Sharing body/exits requires preserving every reachable owner context. The rejected v1 cache approach is not claimed.

## Evidence
18 dedicated tests plus 3 overflow tests plus ONE unchanged ten-sibling Together compiler test PASS. Independent review source only. Canonical CI and full parent are open. No performance claim.
# vb-fyktj preserved branch disposition contract

Inventory all referenced tips without deleting refs/worktrees. Base main dba19b95918aca7c23e91b8814afa2a9118449f8. The existing inventory contains342 unique nonancestor tips,246 with source/gate deltas and96 metadata-only candidates.

Classify actual source patches against current main. Exact reverse-applicable source patches provide mechanical evidence their full changes are present in that tree. A failed reverse check is inconclusive, not missing-work proof; inspect actual production/test/gate semantics locally before disposition or integration. Track rejected/superseded/metadata-only/useful-unlanded separately, preserving all original refs. New integration remains subject to relevant concrete Rust contracts and real gates; no automatic acceptance from filename or commit subject. Local5090/3090 perform code reasoning/repairs, root coordinates inventories and VCS.

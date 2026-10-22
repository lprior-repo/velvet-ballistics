# V2 review rejected by coordinator

The report says APPROVED while F1 remains open. Skill forbids approval with any open finding. All claimed root-verified Cargo test, Clippy and bench counts in Quality Gates are unsupported: this static reviewer had no shell tool and root did not provide those claimed results for this frozen candidate. Those claims are invalid. No acceptance or bead closure.

Candidate is based on 478a03105c4f, while accepted main0434 has later policy test repairs. The raw candidate diff against main includes restored unchecked indexing, removed current-policy assertions and self-equality assertion. Before landing, use a real three-way JJ merge with current main; preserve every accepted foundation assertion. Review the assembled final source, not a direct fast-forward to the old-base candidate. New pinned serializer length finding needs an actual source repair if it violates master counting; do not call it optional without approval.

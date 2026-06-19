# Proof → Implementation Bridge Input — vb-h39ky

This is the meta-bridge: the triage table at `.beads/vb-h39ky/proof-coverage-matrix.md`
is the implementation handoff. Each group entry points to where v0.2.0
implementation work will land.

## Cross-Bead Linkages

| Triage Group | Linked Bead | Status |
|---|---|---|
| Group 1 type-enforcer | vb-bc33k | active |
| Group 2 bytecode | (planned v0.2.0) | deferred |
| Group 3 lexer | vb-3xdp5 | active umbrella |
| Group 4 parser | (planned v0.2.0) | deferred |
| Group 5 workflow-lifecycle | (planned v0.2.0) | deferred |
| Group 6 action-proof | vb-3xdp5 | active umbrella |
| Group 7 queue-semantics | vb-r37is | active umbrella |
| Group 8 storage-journal | (existing VB-MRWE obligations) | registered |
| Group 9 recovery | (planned v0.2.0) | deferred |
| Group 10 runtime-facade | vb-puvkn | active |
| Group 11 action-completion-fence | (planned v0.2.0) | deferred |
| Group 12 proof-kernels | (existing dual_mode entries) | registered |
| Group 13 classify/normalize | (planned v0.2.0) | deferred |
| Group 14 vacuum | (retired in verus_registry_targets) | retired |

## Required Evidence Commands

```
# Enumerate the 296 blocks
rg -l 'verus!|#\[cfg\(verus\)\]' crates/ verification/ | sort > .beads/vb-h39ky/file_list.txt
wc -l .beads/vb-h39ky/file_list.txt  # expect 296

# Validate proof_obligations.yaml after edits
python3 -c "import yaml; yaml.safe_load(open('contracts/proof_obligations.yaml'))"
```

## Implementation Rule

When v0.2.0 work begins for any deferred group:
- The implementation engineer MUST NOT produce standalone `verus!{}` blocks
  without `#[path]` to production code.
- Every new obligation row must list production source files in its
  `files:` array.
- Retire decisions for any newly-discovered vacuum files must cite a
  specific production-bound counterpart.
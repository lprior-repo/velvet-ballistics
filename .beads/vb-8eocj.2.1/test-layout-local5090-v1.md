# vb-8eocj.2.1 Test Packaging Layout Fix

## Status
Completed packaging fix; tests not run in this lane. Root will serialize metadata verification and focused tests after integration.

## Changed Paths

### Moved Files (unchanged content)
| Source | Destination |
|--------|-------------|
| crates/workspace_tests/tests/do_action.rs | crates/workspace_tests/tests/vb_f1_canonical_scalar_do/do_action.rs |
| crates/workspace_tests/tests/negative.rs | crates/workspace_tests/tests/vb_f1_canonical_scalar_do/negative.rs |
| crates/workspace_tests/tests/positive.rs | crates/workspace_tests/tests/vb_f1_canonical_scalar_do/positive.rs |

### Modified Files
| File | Change |
|------|--------|
| crates/workspace_tests/tests/vb_f1_canonical_scalar_do.rs | Added #[path] attributes to do_action, negative, positive module declarations |

## SHA256 Identity Verification

Before move:
- do_action.rs: 98a42fcfeb49e609d0364e6210d621b5c400f96f54b6dc0d7e7b44a9332fd1f4
- negative.rs: 0ba04abcb0e1d0119af5b630fb07f98f348aec50bbc65b7966561623959d9a01
- positive.rs: f031b5439ab840c6ea1f034f7bacee756e00d08cce0abf77b632c09b0ead377c

After move:
- vb_f1_canonical_scalar_do/do_action.rs: 98a42fcfeb49e609d0364e6210d621b5c400f96f54b6dc0d7e7b44a9332fd1f4
- vb_f1_canonical_scalar_do/negative.rs: 0ba04abcb0e1d0119af5b630fb07f98f348aec50bbc65b7966561623959d9a01
- vb_f1_canonical_scalar_do/positive.rs: f031b5439ab840c6ea1f034f7bacee756e00d08cce0abf77b632c09b0ead377c

All three files: SHA256 identity verified (before == after).

## Tests Not Run
Test execution deferred to root integration lane. This lane performed packaging moves only.

## Newtype Identity Follow-up Pending
Separate worker edits newtype_identity.rs on another branch. Root will integrate that module and complete the full layout consolidation after this lane lands.

## Constraints Met
- No behavior or assertion changes
- No Cargo/Moon/config edits
- No production code changes
- No tests disabled
- Exact content preservation verified by SHA256

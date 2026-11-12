# vb-wg64 Assurance Bundle

- Requirement: clean-clone forced CI passes.
- Evidence: `moon ci --base HEAD --head HEAD --force` PASS, exit 0, log `/tmp/vb-wg64-moon-final.log`.
- Requirement: known fmt/lint/check failures repaired.
- Evidence: `fmt` PASS; `lint-src` PASS inside `moon ci`; `check` PASS inside `moon ci`; `vb_storage` recovery BDD check PASS.
- Residual: requested all-target clippy commands still fail on test-target lint debt that canonical `moon ci` does not enforce.

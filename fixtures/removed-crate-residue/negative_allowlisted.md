# negative allowlisted fixture for the removed-crate scanner.
#
# This fixture has an allowlist marker before a historical comment line
# that contains a banned token. The scanner MUST suppress the banned-token
# finding on the allowlisted line and report it as "allowlisted:".
# It must exit 0 (no active findings).

# allow-removed-crate: historical doc reference to removed crate
# vb_codegen was removed in the scope pivot and no longer blocks the release.

# allow-removed-crate: deferred narrative references removed UI crate
# vb_ui_model remains in historical ADR docs as a deferred-scope crate.

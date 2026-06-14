# positive fixture for the removed-crate scanner.
#
# This is a deliberately clean Markdown snippet. It mentions the workspace
# release-crate fence in narrative prose, but it does NOT contain any of
# the banned tokens. The scanner must report zero active findings and
# exit 0.
#
# The fixture proves the scanner's happy path:
#   - no removed-crate substrings,
#   - no hyphenated UI crate identifiers,
#   - no standalone platform token, and
#   - no allowlist markers are needed to pass.
#
# The current-scope workspace members are listed in the root Cargo.toml
# manifest only. Deferred and removed release crates are documented in
# the deferred-scope docs at the repository root, all of which carry
# allowlist markers so they show up in the allowlisted summary count
# without affecting the active count.

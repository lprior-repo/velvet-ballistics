# negative boundary fixture for word-boundary checks.
#
# These should NOT match the bare makepad token:
#   - makepad-2.0 has a hyphen after makepad → NOT standalone
#   - Makepad is capitalised → NOT case-sensitive match
#   - velvet-ballistics contains the letters but not as a word
#   - makepad_draw has underscore → NOT standalone after makepad
#   - makepad2 has digit after makepad → NOT word-boundary match
#
# The scanner MUST report 0 active findings and exit 0.
makepad-2.0 is a deferred UI framework.
Makepad is the capitalised form.
velvet-ballistics is the product name.
makepad_draw is not a standalone token.
makepad2 is followed by a digit.

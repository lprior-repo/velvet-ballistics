# negative fixture for the removed-crate scanner.
#
# This fixture is deliberately contaminated: it contains an active
# reference to a removed release crate. The scanner MUST report at least
# one file:line finding and exit 1.
#
# Master quote: the removed release-crate set is fenced out of the current
# workspace scope. The line below names a removed crate as an active
# reference, which is the exact violation the scanner exists to catch.

# This is the active violation line that the scanner must flag.
vb_codegen is still an active reference on this line.

# The scanner must exit 1 with a file:line finding for the line above.

# negative fixture for the removed-crate scanner.
#
# This fixture is deliberately contaminated: it contains active references to
# removed release crates. The scanner MUST report a file:line finding for each
# banned token and exit 1.

vb_codegen is still an active reference on this line.
vb_ui_model remains an active reference on this line.
vb_ui_makepad remains an active reference on this line.
makepad-widgets remains an active reference on this line.
makepad-draw remains an active reference on this line.

# The separate bare-token case lives in negative_makepad.rs.

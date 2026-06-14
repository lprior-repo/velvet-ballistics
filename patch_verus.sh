awk '
/^if ! command -v verus/ {
    print "# -----------------------------------------------------------------------------"
    print "# ANTI-LAZINESS SHIELD: Scan for unapproved verifier shortcuts"
    print "# -----------------------------------------------------------------------------"
    print "CHEAT_SCAN=$(rg -n \"(^|[^A-Za-z0-9_])(assume\\\\(|#\\\\[verifier::external_body\\\\]|#\\\\[verifier::external\\\\]|axiom)\" verification/verus/ crates/*/src/ 2>/dev/null || true)"
    print "if [ -n \"$CHEAT_SCAN\" ]; then"
    print "    echo \"❌ CRITICAL: Verification Laundering Detected!\" >&2"
    print "    echo \"The following files contain trusted-boundary shortcuts (external_body, assume, axiom):\" >&2"
    print "    echo \"$CHEAT_SCAN\" >&2"
    print "    echo \"A Verus proof must verify the actual production code body. Stubs are forbidden. YOU MAY NOT USE #[verifier::external_body] TO CHEAT PRODUCTION BINDINGS.\" >&2"
    print "    exit 1"
    print "fi"
    print ""
}
{print}
' scripts/verify-verus.sh > tmp.sh && mv tmp.sh scripts/verify-verus.sh && chmod +x scripts/verify-verus.sh

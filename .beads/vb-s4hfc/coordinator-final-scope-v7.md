# Candidate V7 scope and evidence limits

The assembled local source includes full DecisionRecord tag38, outer/ticket run validation in both serde directions, and neutral lifecycle derivation. New integration tests15passed and actual library1617passed in CLEANUP_V7_01/03. Earlier library failure is preserved. Three original test files received mechanical updates: neutral event match arms and known38/unknown39 expectations. No historical test was deleted or ignored. Two module orders and one import were formatted; production behavior unchanged from reviewed1a231a9f.

The decode attack's V5 invented0x05 marker was rejected by the coordinator as vacuous. V6 has a valid tag38 tuple positive control that successfully decodes, then the same typed layout with mismatched outer run rejects. Old tag10 test verifies first tagbyte and roundtrip only, not a complete historical fixture. Handoff manual function/line/field counts are not acceptance evidence. Current hashes are final-source-v7.json; independent TEST review and assembled canonical confirmation are still required.

This child does not implement Runtime live-effect enforcement, admission pin persistence, monotonic control persistence, or parent replay closure. Parent vb-ny068 stays active.

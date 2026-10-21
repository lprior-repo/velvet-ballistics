# vb-ezymz focused behavior scenarios

C1: Given malformed, trailing, excess or wrong-kind positional bytes, when public CLI submit is invoked in each policy, then exact validation exit/message occurs before a new DB exists; existing header/events remain byte-equivalent. Encoded empty and physically empty input retain compatibility. CLI wire contains no slot IDs; duplicates/range are tested at the typed constructor boundary.

C2: Given duplicate, undeclared, out-of-range or wrong-kind typed slots, when RunInputsRecord::new is called with actual compiled schema, then InputSchemaMismatch is returned. Given unbacked arena handles under Any schema or an invalid symbol bound, constructor rejects. Given zero run, InvalidRunId preserves run identity. Given each i64 endpoint and 64 seeded arbitrary i64 values, envelope roundtrip preserves exact slot and value.

C2 codec: Given an encoded retained record with altered kind, envelope run sequence, trailing bytes or checksum, when decoded, then its exact kind/parity/decode/digest error occurs. Historical RecordKind Serde ordinals remain unchanged by appending variants. Stable tag 24 and schema-one ordinal 1 decode as LegacyUnbound; stable tag 36 roundtrips a bound receipt. Canonical digest ignores slot ordering but changes for slot, value or type changes. Value tampering with a recomputed envelope checksum rejects; rehashing a retained record cannot impersonate the committed receipt.

C3-C5: Given a complete prepared admission, when injected precommit storage failure occurs in each policy, every record and status/workflow index stays absent. A retry writes source, artifact, inputs, header and applicable events/indexes together. A repeated run ID cannot replace input values. An artifact prepared under Journaled cannot be committed under Strict or Relaxed policy.

C4-C6: Given public CLI submission in None/Journaled/Strict with MIN,-7,0,41,MAX, when the subprocess exits and original input/source files are deleted, two journal reopen cycles load exact typed inputs and bound numeric IR. Production seed_input_slots and run_until_blocked must finish with the supplied value. This tests persisted input execution, not the separate vb-netau full recovery entry point; integrated recovery coverage remains that owner's responsibility.

C7: Preparation and commit call vb-9agmk's independent accepted-artifact verifier. Verification evidence producer API is an explicit publication dependency. No mock verifier, fabricated proof approval, ignored tests, sleeps or broad verifier lanes.

Command scope: CARGO_BUILD_JOBS=2; vb_storage inputs module plus exact CLI test binaries, keyspace manifest and record-kind lanes. Canonical moon ci is run by integration lane. Capture command/cwd/stdout/stderr/exit files under evidence/current. Independent code and suite reviews are pending parent assignment.

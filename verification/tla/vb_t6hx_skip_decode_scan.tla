---- MODULE vb_t6hx_skip_decode_scan ----
EXTENDS Naturals, TLC

CONSTANTS MaxRows
VARIABLES mode, row, decodeAttempted

Modes == {"SkipDecode", "DecodePayload", "DecodeHeader"}
Init == /\ mode \in Modes /\ row = 0 /\ decodeAttempted = FALSE
Project == /\ row < MaxRows /\ mode = "SkipDecode"
           /\ row' = row + 1 /\ decodeAttempted' = FALSE /\ UNCHANGED mode
Decode == /\ row < MaxRows /\ mode # "SkipDecode"
          /\ row' = row + 1 /\ decodeAttempted' = TRUE /\ UNCHANGED mode
Done == /\ row = MaxRows /\ UNCHANGED <<mode, row, decodeAttempted>>
Next == Project \/ Decode \/ Done

SkipDecodeNeverAttemptsDecode == mode = "SkipDecode" => decodeAttempted = FALSE
DecodeRequestedEventuallyAttemptsDecode == mode # "SkipDecode" /\ row > 0 => decodeAttempted
TypeOK == /\ mode \in Modes /\ row \in 0..MaxRows /\ decodeAttempted \in BOOLEAN

====

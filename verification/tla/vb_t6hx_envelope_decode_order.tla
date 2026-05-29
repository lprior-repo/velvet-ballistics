---- MODULE vb_t6hx_envelope_decode_order ----
EXTENDS Naturals, TLC

CONSTANTS MaxPayload, HeaderBytes, PayloadCases, U32OverflowPayload
VARIABLES stage, postcard, headerAvailable, magicOk, schemaOk, familyOk, payloadLen, crcOk, digestOk, availableBytes, terminal, err

Stages == {"HeaderLen", "Magic", "Schema", "Family", "PayloadLen", "HeaderCrc", "Availability", "Digest", "Postcard", "Error", "Done"}
Errors == {"None", "UnexpectedEof", "BadMagic", "BadSchema", "BadFamily", "PayloadTooLarge", "HeaderChecksumMismatch", "PayloadDigestMismatch"}
IsOverflowU32 == payloadLen = U32OverflowPayload
LenOk == ~IsOverflowU32 /\ payloadLen <= MaxPayload
AvailableOk == ~IsOverflowU32 /\ availableBytes >= HeaderBytes + payloadLen
Init == /\ stage = "HeaderLen" /\ postcard = FALSE /\ terminal = FALSE /\ err = "None"
        /\ headerAvailable \in BOOLEAN /\ magicOk \in BOOLEAN /\ schemaOk \in BOOLEAN /\ familyOk \in BOOLEAN
        /\ payloadLen \in PayloadCases /\ crcOk \in BOOLEAN /\ digestOk \in BOOLEAN
        /\ availableBytes \in 0..(HeaderBytes + MaxPayload + 1)

Advance == 
  \/ /\ stage = "HeaderLen"
     /\ IF headerAvailable THEN /\ stage' = "Magic" /\ err' = err ELSE /\ stage' = "Error" /\ err' = "UnexpectedEof"
  \/ /\ stage = "Magic"
     /\ IF magicOk THEN /\ stage' = "Schema" /\ err' = err ELSE /\ stage' = "Error" /\ err' = "BadMagic"
  \/ /\ stage = "Schema"
     /\ IF schemaOk THEN /\ stage' = "Family" /\ err' = err ELSE /\ stage' = "Error" /\ err' = "BadSchema"
  \/ /\ stage = "Family"
     /\ IF familyOk THEN /\ stage' = "PayloadLen" /\ err' = err ELSE /\ stage' = "Error" /\ err' = "BadFamily"
  \/ /\ stage = "PayloadLen"
     /\ IF LenOk THEN /\ stage' = "HeaderCrc" /\ err' = err ELSE /\ stage' = "Error" /\ err' = "PayloadTooLarge"
  \/ /\ stage = "HeaderCrc"
     /\ IF crcOk THEN /\ stage' = "Availability" /\ err' = err ELSE /\ stage' = "Error" /\ err' = "HeaderChecksumMismatch"
  \/ /\ stage = "Availability"
     /\ IF AvailableOk THEN /\ stage' = "Digest" /\ err' = err ELSE /\ stage' = "Error" /\ err' = "UnexpectedEof"
  \/ /\ stage = "Digest"
     /\ IF digestOk THEN /\ stage' = "Postcard" /\ err' = err ELSE /\ stage' = "Error" /\ err' = "PayloadDigestMismatch"
  \/ /\ stage = "Postcard" /\ postcard' = TRUE /\ stage' = "Done" /\ err' = err
  \/ /\ stage \in {"Error", "Done"} /\ terminal' = TRUE /\ stage' = stage /\ err' = err
Next == /\ Advance /\ UNCHANGED <<headerAvailable, magicOk, schemaOk, familyOk, payloadLen, crcOk, digestOk, availableBytes>>
        /\ IF stage # "Postcard" THEN postcard' = postcard ELSE TRUE
        /\ IF stage \notin {"Error", "Done"} THEN terminal' = terminal ELSE TRUE

PostcardOnlyAfterIntegrity == postcard => headerAvailable /\ magicOk /\ schemaOk /\ familyOk /\ LenOk /\ crcOk /\ AvailableOk /\ digestOk
PayloadTooLargeBeforeAllocation == (~LenOk /\ headerAvailable /\ magicOk /\ schemaOk /\ familyOk) => stage # "Postcard" /\ err \in {"None", "PayloadTooLarge"}
TypedTerminal == terminal => stage \in {"Error", "Done"} /\ err \in Errors
PrePostcardErrorsTyped == stage = "Error" => err \in Errors \ {"None"}
TypeOK == /\ stage \in Stages /\ postcard \in BOOLEAN /\ terminal \in BOOLEAN /\ err \in Errors
          /\ headerAvailable \in BOOLEAN /\ magicOk \in BOOLEAN /\ schemaOk \in BOOLEAN /\ familyOk \in BOOLEAN
          /\ payloadLen \in PayloadCases /\ crcOk \in BOOLEAN /\ digestOk \in BOOLEAN /\ availableBytes \in 0..(HeaderBytes + MaxPayload + 1)

====

# Native gate receipts, published retrospectively

Raw cache bytes copied without editing. Gate JSON records actual command, cwd, exit, duration, and timeout. It does not record a contemporaneous Git SHA or source tree hash: tested source identity is UNRECORDED in these raw receipts.

Historical coordinator association, not cryptographic source provenance: expanded68 V43 -> 6b591076ebc4423dbe01e320bd738f4b9eaa5aa7; compilerlib V41 -> same eventual V43 production state, before final test additions; Collect V38 -> 894146ec19ee7147c224b84de996db93f0f29f60 before CollectNext rearm; CI V32 argv explicitly specifies intended head 784d6f67227f8f8b3490eb5062855b24a1530a59, actual working tree hash UNRECORDED. No retrospective source-equality attestation is supplied.

Observed outcomes: expanded suite 68 PASS; compiler library 504 PASS / 1 FAIL; Collect runtime suite 169 PASS / 7 FAIL; canonical moon ci incomplete at 600 second timeout, exit124. These are bounded outcomes, not acceptance or proof closure. manifest.json binds only published raw bytes.

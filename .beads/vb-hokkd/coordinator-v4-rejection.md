# V4 contract is unaccepted

Root source audit rejects the actual written V4 before implementation. The admission snippet substitutes admission.artifact_digest() for the per-run admitted policy digest; this violates the explicit identity boundary and does not type-match PolicyDigest. Use actual RunPolicyPin::Pinned digest conversion, never artifact identity. Proposed adapter.journal.append_sequenced is not an actual storage/sequence API; shard-owned sequencing and strict completion must name actual methods/ownership. The draft also reproduces instance-bool emergency activation while describing durable store control later, so the proposed runtime API is internally inconsistent and does not check the durable current marker at the exact dispatch stage.

Scenario4 omits required actual runtime/store cold reopen. Scenario5 queues are not a demonstrated journal append failure seam; exact existing fault API and typederror must be sourced. No new Runtime code is accepted or authorized by this document. Existing V3/V4 drafts are preserved; a bounded corrected source-bound contract is still required before live dispatch implementation.

Source hash claims must be recomputed; see coordinator actual audit output and source-map. No tests or gates were run by this contract author. Parent vb-ny068 remains open.

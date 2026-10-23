# Actual independent local3090 extraction review

Base: `c0d3e4ccf68f7299fb1aaddfa7bfaa865ea9284b`. Exact published candidate: `dd823160331dfac4280289541de419563cf0521f`. A supplied spelling with an additional trailing `a` had 41 characters and was not a valid Git object. The named published bookmark resolved to the exact 40-character pin above; this review used that actual object and its exact source.

The RTX5090 authored the extraction. A separate actual RTX3090 request used `qwen3.8-27b-uncensored`, medium reasoning, endpoint `http://127.0.0.1:11001/v1/chat/completions`, with no tools or cloud fallback. The full request, response, report, runner and timing are preserved. Finish reason `stop`, elapsed 61.5619297 seconds, 8220 prompt and 3775 completion tokens. The outcome was `APPROVE_STATIC_EQUIVALENCE` with no blocking findings.

Independent mechanical comparison confirms 37 assertion predicates/messages after the disclosed reserve-result expression inlining, 12 cover calls in identical order, five `kani::any` calls, no assumptions, unchanged `unwind(12)`, and byte-identical three raw-field assertion helpers. Static review checks tuple argument order, production-call order, success/failure branch reachability, partial-allocation/final-count errors and value/borrow/forget lifetime placement.

Measured source locations: the original proof is lines 96 through 204 inclusive, **109** physical lines, and the new proof is 21 lines. The raw model report says 108 for the original; that is an off-by-one reporting error, retained without alteration. New helper functions are 14–26 lines, and every function is at most 32 lines. This resolves the prior one-page reviewability finding.

No Cargo, build, Kani execution or Rust edits occurred during this review. Static equivalence is not a formal verdict, unwind-adequacy proof, runtime acceptance or performance evidence. V187 remains the historical timeout/no-verdict. Master owns the sole V196 formal run. This artifact commit adds only this review directory on the exact candidate; do not substitute the artifact commit for native tested-source identity.

The earlier complete local3090 source/domain review is separately preserved at commit `9d3ecb796b06b6caa64114ca3310eb3657d96427`, `.beads/vb-8eocj.5/reviews/local3090-kani-c0d3-v193/`. Original cloud CODE/TEST reports require their coordinator's separate publication; this directory does not relabel summaries as originals.

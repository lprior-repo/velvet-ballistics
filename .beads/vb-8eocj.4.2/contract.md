# Rust contract: bounded source symbol interning

Standalone compiler dependency. Each unique UTF-8 byte string receives one checked SymbolId in first-definition order; duplicates reuse IDs without changing totals. Existing4096byte/symbol and MAX_VALUES_PER_RUN count bounds apply, with checkedbyte totals and typedreservation/limitfailures. finish consumes the table and returns a dense ID-ordered immutablepayload independent ofhash iteration order. Rejectedoversized additions preserve table/ID/byte state.

Contract persisted before applying reviewed standalonecandidate to nativeintegration. Earlier cloud source draft exists; no retroactive contract-first-file claim. This dependency is not registered in canonical lowering or runtime. Source text support, metadata, secret-value handling, admission/prefix seeding, persistence/versionedartifact migration remain separate open integration contracts. Native standaloneunit success establishes helperbehavior only.

Ten tests cover deterministicID/order/deduplication, UTF-8byte limits, checkedbounds/overflow, unchangedrejectedstate and fullcountreuse. Allocationfault/NonDensefault injection and fullpipeline are not claimed. Independent CODE/TEST and nativegates required before acceptance.

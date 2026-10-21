# Boundaries
Cold compiler accepts bounded source bytes and action contracts. Pure artifact
binding uses postcard and BLAKE3 and performs no storage/network/time/random I/O.
Runtime verifier must recompute payload/contract/policy binding and validate IR
independently. Storage owns persisted ledger schema and successful durability.
CLI owns source provenance capture and calls the checked compilation entry point.

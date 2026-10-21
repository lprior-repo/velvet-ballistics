# Integrated actual runtime package result

Bounded fullpackage command systemd-run --user --scope --collect --quiet -p MemoryHigh=20G -p MemoryMax=24G -p MemorySwapMax=0 env RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 timeout300 cargo test -p vb_runtime --no-fail-fast completedexit0. Aggregate2449 passed,0failed,1ignored across36resultrows. The1ignoreisintentionalparent-invokedcrash_child_marker, not a skippedbehaviorclaim. Rawcompleteoutputretained. Fullpackagecheck nowincludespriorreviewedsnapshotfixandnew28ticketauthoritytests. This isruntimepackageonly, notcanonicalmoonci/v1green.

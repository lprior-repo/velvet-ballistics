#![forbid(unsafe_code)]

#[path = "vb_eirmk_bound_symbols_runtime/cold.rs"]
mod cold;
#[path = "vb_eirmk_bound_symbols_runtime/failures.rs"]
mod failures;
#[path = "vb_eirmk_bound_symbols_runtime/fresh.rs"]
mod fresh;
#[path = "vb_eirmk_bound_symbols_runtime/goldens.rs"]
mod goldens;
#[path = "vb_eirmk_bound_symbols_runtime/legacy_authority.rs"]
mod legacy_authority;
#[path = "vb_eirmk_bound_symbols_runtime/retry.rs"]
mod retry;
#[path = "vb_eirmk_bound_symbols_runtime/retry_support.rs"]
mod retry_support;
#[path = "vb_eirmk_bound_symbols_runtime/support.rs"]
mod support;

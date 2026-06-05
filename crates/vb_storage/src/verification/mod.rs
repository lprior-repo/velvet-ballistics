#![forbid(unsafe_code)]

#[cfg(any(test, flux))]
pub mod flux;

#[cfg(test)]
pub mod mrwe5_production_bridge;

#[cfg(kani)]
#[path = "vb-fn4vt/mod.rs"]
pub mod vb_fn4vt;

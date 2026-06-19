#![cfg(all(kani, feature = "kani-vb-god2f-hard-verus"))]
#![forbid(unsafe_code)]

//! HVR-PO-STORAGE-{002,005}: production storage classification/recovery harnesses.

mod classification;
mod recovery;
mod replay_model;

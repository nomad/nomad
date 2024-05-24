//! TODO: docs

#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc;

pub use nvim_oxi as oxi;

#[cfg(feature = "executor")]
#[cfg_attr(docsrs, doc(cfg(feature = "executor")))]
pub mod executor;

//! TODO: docs

#![cfg_attr(docsrs, feature(doc_cfg))]

pub use nvim_oxi as oxi;

#[cfg(feature = "executor")]
#[cfg_attr(docsrs, doc(cfg(feature = "executor")))]
pub mod executor;

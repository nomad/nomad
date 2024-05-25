//! TODO: docs

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "test_macro")]
mod build;
#[cfg(feature = "test_macro")]
#[doc(hidden)]
pub mod test_macro;

#[cfg(feature = "test_macro")]
#[cfg_attr(docsrs, doc(cfg(feature = "test_macro")))]
pub use build::build;

type TestError = Box<dyn std::error::Error>;
type TestResult = Result<(), TestError>;

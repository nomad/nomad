//! TODO: docs

#![cfg_attr(docsrs, feature(doc_cfg))]

pub use nvim_oxi as oxi;
#[cfg(feature = "executor")]
#[cfg_attr(docsrs, doc(cfg(feature = "executor")))]
#[doc(inline)]
pub use nvimx_executor as executor;
#[cfg(feature = "macros")]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
#[doc(inline)]
pub use nvimx_macros as macros;
#[cfg(feature = "project")]
#[cfg_attr(docsrs, doc(cfg(feature = "project")))]
#[doc(inline)]
pub use nvimx_project as project;
#[cfg(feature = "tests")]
#[cfg_attr(docsrs, doc(cfg(feature = "tests")))]
#[doc(inline)]
pub use nvimx_tests as tests;
#[cfg(feature = "ui")]
#[cfg_attr(docsrs, doc(cfg(feature = "ui")))]
#[doc(inline)]
pub use nvimx_ui as ui;

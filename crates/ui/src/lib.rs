//! TODO: docs

extern crate alloc;

mod bound;
mod cells;
mod color;
mod component;
mod expand_rect;
mod highlight;
mod memoize;
mod metric;
mod popover;
mod react;
pub mod render;
mod requested_bound;
mod scene;
mod scene_fragment;
mod surface;
mod view;

use bound::Bound;
pub use cells::Cells;
pub use color::Color;
pub use component::Component;
pub use expand_rect::ExpandRect;
pub(crate) use highlight::HighlightGroup;
pub use highlight::{Highlight, HighlightName};
use memoize::Memoize;
pub use metric::Metric;
pub use popover::{Popover, PopoverAnchor, PopoverBuilder};
pub use react::React;
pub use render::{IntoRender, Render};
pub use requested_bound::RequestedBound;
pub(crate) use scene::Scene;
pub use scene_fragment::{Cutout, SceneFragment};
pub(crate) use surface::{Point, Surface};
use view::View;

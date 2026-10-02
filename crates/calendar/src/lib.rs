//! Pure native calendar preparation and scoped versioning. No persistence, clock or publication authority.
#![allow(clippy::result_large_err)]
mod events;
mod model;
mod projection;
mod revisions;
pub use events::prepare_events;
pub use model::*;
pub use revisions::version_events;

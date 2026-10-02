//! Pure native calendar preparation, scoped versioning and ICS bytes. No persistence, clock or publication authority.
#![allow(clippy::result_large_err)]
mod events;
mod ics;
mod model;
mod projection;
mod revisions;
pub use events::prepare_events;
pub use ics::render_ics;
pub use model::*;
pub use revisions::version_events;

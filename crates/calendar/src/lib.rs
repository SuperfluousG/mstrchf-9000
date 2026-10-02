//! Pure native calendar preparation. No persistence, clock or publication authority.
#![allow(clippy::result_large_err)]
mod events;
mod model;
mod projection;
pub use events::prepare_events;
pub use model::*;

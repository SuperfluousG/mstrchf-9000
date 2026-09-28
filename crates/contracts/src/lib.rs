//! Strict versioned wire contracts for MSTRCHF 9000.
#[macro_export]
macro_rules! record {
    ($(#[$meta:meta])* pub struct $name:ident { $(pub $field:ident: $ty:ty),* $(,)? }) => {
        $(#[$meta])* #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $(pub $field: $ty),* }
    };
}
#[macro_export]
macro_rules! string_enum {
    (pub enum $name:ident { $($variant:ident),* $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
        #[serde(rename_all="snake_case")]
        pub enum $name { $($variant),* }
    };
}
#[macro_export]
macro_rules! tagged {
    (pub enum $name:ident { $($variant:ident { $($field:ident: $ty:ty),* $(,)? }),* $(,)? }) => {
        #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
        #[serde(tag="kind", content="data", rename_all="snake_case", deny_unknown_fields)]
        pub enum $name { $($variant { $($field: $ty),* }),* }
    };
}
mod error;
mod ids;
mod limits;
mod parse;
pub use error::*;
pub use ids::*;
pub use limits::*;
pub use parse::*;
pub mod quantity;
pub mod revision;
pub mod time;
pub use quantity::*;
pub use revision::*;
pub use time::*;
mod calendar;
mod canonical;
mod command;
mod context;
mod derived;
mod household;
mod job;
mod operation;
mod privacy;
mod read;
mod recipe;
mod week;
pub use calendar::*;
pub use canonical::*;
pub use command::*;
pub use context::*;
pub use derived::*;
pub use household::*;
pub use job::*;
pub use operation::*;
pub use privacy::*;
pub use read::*;
pub use recipe::*;
pub use week::*;
pub mod schema;

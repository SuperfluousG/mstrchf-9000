//! Frozen checked representations and private store adapters; no filesystem authority.
#![allow(dead_code)] // Preserve the full frozen graph and its independent inspection surface.
pub(crate) mod b1_v1;
mod codec;
pub(crate) mod draft005;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Code {
    MalformedJson,
    DuplicateKey,
    LimitExceeded,
    UnsupportedVersion,
    UnknownField,
    MissingField,
    InvalidValue,
    MissingReference,
    ForeignReference,
    NoncanonicalValue,
    ArithmeticOverflow,
    ContextStale,
    IdentityMismatch,
    DigestMismatch,
    Integrity,
    UnknownClosure,
}
#[derive(Debug, PartialEq, Eq)]
struct Error {
    code: Code,
    path: String,
}
impl Error {
    fn new(code: Code, path: &str) -> Self {
        Self {
            code,
            path: path.into(),
        }
    }
    fn from_decode(message: &str) -> Self {
        let code = if message.contains("CODE:unknown_closure") {
            Code::UnknownClosure
        } else if message.contains("CODE:duplicate_key") {
            Code::DuplicateKey
        } else if message.contains("CODE:limit_exceeded") {
            Code::LimitExceeded
        } else if message.contains("CODE:noncanonical_value") {
            Code::NoncanonicalValue
        } else if message.contains("CODE:unsupported_version") {
            Code::UnsupportedVersion
        } else if message.contains("missing field") {
            Code::MissingField
        } else if message.contains("unknown field") {
            Code::UnknownField
        } else {
            Code::InvalidValue
        };
        // Never retain serde messages: they may contain household text.
        Self::new(code, "")
    }
}
/// Pure externally supplied projections. This record asserts no DB provenance,
/// catalogue identity, total metadata membership, receipt closure or authority.
struct ReceiptBinding<'a> {
    command_id: &'a str,
    household_id: &'a str,
    response_revision: &'a str,
    live_revision: &'a str,
    command_digest: &'a str,
    response_digest: &'a str,
}
#[cfg(test)]
mod tests;

mod integrated;
pub(crate) use integrated::*;
pub use integrated::{CommandDisposition, ReceiptSnapshot, UiCommandOutcome};

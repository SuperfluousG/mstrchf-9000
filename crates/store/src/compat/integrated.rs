//! Closed store-facing adapters. JSON projections below are read-only and follow frozen decoding.
use super::*;
use mstrchf_contracts::{Digest, DomainError, ErrorCode};
use serde_json::Value;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Profile {
    Old,
    Current,
}
impl Profile {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Old => "B1RetainedV1",
            Self::Current => "B1Draft005",
        }
    }
    pub(crate) fn bound(s: &str) -> Result<Self, DomainError> {
        match s {
            "B1RetainedV1" => Ok(Self::Old),
            "B1Draft005" => Ok(Self::Current),
            _ => Err(crate::records::storage("unknown receipt format")),
        }
    }
}
fn domain(e: Error) -> DomainError {
    match e.code {
        Code::UnknownClosure => {
            DomainError::new(ErrorCode::UnsupportedCapability, "/storage/backup/closure")
        }
        Code::LimitExceeded => DomainError::new(ErrorCode::LimitExceeded, "/storage/backup"),
        Code::ArithmeticOverflow => DomainError::new(ErrorCode::ArithmeticOverflow, "/revision"),
        _ => crate::records::storage("frozen interpretation failed"),
    }
}
pub(crate) fn value(p: Profile, root: &str, b: &[u8]) -> Result<Value, DomainError> {
    macro_rules! decode {
        ($m:ident) => {
            match root {
                "draft" => {
                    $m::draft(b).map_err(domain)?;
                }
                "privacy" => {
                    $m::privacy(b).map_err(domain)?;
                }
                "profile" => {
                    $m::profile(b).map_err(domain)?;
                }
                "job" => {
                    $m::job(b).map_err(domain)?;
                }
                "snapshot" => {
                    $m::snapshot(b).map_err(domain)?;
                }
                _ => return Err(crate::records::storage("closed root")),
            }
        };
    }
    match p {
        Profile::Old => decode!(b1_v1),
        Profile::Current => decode!(draft005),
    }
    serde_json::from_slice(b).map_err(crate::records::storage)
}
pub(crate) fn transition_draft(b: &[u8]) -> Result<Vec<u8>, DomainError> {
    let out = b1_v1::transition_draft(b).map_err(domain)?;
    draft005::draft(&out).map_err(domain)?;
    Ok(out)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandDisposition {
    Applied,
    Replayed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReceiptSnapshot {
    B1RetainedV1(b1_v1::B1RetainedV1),
    B1Draft005(draft005::B1Draft005),
    #[cfg(all(debug_assertions, feature = "synthetic-development"))]
    DevelopmentCurrent(Box<mstrchf_contracts::Snapshot>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiCommandOutcome {
    disposition: CommandDisposition,
    response: ReceiptSnapshot,
    command: Vec<u8>,
    bytes: Vec<u8>,
    command_digest: Digest,
    response_digest: Digest,
}
impl UiCommandOutcome {
    #[cfg(all(debug_assertions, feature = "synthetic-development"))]
    pub(crate) fn development(
        command: Vec<u8>,
        bytes: Vec<u8>,
        snapshot: mstrchf_contracts::Snapshot,
    ) -> Self {
        Self {
            disposition: CommandDisposition::Replayed,
            response: ReceiptSnapshot::DevelopmentCurrent(Box::new(snapshot)),
            command_digest: mstrchf_contracts::bytes_digest(&command),
            response_digest: mstrchf_contracts::bytes_digest(&bytes),
            command,
            bytes,
        }
    }
    pub fn disposition(&self) -> CommandDisposition {
        self.disposition
    }
    pub fn response(&self) -> &ReceiptSnapshot {
        &self.response
    }
    pub fn original_command_bytes(&self) -> &[u8] {
        &self.command
    }
    pub fn original_response_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn command_digest(&self) -> &Digest {
        &self.command_digest
    }
    pub fn response_digest(&self) -> &Digest {
        &self.response_digest
    }
    pub(crate) fn applied(mut self) -> Self {
        self.disposition = CommandDisposition::Applied;
        self
    }
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn receipt(
    p: Profile,
    id: &str,
    hh: &str,
    rev: &str,
    max: &str,
    cd: &str,
    rd: &str,
    a: &[u8],
    b: &[u8],
) -> Result<UiCommandOutcome, DomainError> {
    let binding = ReceiptBinding {
        command_id: id,
        household_id: hh,
        response_revision: rev,
        live_revision: max,
        command_digest: cd,
        response_digest: rd,
    };
    let response = match p {
        Profile::Old => ReceiptSnapshot::B1RetainedV1(
            b1_v1::receipt(a, b, &binding)
                .map_err(domain)?
                .response()
                .clone(),
        ),
        Profile::Current => ReceiptSnapshot::B1Draft005(
            draft005::receipt(a, b, &binding)
                .map_err(domain)?
                .response()
                .clone(),
        ),
    };
    Ok(UiCommandOutcome {
        disposition: CommandDisposition::Replayed,
        response,
        command: a.into(),
        bytes: b.into(),
        command_digest: Digest(cd.into()),
        response_digest: Digest(rd.into()),
    })
}

pub(crate) fn envelope(bytes: &[u8]) -> Result<String, DomainError> {
    codec::envelope(bytes).map_err(|e| {
        DomainError::new(
            match e.code {
                Code::LimitExceeded => ErrorCode::LimitExceeded,
                Code::DuplicateKey => ErrorCode::DuplicateKey,
                Code::MalformedJson => ErrorCode::MalformedJson,
                _ => ErrorCode::InvalidValue,
            },
            "/command_id",
        )
    })
}

// Storage projections and ledger instants share the reviewed frozen primitive
// grammar, never whichever primitive parser an evolving wire crate supplies.
pub(crate) fn scalar(name: &str, value: &str) -> Result<(), DomainError> {
    if !matches!(name, "HouseholdId" | "JobId" | "CommandId" | "Instant") {
        return Err(crate::records::storage("closed scalar root"));
    }
    let b = codec::canonical(&value).map_err(domain)?;
    codec::decode::<Value>(&b, b1_v1::SCHEMA, name).map_err(domain)?;
    Ok(())
}

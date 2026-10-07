//! Outbound shell acknowledgement. Historical state bodies never cross this boundary.
use crate::features::SupportedCommand;
use serde::Serialize;
use ts_rs::TS;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ShellReceiptDisposition {
    Applied,
    Replayed,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub enum ShellReceiptProfile {
    B1RetainedV1,
    B1Draft005,
    DevelopmentCurrent,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ShellRequestEncoding {
    CurrentJson,
    CanonicalReceiptJson,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ShellCommandReceipt {
    #[ts(type = "1")]
    pub format_version: u8,
    pub disposition: ShellReceiptDisposition,
    pub receipt_profile: ShellReceiptProfile,
    pub request_encoding: ShellRequestEncoding,
    pub request_digest: String,
    pub command_id: String,
    pub command_kind: SupportedCommand,
    pub expected_snapshot_revision: String,
    pub household_id: String,
    pub receipt_snapshot_revision: String,
    pub command_digest: String,
    pub response_digest: String,
}
pub fn typescript() -> String {
    let cfg = ts_rs::Config::default();
    format!(
        "export {}\nexport {}\nexport {}\nexport {}\n",
        ShellReceiptDisposition::decl(&cfg),
        ShellReceiptProfile::decl(&cfg),
        ShellRequestEncoding::decl(&cfg),
        ShellCommandReceipt::decl(&cfg)
    )
}
#[cfg(all(
    feature = "store-boundary",
    debug_assertions,
    any(feature = "synthetic-development", test)
))]
mod projection {
    use super::*;
    use mstrchf_contracts::{
        AppCommandV1, DomainError, ErrorCode, MAX_DOCUMENT_BYTES, bytes_digest,
    };
    use mstrchf_store::{CommandDisposition, ReceiptSnapshot, UiCommandOutcome};
    use serde_json::Value;
    fn fail() -> DomainError {
        DomainError::new(ErrorCode::StorageFailure, "/shell/receipt")
    }
    fn id(s: &str, prefix: &str) -> bool {
        s.strip_prefix(prefix).is_some_and(|v| {
            v.len() == 32
                && v.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                && v.bytes().any(|b| b != b'0')
        })
    }
    fn revision(s: &str) -> Result<u64, DomainError> {
        if s.is_empty() || s.starts_with('0') || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(fail());
        }
        s.parse::<u64>().map_err(|_| fail())
    }
    fn field<'a>(v: &'a Value, k: &str) -> Result<&'a str, DomainError> {
        v.get(k).and_then(Value::as_str).ok_or_else(fail)
    }
    // Only called with an actual Store-validated outcome. Parsing here projects its
    // header; it never admits a command or chooses a frozen reader.
    pub(crate) fn project(
        outcome: &UiCommandOutcome,
        request: &str,
        encoding: ShellRequestEncoding,
        expected_bytes: &[u8],
        current: Option<&AppCommandV1>,
    ) -> Result<ShellCommandReceipt, DomainError> {
        let a = outcome.original_command_bytes();
        let b = outcome.original_response_bytes();
        if a.len() > MAX_DOCUMENT_BYTES
            || b.len() > MAX_DOCUMENT_BYTES
            || a != expected_bytes
            || bytes_digest(a) != *outcome.command_digest()
            || bytes_digest(b) != *outcome.response_digest()
        {
            return Err(fail());
        }
        let (profile, household, receipt_revision) = match outcome.response() {
            ReceiptSnapshot::B1RetainedV1(s) => (
                ShellReceiptProfile::B1RetainedV1,
                s.household_draft().household_id().value().to_owned(),
                s.snapshot_revision().value().to_owned(),
            ),
            ReceiptSnapshot::B1Draft005(s) => (
                ShellReceiptProfile::B1Draft005,
                s.household_draft().household_id().value().to_owned(),
                s.snapshot_revision().value().to_owned(),
            ),
            #[cfg(feature = "synthetic-development")]
            ReceiptSnapshot::DevelopmentCurrent(s) => (
                ShellReceiptProfile::DevelopmentCurrent,
                s.household_draft.household_id.0.clone(),
                s.snapshot_revision.0.to_string(),
            ),
        };
        let disposition = match outcome.disposition() {
            CommandDisposition::Applied => ShellReceiptDisposition::Applied,
            CommandDisposition::Replayed => ShellReceiptDisposition::Replayed,
        };
        let v: Value = serde_json::from_slice(a).map_err(|_| fail())?;
        let command_id = field(&v, "command_id")?;
        let expected = field(&v, "expected_snapshot_revision")?;
        let kind = field(v.get("command").ok_or_else(fail)?, "kind")?;
        let command_kind = match kind {
            "save_household_draft" => SupportedCommand::SaveHouseholdDraft,
            "set_privacy" => SupportedCommand::SetPrivacy,
            "cancel_job" => SupportedCommand::CancelJob,
            "update_profile" => SupportedCommand::UpdateProfile,
            "save_week" => SupportedCommand::SaveWeek,
            _ => return Err(fail()),
        };
        if !id(command_id, "cm_")
            || !id(&household, "hh_")
            || revision(expected)?.checked_add(1) != Some(revision(&receipt_revision)?)
            || (disposition == ShellReceiptDisposition::Applied
                && !matches!(
                    profile,
                    ShellReceiptProfile::B1Draft005 | ShellReceiptProfile::DevelopmentCurrent
                ))
        {
            return Err(fail());
        }
        if let Some(c) = current {
            let header = serde_json::to_value(c).map_err(|_| fail())?;
            if header["command_id"] != v["command_id"]
                || header["expected_snapshot_revision"] != v["expected_snapshot_revision"]
                || header["command"]["kind"] != v["command"]["kind"]
            {
                return Err(fail());
            }
        }
        let request_digest = bytes_digest(request.as_bytes()).0;
        if encoding == ShellRequestEncoding::CanonicalReceiptJson
            && (disposition != ShellReceiptDisposition::Replayed
                || request_digest != outcome.command_digest().0)
        {
            return Err(fail());
        }
        Ok(ShellCommandReceipt {
            format_version: 1,
            disposition,
            receipt_profile: profile,
            request_encoding: encoding,
            request_digest,
            command_id: command_id.into(),
            command_kind,
            expected_snapshot_revision: expected.into(),
            household_id: household,
            receipt_snapshot_revision: receipt_revision,
            command_digest: outcome.command_digest().0.clone(),
            response_digest: outcome.response_digest().0.clone(),
        })
    }
}
#[cfg(all(
    feature = "store-boundary",
    debug_assertions,
    any(feature = "synthetic-development", test)
))]
pub(crate) use projection::project;

use crate::*;
string_enum! {pub enum ErrorCode { MalformedJson, DuplicateKey, LimitExceeded, UnsupportedVersion, UnknownField, MissingField, InvalidValue, MissingReference, ForeignReference, NoncanonicalValue, ArithmeticOverflow, ContextStale, IdentityMismatch, DigestMismatch, ReplayConflict, ScopeViolation, PrivacyDenied, AuthorityDenied, UnsupportedCapability, DomainConflict, WorkerNotStopped, ReconciliationRequired, OperationUncertain, StorageFailure }}
string_enum! {pub enum Retry { Never, AfterRefresh, AfterUserAction, ReadBackoff }}
record! {pub struct ValidationIssue {pub path: String, pub code: ErrorCode, pub entity_id: Nullable<Text>}}
record! {pub struct DomainError {pub code: ErrorCode, pub issues: BoundedVec<ValidationIssue,0,100>, pub retry: Retry, pub truncated: bool}}
impl DomainError {
    pub fn new(code: ErrorCode, path: &str) -> Self {
        Self {
            code,
            issues: BoundedVec(vec![ValidationIssue {
                path: path.into(),
                code,
                entity_id: Nullable(None),
            }]),
            retry: Retry::Never,
            truncated: false,
        }
    }
}
impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.code)
    }
}
impl std::error::Error for DomainError {}

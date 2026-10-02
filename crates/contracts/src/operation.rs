//! External operation envelopes and pure basket recovery predicates.
use crate::*;

string_enum! { pub enum Provider { CodexLocal, ClaudeCowork } }
string_enum! { pub enum Retailer { Tesco } }
string_enum! { pub enum OperationOutcome { ObservedSuccess, ObservedFailure, Uncertain } }
string_enum! { pub enum OperationState { Intended, Granted, ObservedSuccess, ObservedFailure, Uncertain } }
string_enum! { pub enum BasketLockState { Active, Quarantined } }
string_enum! { pub enum NativeTerminalStatus { Cancelled, Completed, Failed } }
string_enum! { pub enum VisibleTerminalStatus { Cancelled, Completed } }
string_enum! { pub enum FinancialCommitment { None, Unknown } }

record! { pub struct ProductRef {
    pub retailer: Retailer, pub product_id: Text, pub variant_id: Nullable<Text>
} }
record! { pub struct BasketKey {
    pub retailer: Retailer, pub account_ref: Text, pub basket_id: BasketId
} }
tagged! { pub enum ExternalResource {
    Basket { key: BasketKey }, Calendar { destination_id: CalendarDestinationId }
} }
record! { pub struct WorkerIdentity {
    pub provider: Provider, pub provider_build: Text, pub native_task_ref: Text,
    pub job_id: JobId, pub generation: Generation
} }
tagged! { pub enum ExternalAction {
    BasketSetQuantity {
        product_ref: ProductRef, expected_observed_quantity: u32,
        target_quantity: u32, planner_delta: i32,
        product_evidence_digest: Digest, price_approval_digest: Digest,
        baseline_digest: Digest
    },
    HoldSlot { slot_ref: Text, expires_at: Instant, nonfinancial_evidence_digest: Digest },
    CalendarCreate { event_id: CalendarEventId, payload_digest: PayloadDigest },
    CalendarUpdate { event_id: CalendarEventId, provider_event_ref: Text,
        expected_observed_digest: Digest, payload_digest: PayloadDigest },
    CalendarCancel { event_id: CalendarEventId, provider_event_ref: Text,
        expected_observed_digest: Digest }
} }
record! { pub struct OperationRequest {
    pub schema_version: SchemaVersion, pub operation_id: OperationId,
    pub worker: WorkerIdentity, pub resource: ExternalResource,
    pub context_stamp: ContextStamp, pub input_digest: InputDigest,
    pub approval_id: ApprovalId, pub action: ExternalAction, pub requested_at: Instant
} }
record! { pub struct OperationGrant {
    pub schema_version: SchemaVersion, pub operation_id: OperationId,
    pub worker: WorkerIdentity, pub resource: ExternalResource,
    pub context_stamp: ContextStamp, pub input_digest: InputDigest,
    pub approval_id: ApprovalId, pub action: ExternalAction,
    pub grant_token: GrantToken, pub granted_at: Instant, pub expires_at: Instant
} }
tagged! { pub enum OperationEvidence {
    Basket { live_quantity: u32, observed_product_ref: ProductRef,
        basket_digest: Digest, evidence_id: EvidenceId },
    Calendar { provider_event_ref: Nullable<Text>, logical_event_id: CalendarEventId,
        observed_payload_digest: Nullable<Digest>, evidence_id: EvidenceId },
    Slot { slot_ref: Text, hold_expires_at: Nullable<Instant>,
        financial_commitment: FinancialCommitment, evidence_id: EvidenceId }
} }
record! { pub struct OperationObservation {
    pub schema_version: SchemaVersion, pub operation_id: OperationId,
    pub worker: WorkerIdentity, pub grant_token: GrantToken,
    pub observed_at: Instant, pub outcome: OperationOutcome,
    pub evidence: OperationEvidence
} }
tagged! { pub enum StopSource {
    NativeTerminal { status: NativeTerminalStatus,
        provider_evidence_id: EvidenceId, observed_at: Instant },
    UserConfirmed { procedure_version: Revision,
        visible_status: VisibleTerminalStatus, confirmed_at: Instant }
} }
record! { pub struct WorkerStopEvidence {
    pub schema_version: SchemaVersion, pub worker: WorkerIdentity,
    pub qualification_id: EvidenceId, pub source: StopSource,
    pub recorded_at: Instant
} }
record! { pub struct ReconciliationEvidence {
    pub evidence_id: EvidenceId, pub key: BasketKey,
    pub old_worker: WorkerIdentity, pub observed_at: Instant,
    pub basket_digest: Digest,
    pub unresolved_operation_ids: BoundedVec<OperationId, 0, 256>,
    pub manual_conflicts: bool
} }
record! { pub struct BasketLock {
    pub key: BasketKey, pub worker: WorkerIdentity,
    pub state: BasketLockState,
    pub outstanding_operation: Nullable<OperationId>,
    pub stop_evidence: Nullable<WorkerStopEvidence>,
    pub last_reconciliation: Nullable<ReconciliationEvidence>
} }

/// A local qualification result. Neither value is wire evidence supplied by a worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopSourceTrust {
    Untrusted,
    LocalUser,
    QualifiedProvider,
}

/// App-owned facts checked before P02b can atomically advance a quarantined basket lock.
pub struct ResumeFacts<'a> {
    pub lock: &'a BasketLock,
    pub stop_source_trust: StopSourceTrust,
    pub qualification_matches: bool,
    pub reconciliation: Option<&'a ReconciliationEvidence>,
    pub current_context: &'a ContextStamp,
    pub expected_context: &'a ContextStamp,
    pub policy: &'a PrivacyPolicy,
    pub approval_current: bool,
}

pub fn may_resume_basket(facts: &ResumeFacts<'_>) -> Result<(), DomainError> {
    let lock = facts.lock;
    if lock.state != BasketLockState::Quarantined {
        return Err(DomainError::new(ErrorCode::DomainConflict, "/state"));
    }
    let stop = lock
        .stop_evidence
        .0
        .as_ref()
        .ok_or_else(|| DomainError::new(ErrorCode::WorkerNotStopped, "/stop_evidence"))?;
    if stop.worker != lock.worker || !facts.qualification_matches {
        return Err(DomainError::new(
            ErrorCode::WorkerNotStopped,
            "/stop_evidence",
        ));
    }
    let confirmed_at = match (&stop.source, facts.stop_source_trust) {
        (StopSource::NativeTerminal { observed_at, .. }, StopSourceTrust::QualifiedProvider) => {
            observed_at
        }
        (StopSource::UserConfirmed { confirmed_at, .. }, StopSourceTrust::LocalUser) => {
            confirmed_at
        }
        _ => {
            return Err(DomainError::new(
                ErrorCode::WorkerNotStopped,
                "/stop_evidence/source",
            ));
        }
    };
    if stop.recorded_at.timestamp_seconds() < confirmed_at.timestamp_seconds() {
        return Err(DomainError::new(
            ErrorCode::WorkerNotStopped,
            "/stop_evidence/recorded_at",
        ));
    }
    let reconciliation = facts
        .reconciliation
        .ok_or_else(|| DomainError::new(ErrorCode::ReconciliationRequired, "/reconciliation"))?;
    if reconciliation.key != lock.key
        || reconciliation.old_worker != lock.worker
        || reconciliation.observed_at.timestamp_seconds() <= stop.recorded_at.timestamp_seconds()
        || !reconciliation.unresolved_operation_ids.is_empty()
        || reconciliation.manual_conflicts
        || lock.outstanding_operation.0.is_some()
    {
        return Err(DomainError::new(
            ErrorCode::ReconciliationRequired,
            "/reconciliation",
        ));
    }
    compare_context(facts.expected_context, facts.current_context)?;
    if facts.policy.revision != facts.current_context.privacy_revision {
        return Err(DomainError::new(
            ErrorCode::ContextStale,
            "/policy/revision",
        ));
    }
    if !facts.policy.use_ai_features {
        return Err(DomainError::new(
            ErrorCode::PrivacyDenied,
            "/policy/use_ai_features",
        ));
    }
    if !facts.approval_current {
        return Err(DomainError::new(ErrorCode::AuthorityDenied, "/approval_id"));
    }
    Ok(())
}

pub fn validate_external_action(
    action: &ExternalAction,
    resource: &ExternalResource,
) -> Result<(), DomainError> {
    match (action, resource) {
        (
            ExternalAction::BasketSetQuantity {
                product_ref,
                expected_observed_quantity,
                target_quantity,
                planner_delta,
                ..
            },
            ExternalResource::Basket { key },
        ) => {
            if product_ref.retailer != key.retailer {
                return Err(DomainError::new(
                    ErrorCode::IdentityMismatch,
                    "/action/product_ref/retailer",
                ));
            }
            if *expected_observed_quantity > 10_000
                || *target_quantity > 10_000
                || !(-10_000..=10_000).contains(planner_delta)
            {
                return Err(DomainError::new(ErrorCode::LimitExceeded, "/action"));
            }
            if i64::from(*expected_observed_quantity) + i64::from(*planner_delta)
                != i64::from(*target_quantity)
            {
                return Err(DomainError::new(
                    ErrorCode::InvalidValue,
                    "/action/target_quantity",
                ));
            }
        }
        (ExternalAction::HoldSlot { .. }, ExternalResource::Basket { .. }) => {}
        (
            ExternalAction::CalendarCreate { .. }
            | ExternalAction::CalendarUpdate { .. }
            | ExternalAction::CalendarCancel { .. },
            ExternalResource::Calendar { .. },
        ) => {}
        _ => return Err(DomainError::new(ErrorCode::ScopeViolation, "/resource")),
    }
    Ok(())
}

pub fn validate_operation_approval(
    action: &ExternalAction,
    approval: &ApprovedContributionRef,
) -> Result<(), DomainError> {
    validate_approved_contribution_shape(approval)?;
    let ExternalAction::BasketSetQuantity {
        product_ref,
        expected_observed_quantity,
        target_quantity,
        planner_delta,
        product_evidence_digest,
        price_approval_digest,
        ..
    } = action
    else {
        return Err(DomainError::new(ErrorCode::ScopeViolation, "/action"));
    };
    if *expected_observed_quantity > 10_000
        || *target_quantity > 10_000
        || !(-10_000..=10_000).contains(planner_delta)
        || i64::from(*expected_observed_quantity) + i64::from(*planner_delta)
            != i64::from(*target_quantity)
    {
        return Err(DomainError::new(
            ErrorCode::InvalidValue,
            "/action/target_quantity",
        ));
    }
    if *product_evidence_digest != approval.product_evidence_digest
        || *price_approval_digest != approval.price_approval_digest
    {
        return Err(DomainError::new(ErrorCode::DigestMismatch, "/action"));
    }
    let line = approval
        .lines
        .iter()
        .find(|line| &line.product == product_ref)
        .ok_or_else(|| DomainError::new(ErrorCode::AuthorityDenied, "/action/product_ref"))?;
    if line.expected_quantity != *expected_observed_quantity
        || line.target_quantity != *target_quantity
    {
        return Err(DomainError::new(
            ErrorCode::AuthorityDenied,
            "/action/target_quantity",
        ));
    }
    if *planner_delta < 0 && (!line.removal_approved || line.planner_attribution_digest.0.is_none())
    {
        return Err(DomainError::new(
            ErrorCode::AuthorityDenied,
            "/action/planner_delta",
        ));
    }
    Ok(())
}

pub fn validate_approved_contribution_shape(
    approval: &ApprovedContributionRef,
) -> Result<(), DomainError> {
    if !(1..=256).contains(&approval.lines.len()) {
        return Err(DomainError::new(ErrorCode::LimitExceeded, "/lines"));
    }
    for (index, line) in approval.lines.iter().enumerate() {
        if line.expected_quantity > 10_000 || line.target_quantity > 10_000 {
            return Err(DomainError::new(ErrorCode::LimitExceeded, "/lines"));
        }
        if approval.lines[..index]
            .iter()
            .any(|prior| prior.product == line.product)
        {
            return Err(DomainError::new(ErrorCode::InvalidValue, "/lines"));
        }
        if line.target_quantity < line.expected_quantity
            && (!line.removal_approved || line.planner_attribution_digest.0.is_none())
        {
            return Err(DomainError::new(ErrorCode::AuthorityDenied, "/lines"));
        }
    }
    Ok(())
}

pub fn validate_operation_shape(request: &OperationRequest) -> Result<(), DomainError> {
    validate_external_action(&request.action, &request.resource)?;
    if let ExternalAction::HoldSlot { expires_at, .. } = &request.action
        && expires_at.timestamp_seconds() <= request.requested_at.timestamp_seconds()
    {
        return Err(DomainError::new(
            ErrorCode::InvalidValue,
            "/action/expires_at",
        ));
    }
    Ok(())
}

pub fn validate_grant_shape(grant: &OperationGrant) -> Result<(), DomainError> {
    validate_external_action(&grant.action, &grant.resource)?;
    if grant.expires_at.timestamp_seconds() <= grant.granted_at.timestamp_seconds() {
        return Err(DomainError::new(ErrorCode::InvalidValue, "/expires_at"));
    }
    if let ExternalAction::HoldSlot { expires_at, .. } = &grant.action
        && expires_at.timestamp_seconds() <= grant.granted_at.timestamp_seconds()
    {
        return Err(DomainError::new(
            ErrorCode::InvalidValue,
            "/action/expires_at",
        ));
    }
    Ok(())
}

pub fn validate_operation_observation_shape(
    observation: &OperationObservation,
) -> Result<(), DomainError> {
    match &observation.evidence {
        OperationEvidence::Basket { live_quantity, .. } if *live_quantity > 10_000 => Err(
            DomainError::new(ErrorCode::LimitExceeded, "/evidence/live_quantity"),
        ),
        OperationEvidence::Slot {
            financial_commitment: FinancialCommitment::Unknown,
            ..
        } if observation.outcome == OperationOutcome::ObservedSuccess => Err(DomainError::new(
            ErrorCode::OperationUncertain,
            "/evidence/financial_commitment",
        )),
        OperationEvidence::Slot {
            hold_expires_at: Nullable(None),
            ..
        } if observation.outcome == OperationOutcome::ObservedSuccess => Err(DomainError::new(
            ErrorCode::OperationUncertain,
            "/evidence/hold_expires_at",
        )),
        _ => Ok(()),
    }
}

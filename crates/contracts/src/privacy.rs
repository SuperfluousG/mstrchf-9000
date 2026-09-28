use crate::*;
record! {pub struct PrivacyPolicy {pub revision:Revision,pub use_ai_features:bool,pub include_household_requirements:bool,pub confirmed_at:Nullable<Instant>}}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllowedArtefactBytes {
    pub artefact_id: ArtefactId,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutboundBundle {
    pub request: JobRequestV1,
    pub artefacts: Vec<AllowedArtefactBytes>,
}
fn policy_payload(payload: &JobPayload, policy: &PrivacyPolicy) -> Result<(), DomainError> {
    if !policy.use_ai_features {
        return Err(DomainError::new(
            ErrorCode::PrivacyDenied,
            "/use_ai_features",
        ));
    }
    if !policy.include_household_requirements
        && match payload {
            JobPayload::PersonalisedPlan { .. } | JobPayload::CalendarPublication { .. } => true,
            JobPayload::RecipeCandidate { adaptation, .. } => adaptation.0.is_some(),
            JobPayload::ProductObservation {
                approved_contribution,
                ..
            } => approved_contribution.0.is_some(),
            JobPayload::GenericCatalogue { .. }
            | JobPayload::BookingCheck { .. }
            | JobPayload::SetupCheck { .. } => false,
        }
    {
        return Err(DomainError::new(ErrorCode::PrivacyDenied, "/payload"));
    }
    Ok(())
}
pub fn validate_outbound(
    bundle: &OutboundBundle,
    policy: &PrivacyPolicy,
) -> Result<(), DomainError> {
    policy_payload(&bundle.request.payload, policy)?;
    if bundle.request.privacy_policy_revision != policy.revision {
        return Err(DomainError::new(
            ErrorCode::ContextStale,
            "/privacy_policy_revision",
        ));
    }
    bundle
        .request
        .validate_shape()
        .map_err(crate::parse::shape_error)?;
    if bundle.artefacts.len() > 64
        || bundle
            .artefacts
            .iter()
            .map(|a| a.bytes.len())
            .sum::<usize>()
            > MAX_ARTEFACT_BYTES
    {
        return Err(DomainError::new(ErrorCode::LimitExceeded, "/artefacts"));
    }
    if bundle.artefacts.len() != bundle.request.allowed_artefacts.len() {
        return Err(DomainError::new(ErrorCode::MissingReference, "/artefacts"));
    }
    let mut ids = std::collections::BTreeSet::new();
    for a in &bundle.artefacts {
        if !ids.insert(&a.artefact_id) {
            return Err(DomainError::new(ErrorCode::InvalidValue, "/artefacts"));
        }
        let declared = bundle
            .request
            .allowed_artefacts
            .iter()
            .find(|r| r.artefact_id == a.artefact_id)
            .ok_or_else(|| DomainError::new(ErrorCode::MissingReference, "/artefacts"))?;
        if a.bytes.len() != declared.byte_length as usize
            || bytes_digest(&a.bytes) != declared.digest
        {
            return Err(DomainError::new(ErrorCode::DigestMismatch, "/artefacts"));
        }
        let payload: JobPayload = parse_document(&a.bytes)?;
        policy_payload(&payload, policy)?;
        validate_fragment(&payload, &bundle.request)?;
    }
    Ok(())
}

/// A fragment inherits the containing request's authority. Narrow a copied scope
/// only after checking membership; never admit it as an independent request.
fn validate_fragment(payload: &JobPayload, request: &JobRequestV1) -> Result<(), DomainError> {
    if std::mem::discriminant(payload) != std::mem::discriminant(&request.payload) {
        return Err(DomainError::new(
            ErrorCode::ScopeViolation,
            "/artefacts/payload",
        ));
    }
    let mut scoped = request.clone();
    match (payload, &mut scoped.scope) {
        (JobPayload::PersonalisedPlan { requirements, .. }, ActionScope::Plan { slots, .. }) => {
            if requirements.iter().any(|r| !slots.contains(&r.slot)) {
                return Err(DomainError::new(
                    ErrorCode::ScopeViolation,
                    "/artefacts/requirements",
                ));
            }
            *slots = BoundedVec(requirements.iter().map(|r| r.slot.clone()).collect());
        }
        (
            JobPayload::CalendarPublication { batch, .. },
            ActionScope::Calendar { event_ids, .. },
        ) => {
            if batch
                .events
                .iter()
                .any(|e| !event_ids.contains(&e.event_id))
            {
                return Err(DomainError::new(
                    ErrorCode::ScopeViolation,
                    "/artefacts/events",
                ));
            }
            *event_ids = BoundedVec(batch.events.iter().map(|e| e.event_id.clone()).collect());
        }
        (
            JobPayload::BookingCheck { week_ids, .. },
            ActionScope::Booking {
                week_ids: permitted,
                ..
            },
        ) => {
            if week_ids.iter().any(|id| !permitted.contains(id)) {
                return Err(DomainError::new(
                    ErrorCode::ScopeViolation,
                    "/artefacts/week_ids",
                ));
            }
            *permitted = week_ids.clone();
        }
        _ => {}
    }
    scoped.payload = payload.clone();
    scoped.input_digest = input_digest(&RequestMaterial::from(&scoped))?;
    validate_request_shape(&scoped)
}

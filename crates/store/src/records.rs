use mstrchf_contracts::*;

pub(crate) fn storage(detail: impl std::fmt::Display) -> DomainError {
    #[cfg(test)]
    eprintln!("synthetic test storage diagnostic: {detail}");
    let _ = detail;
    DomainError::new(ErrorCode::StorageFailure, "/store")
}
pub(crate) fn unsupported() -> DomainError {
    DomainError::new(ErrorCode::UnsupportedCapability, "/command")
}
pub(crate) fn random_id() -> Result<String, DomainError> {
    loop {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(storage)?;
        if bytes.iter().any(|b| *b != 0) {
            return Ok(bytes.iter().map(|b| format!("{b:02x}")).collect());
        }
    }
}
pub(crate) fn initial(household_id: HouseholdId) -> Snapshot {
    Snapshot {
        schema_version: SchemaVersion,
        snapshot_revision: Revision(1),
        privacy: PrivacyPolicy {
            revision: Revision(1),
            use_ai_features: false,
            include_household_requirements: false,
            confirmed_at: Nullable(None),
        },
        household_draft: HouseholdDraft {
            schema_version: SchemaVersion,
            household_id,
            revision: Revision(1),
            completed_stages: BoundedVec(vec![]),
            diners: BoundedVec(vec![]),
            preferences: Nullable(None),
            usual_week: Nullable(None),
            connections: Nullable(None),
        },
        profile: Nullable(None),
        weeks: BoundedVec(vec![]),
        jobs: BoundedVec(vec![]),
        conflicts: BoundedVec(vec![]),
    }
}
pub(crate) fn terminal(s: JobState) -> bool {
    matches!(
        s,
        JobState::NeedsUser | JobState::Succeeded | JobState::Failed | JobState::Cancelled
    )
}

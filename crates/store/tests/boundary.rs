use mstrchf_contracts::*;
use mstrchf_store::{Store, StoreConfig};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

fn store() -> Store {
    let nonce = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "mstrchf-b1-boundary-{}-{}-{nonce}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    Store::open(StoreConfig {
        app_data_root: root,
    })
    .unwrap()
}

fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/contracts")
        .join(name);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn command(command_id: u64, expected_snapshot_revision: u64, command: AppCommand) -> AppCommandV1 {
    AppCommandV1 {
        schema_version: SchemaVersion,
        command_id: CommandId(format!("cm_{command_id:032x}")),
        expected_snapshot_revision: Revision(expected_snapshot_revision),
        command,
    }
}

fn initial_policy() -> PrivacyPolicy {
    PrivacyPolicy {
        revision: Revision(1),
        use_ai_features: false,
        include_household_requirements: false,
        confirmed_at: Nullable(None),
    }
}

fn draft_with_connections(policy: &PrivacyPolicy, household_id: &HouseholdId) -> HouseholdDraft {
    let mut draft = fixture("household-draft.json");
    let profile = fixture("household-profile.json");
    draft["household_id"] = serde_json::to_value(household_id).unwrap();
    draft["completed_stages"] =
        json!(["connections", "food_preferences", "household", "usual_week"]);
    draft["diners"] = profile["diners"].clone();
    draft["preferences"] = profile["preferences"].clone();
    draft["usual_week"] = profile["usual_week"].clone();
    let mut connections = profile["connections"].clone();
    connections["privacy"] = serde_json::to_value(policy).unwrap();
    draft["connections"] = connections;
    serde_json::from_value(draft).unwrap()
}

fn imported_result() -> Vec<u8> {
    serde_json::to_vec(&fixture("ruling004-current--job-result.json")).unwrap()
}

#[test]
fn fresh_store_has_private_empty_state_and_reopens_identically() {
    let config = StoreConfig {
        app_data_root: std::env::temp_dir().join(format!(
            "mstrchf-b1-reopen-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        )),
    };
    let first = Store::open(StoreConfig {
        app_data_root: config.app_data_root.clone(),
    })
    .unwrap();
    let snapshot = first.snapshot().unwrap();
    assert_eq!(snapshot.snapshot_revision, Revision(1));
    assert_eq!(snapshot.privacy, initial_policy());
    assert!(snapshot.household_draft.connections.0.is_none());
    assert!(snapshot.profile.0.is_none());
    assert!(snapshot.weeks.is_empty());
    assert!(snapshot.jobs.is_empty());
    assert!(snapshot.conflicts.is_empty());
    drop(first);

    let reopened = Store::open(StoreConfig {
        app_data_root: config.app_data_root.clone(),
    })
    .unwrap();
    assert_eq!(reopened.snapshot().unwrap(), snapshot);
}

#[test]
fn household_connections_copy_authoritative_privacy_and_follow_revision_changes() {
    let store = store();
    let base = store.snapshot().unwrap();
    let saved = store
        .dispatch_ui(command(
            1,
            base.snapshot_revision.0,
            AppCommand::SaveHouseholdDraft {
                draft: draft_with_connections(&base.privacy, &base.household_draft.household_id),
            },
        ))
        .unwrap();
    assert_eq!(saved.snapshot_revision, Revision(2));
    assert_eq!(
        saved
            .household_draft
            .connections
            .0
            .as_ref()
            .unwrap()
            .privacy,
        saved.privacy
    );

    let policy = PrivacyPolicy {
        revision: saved.privacy.revision,
        use_ai_features: true,
        include_household_requirements: false,
        confirmed_at: Nullable(Some(Instant::try_from("2026-12-28T12:00:00Z").unwrap())),
    };
    let updated = store
        .dispatch_ui(command(
            2,
            saved.snapshot_revision.0,
            AppCommand::SetPrivacy {
                policy: policy.clone(),
                expected_policy_revision: saved.privacy.revision,
            },
        ))
        .unwrap();
    assert_eq!(updated.snapshot_revision, Revision(3));
    assert_eq!(updated.privacy.revision, Revision(2));
    assert_eq!(updated.privacy.use_ai_features, policy.use_ai_features);
    assert_eq!(
        updated.privacy.include_household_requirements,
        policy.include_household_requirements
    );
    assert_eq!(updated.privacy.confirmed_at, policy.confirmed_at);
    assert_eq!(
        updated
            .household_draft
            .connections
            .0
            .as_ref()
            .unwrap()
            .privacy,
        updated.privacy
    );
}

#[test]
fn null_privacy_copy_survives_revision_two_reopen_and_revision_three_update() {
    let root = std::env::temp_dir().join(format!(
        "mstrchf-b1-null-privacy-{}-{}",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    let store = Store::open(StoreConfig {
        app_data_root: root.clone(),
    })
    .unwrap();
    let initial = store.snapshot().unwrap();
    let mut draft = fixture("household-draft.json");
    draft["household_id"] = serde_json::to_value(&initial.household_draft.household_id).unwrap();
    let partial: HouseholdDraft = serde_json::from_value(draft).unwrap();
    let saved = store
        .dispatch_ui(command(
            30,
            initial.snapshot_revision.0,
            AppCommand::SaveHouseholdDraft { draft: partial },
        ))
        .unwrap();
    assert!(saved.household_draft.completed_stages.0.is_empty());
    assert!(saved.household_draft.connections.0.is_none());

    let first_policy = PrivacyPolicy {
        revision: saved.privacy.revision,
        use_ai_features: true,
        include_household_requirements: false,
        confirmed_at: Nullable(Some(Instant::try_from("2026-12-28T12:00:00Z").unwrap())),
    };
    let at_revision_two = store
        .dispatch_ui(command(
            31,
            saved.snapshot_revision.0,
            AppCommand::SetPrivacy {
                policy: first_policy,
                expected_policy_revision: saved.privacy.revision,
            },
        ))
        .unwrap();
    assert_eq!(at_revision_two.privacy.revision, Revision(2));
    assert!(at_revision_two.household_draft.connections.0.is_none());
    drop(store);

    let reopened = Store::open(StoreConfig {
        app_data_root: root,
    })
    .unwrap();
    assert_eq!(reopened.snapshot().unwrap(), at_revision_two);
    let second_policy = PrivacyPolicy {
        revision: at_revision_two.privacy.revision,
        use_ai_features: false,
        include_household_requirements: true,
        confirmed_at: Nullable(Some(Instant::try_from("2026-12-28T12:01:00Z").unwrap())),
    };
    let at_revision_three = reopened
        .dispatch_ui(command(
            32,
            at_revision_two.snapshot_revision.0,
            AppCommand::SetPrivacy {
                policy: second_policy,
                expected_policy_revision: at_revision_two.privacy.revision,
            },
        ))
        .unwrap();
    assert_eq!(at_revision_three.privacy.revision, Revision(3));
    assert!(at_revision_three.household_draft.connections.0.is_none());
}

#[test]
fn identity_cas_invalid_value_and_privacy_injection_rejections_leave_state_unchanged() {
    let store = store();
    let before = store.snapshot().unwrap();

    let mut foreign_draft =
        draft_with_connections(&before.privacy, &before.household_draft.household_id);
    foreign_draft.household_id = HouseholdId("hh_00000000000000000000000000000002".into());
    let error = store
        .dispatch_ui(command(
            1,
            before.snapshot_revision.0,
            AppCommand::SaveHouseholdDraft {
                draft: foreign_draft,
            },
        ))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::IdentityMismatch);
    assert_eq!(store.snapshot().unwrap(), before);

    let error = store
        .dispatch_ui(command(
            2,
            before.snapshot_revision.0 + 1,
            AppCommand::SaveHouseholdDraft {
                draft: draft_with_connections(
                    &before.privacy,
                    &before.household_draft.household_id,
                ),
            },
        ))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::ContextStale);
    assert_eq!(store.snapshot().unwrap(), before);

    let mut invalid_draft =
        draft_with_connections(&before.privacy, &before.household_draft.household_id);
    invalid_draft.revision = Revision(0);
    let error = store
        .dispatch_ui(command(
            3,
            before.snapshot_revision.0,
            AppCommand::SaveHouseholdDraft {
                draft: invalid_draft,
            },
        ))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::NoncanonicalValue);
    assert_eq!(store.snapshot().unwrap(), before);

    let mut injected_policy = before.privacy.clone();
    injected_policy.use_ai_features = true;
    let error = store
        .dispatch_ui(command(
            4,
            before.snapshot_revision.0,
            AppCommand::SaveHouseholdDraft {
                draft: draft_with_connections(
                    &injected_policy,
                    &before.household_draft.household_id,
                ),
            },
        ))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::ContextStale);
    assert!(error.issues.iter().any(|issue| {
        issue.path == "/command/data/draft/connections/privacy"
            && issue.code == ErrorCode::ContextStale
    }));
    assert_eq!(store.snapshot().unwrap(), before);
}

#[test]
fn exact_replay_returns_original_response_after_intervening_state_change() {
    let store = store();
    let initial = store.snapshot().unwrap();
    let first_command = command(
        10,
        initial.snapshot_revision.0,
        AppCommand::SaveHouseholdDraft {
            draft: {
                let mut draft = fixture("household-draft.json");
                draft["household_id"] =
                    serde_json::to_value(&initial.household_draft.household_id).unwrap();
                serde_json::from_value(draft).unwrap()
            },
        },
    );
    let first_response = store.dispatch_ui(first_command.clone()).unwrap();

    let policy = PrivacyPolicy {
        revision: Revision(1),
        use_ai_features: true,
        include_household_requirements: false,
        confirmed_at: Nullable(Some(Instant::try_from("2026-12-28T12:00:00Z").unwrap())),
    };
    store
        .dispatch_ui(command(
            11,
            first_response.snapshot_revision.0,
            AppCommand::SetPrivacy {
                policy,
                expected_policy_revision: Revision(1),
            },
        ))
        .unwrap();

    assert_eq!(store.dispatch_ui(first_command).unwrap(), first_response);
    assert_ne!(store.snapshot().unwrap(), first_response);
}

#[test]
fn command_id_reuse_with_changed_data_or_expected_revision_is_a_replay_conflict() {
    let store = store();
    let initial = store.snapshot().unwrap();
    let original = command(
        20,
        initial.snapshot_revision.0,
        AppCommand::SaveHouseholdDraft {
            draft: {
                let mut draft = fixture("household-draft.json");
                draft["household_id"] =
                    serde_json::to_value(&initial.household_draft.household_id).unwrap();
                serde_json::from_value(draft).unwrap()
            },
        },
    );
    let response = store.dispatch_ui(original.clone()).unwrap();

    let mut changed_draft_value = fixture("household-draft.json");
    changed_draft_value["household_id"] =
        serde_json::to_value(&initial.household_draft.household_id).unwrap();
    let mut changed_draft: HouseholdDraft = serde_json::from_value(changed_draft_value).unwrap();
    changed_draft.revision = Revision(2);
    let changed_data = command(
        20,
        initial.snapshot_revision.0,
        AppCommand::SaveHouseholdDraft {
            draft: changed_draft,
        },
    );
    let error = store.dispatch_ui(changed_data).unwrap_err();
    assert_eq!(error.code, ErrorCode::ReplayConflict);
    assert_eq!(store.snapshot().unwrap(), response);

    let changed_expected_revision = command(20, initial.snapshot_revision.0 + 1, original.command);
    let error = store.dispatch_ui(changed_expected_revision).unwrap_err();
    assert_eq!(error.code, ErrorCode::ReplayConflict);
    assert_eq!(store.snapshot().unwrap(), response);
}

#[test]
fn caller_authority_matrix_matches_all_contract_command_kinds() {
    let callers = [
        CallerClass::LocalUi,
        CallerClass::ExchangeImport,
        CallerClass::QualifiedAdapter,
    ];
    let mut checked = 0;
    for kind in CommandKind::ALL {
        for caller in callers {
            let permitted = caller == CallerClass::LocalUi
                || caller == CallerClass::QualifiedAdapter && kind == CommandKind::ConfirmOrder;
            match (permitted, authorize_command(kind, caller)) {
                (true, Ok(())) => {}
                (false, Err(error)) => assert_eq!(error.code, ErrorCode::AuthorityDenied),
                (expected, actual) => {
                    panic!("{kind:?}/{caller:?}: expected permitted={expected}, got {actual:?}")
                }
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 36);
}

#[test]
fn nine_unsupported_ui_commands_are_rejected_without_state_changes() {
    let store = store();
    let before = store.snapshot().unwrap();
    let profile: HouseholdProfile =
        serde_json::from_value(fixture("household-profile.json")).unwrap();
    let week: Week = serde_json::from_value(fixture("week.json")).unwrap();
    let request: JobRequestV1 = serde_json::from_value(fixture(
        "ruling004-current--job-request-product-observation.json",
    ))
    .unwrap();
    let calendar_request: JobRequestV1 = serde_json::from_value(fixture(
        "ruling004-current--job-request-calendar-publication.json",
    ))
    .unwrap();
    let operation: OperationRequest =
        serde_json::from_value(fixture("ruling004-current--operation-request.json")).unwrap();
    let calendar_event = match calendar_request.payload {
        JobPayload::CalendarPublication { batch, .. } => batch.events.0[0].clone(),
        _ => unreachable!(),
    };
    let id = |prefix: &str| format!("{prefix}{:032x}", 1);
    let digest = Digest("a".repeat(64));
    let instant = Instant::try_from("2026-12-28T12:00:00Z").unwrap();
    let commands = vec![
        AppCommand::UpdateProfile {
            profile,
            expected_profile_revision: Revision(1),
        },
        AppCommand::SaveWeek {
            week: WeekEdit {
                week_id: week.range.week_id.clone(),
                defaults_snapshot: week.defaults_snapshot,
                day_enabled: week.day_enabled,
                meal_type_enabled: week.meal_type_enabled,
                cells: week.cells,
            },
            expected_week_revision: Revision(1),
        },
        AppCommand::ApplyPlanProposal {
            proposal: PlanProposal {
                changes: BoundedVec(vec![WeekChange {
                    week_id: WeekId(id("wk_")),
                    base_revision: Revision(1),
                    replace_slots: BoundedVec(vec![]),
                }]),
            },
            context_stamp: ContextStamp {
                household_revision: Revision(1),
                settings_revision: Revision(1),
                privacy_revision: Revision(1),
                history_revision: Revision(1),
                catalogue_revision: Revision(1),
                ranking_policy_version: Revision(1),
                recipe_refs: BoundedVec(vec![]),
                read_weeks: BoundedVec(vec![]),
                write_weeks: BoundedVec(vec![]),
            },
        },
        AppCommand::ConfirmEssentials {
            confirmations: BoundedVec(vec![EssentialsConfirmation {
                line_id: DemandLineId(id("dl_")),
                choice: EssentialsChoice::HaveEnough,
                displayed_quantity: Quantity {
                    amount: PositiveRational(Rational::new(1, 1).unwrap()),
                    unit: Unit::G,
                    form: IngredientForm::Raw,
                },
                demand_digest: DemandDigest("a".repeat(64)),
                confirmed_at: instant.clone(),
            }]),
        },
        AppCommand::ApprovePlan {
            week_id: WeekId(id("wk_")),
            plan_revision: Revision(1),
            derived_digest: PayloadDigest("a".repeat(64)),
        },
        AppCommand::ConfirmOrder {
            confirmation: OrderConfirmation {
                week_id: WeekId(id("wk_")),
                approval_id: ApprovalId(id("au_")),
                delivery_start: calendar_event.start,
                delivery_end: calendar_event.end,
                confirmed_at: instant.clone(),
                source: OrderConfirmationSource::UserConfirmation {},
            },
        },
        AppCommand::RequestJob {
            action: request.action,
            scope: request.scope,
        },
        AppCommand::RecordUserStop {
            worker: operation.worker,
            qualification_id: EvidenceId(id("ev_")),
            procedure_version: Revision(1),
            visible_status: VisibleTerminalStatus::Completed,
        },
        AppCommand::RequestCalendarPublication {
            order_id: OrderId(id("or_")),
            destination_id: CalendarDestinationId(id("cd_")),
            event_batch_digest: digest,
        },
    ];
    let expected_kinds = [
        CommandKind::UpdateProfile,
        CommandKind::SaveWeek,
        CommandKind::ApplyPlanProposal,
        CommandKind::ConfirmEssentials,
        CommandKind::ApprovePlan,
        CommandKind::ConfirmOrder,
        CommandKind::RequestJob,
        CommandKind::RecordUserStop,
        CommandKind::RequestCalendarPublication,
    ];
    assert_eq!(commands.len(), expected_kinds.len());
    for (index, (ui_command, expected_kind)) in commands.into_iter().zip(expected_kinds).enumerate()
    {
        assert_eq!(ui_command.kind(), expected_kind);
        let error = store
            .dispatch_ui(command(
                (index + 100) as u64,
                before.snapshot_revision.0,
                ui_command,
            ))
            .unwrap_err();
        assert_eq!(
            error.code,
            ErrorCode::UnsupportedCapability,
            "{expected_kind:?}"
        );
        assert_eq!(store.snapshot().unwrap(), before, "{expected_kind:?}");
    }
}

#[test]
fn unrequested_import_result_is_rejected_without_state_changes() {
    let store = store();
    let before = store.snapshot().unwrap();
    let outcome = store.import_result(&imported_result());
    match outcome {
        ImportOutcome::Rejected { error } => {
            assert_eq!(error.code, ErrorCode::UnsupportedCapability)
        }
        other => panic!("unexpected import outcome: {other:?}"),
    }
    assert_eq!(store.snapshot().unwrap(), before);
}

#[test]
fn every_partial_onboarding_stage_persists_without_profile_promotion() {
    let path = std::env::temp_dir().join(format!(
        "MSTRCHF partial ü {} {}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut store = Store::open(StoreConfig {
        app_data_root: path.clone(),
    })
    .unwrap();
    let profile: HouseholdProfile =
        parse_document(&serde_json::to_vec(&fixture("household-profile.json")).unwrap()).unwrap();
    for step in 0..5 {
        let before = store.snapshot().unwrap();
        let mut draft = before.household_draft.clone();
        match step {
            0 => {}
            1 => {
                draft.diners = BoundedVec(profile.diners.0.clone());
                draft.completed_stages.0.push(HouseholdStage::Household);
            }
            2 => {
                draft.preferences = Nullable(Some(profile.preferences.clone()));
                draft
                    .completed_stages
                    .0
                    .insert(0, HouseholdStage::FoodPreferences);
            }
            3 => {
                draft.usual_week = Nullable(Some(profile.usual_week.clone()));
                draft.completed_stages.0.push(HouseholdStage::UsualWeek);
            }
            4 => {
                let mut connections = profile.connections.clone();
                connections.privacy = before.privacy;
                draft.connections = Nullable(Some(connections));
                draft
                    .completed_stages
                    .0
                    .insert(0, HouseholdStage::Connections);
            }
            _ => unreachable!(),
        }
        let result = store
            .dispatch_ui(command(
                step + 1,
                before.snapshot_revision.0,
                AppCommand::SaveHouseholdDraft { draft },
            ))
            .unwrap();
        assert!(result.profile.0.is_none());
        assert!(result.weeks.is_empty());
        assert_eq!(result.household_draft.completed_stages.len(), step as usize);
        drop(store);
        store = Store::open(StoreConfig {
            app_data_root: path.clone(),
        })
        .unwrap();
        assert_eq!(store.snapshot().unwrap(), result);
    }
}

#[test]
fn invalid_stage_flags_and_imported_command_cannot_mutate() {
    let store = store();
    let before = store.snapshot().unwrap();
    let mut draft = before.household_draft.clone();
    draft.completed_stages = BoundedVec(vec![HouseholdStage::Connections]);
    let c = command(1, 1, AppCommand::SaveHouseholdDraft { draft });
    assert_eq!(
        store.dispatch_ui(c).unwrap_err().code,
        ErrorCode::InvalidValue
    );
    let c = command(
        2,
        1,
        AppCommand::SaveHouseholdDraft {
            draft: before.household_draft.clone(),
        },
    );
    assert!(matches!(
        store.import_result(&canonical_bytes(&c).unwrap()),
        ImportOutcome::Rejected { .. }
    ));
    assert_eq!(store.snapshot().unwrap(), before);
}

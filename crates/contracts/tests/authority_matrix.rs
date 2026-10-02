use mstrchf_contracts::*;
use serde_json::Value;

fn id(s: &str) -> String {
    format!("{s}{:032x}", 1)
}
fn digest() -> Digest {
    Digest("a".repeat(64))
}
fn time(s: &str) -> Instant {
    Instant::try_from(s).unwrap()
}
fn text(s: &str) -> Text {
    Text(s.into())
}
fn product() -> ProductRef {
    ProductRef {
        retailer: Retailer::Tesco,
        product_id: text("sku"),
        variant_id: Nullable(None),
    }
}
fn key() -> BasketKey {
    BasketKey {
        retailer: Retailer::Tesco,
        account_ref: text("account"),
        basket_id: BasketId(id("bk_")),
    }
}
fn worker() -> WorkerIdentity {
    WorkerIdentity {
        provider: Provider::CodexLocal,
        provider_build: text("build"),
        native_task_ref: text("task"),
        job_id: JobId(id("jb_")),
        generation: Generation(1),
    }
}
fn stamp() -> ContextStamp {
    ContextStamp {
        household_revision: Revision(1),
        settings_revision: Revision(1),
        privacy_revision: Revision(1),
        history_revision: Revision(1),
        catalogue_revision: Revision(1),
        ranking_policy_version: Revision(1),
        recipe_refs: BoundedVec(vec![]),
        read_weeks: BoundedVec(vec![]),
        write_weeks: BoundedVec(vec![]),
    }
}
fn read_scope() -> ReadScope {
    ReadScope {
        read_id: ReadId(id("rd_")),
        resource: ExternalResource::Basket { key: key() },
        purpose: ReadPurpose::Initial,
        allowed_kinds: BoundedVec(vec![ReadKind::Products]),
        products: BoundedVec(vec![]),
        event_ids: BoundedVec(vec![]),
        not_before: time("2026-09-27T10:00:00Z"),
        expires_at: time("2026-09-27T10:10:00Z"),
    }
}
fn job() -> JobRequestV1 {
    let scope = read_scope();
    JobRequestV1 {
        schema_version: SchemaVersion,
        job_id: JobId(id("jb_")),
        action: Action::PrepareShop,
        week_id: Nullable(Some(WeekId(id("wk_")))),
        base_plan_revision: Nullable(Some(Revision(1))),
        context_stamp: stamp(),
        input_digest: InputDigest("b".repeat(64)),
        privacy_policy_revision: Revision(1),
        required_capabilities: BoundedVec(vec![Capability::BrowserRead]),
        allowed_artefacts: BoundedVec(vec![]),
        payload: JobPayload::ProductObservation {
            read_scope: scope.clone(),
            products: BoundedVec(vec![]),
            approved_contribution: Nullable(None),
        },
        scope: ActionScope::Shop {
            key: key(),
            phase: ShopPhase::Observe,
            approval_id: Nullable(None),
            read_scope: scope,
        },
        created_at: time("2026-09-27T09:59:00Z"),
        expires_at: time("2026-09-27T10:10:00Z"),
    }
}
fn reader() -> ReadActor {
    ReadActor {
        provider: Provider::CodexLocal,
        native_task_ref: text("task"),
    }
}
fn observation() -> ReadObservationV1 {
    ReadObservationV1 {
        schema_version: SchemaVersion,
        observation_id: ReadObservationId(id("ob_")),
        read_id: ReadId(id("rd_")),
        job_id: JobId(id("jb_")),
        input_digest: InputDigest("b".repeat(64)),
        reader: reader(),
        resource: ExternalResource::Basket { key: key() },
        observed_at: time("2026-09-27T10:01:00Z"),
        body: ReadBody::Products {
            delivery_context: RetailerDeliveryContext {
                retailer: Retailer::Tesco,
                account_ref: text("account"),
                context_ref: text("delivery"),
                delivery_date: Nullable(None),
                slot_ref: Nullable(None),
            },
            items: BoundedVec(vec![]),
        },
    }
}

#[test]
fn initial_read_requires_no_operation_grant_and_empty_discovery_is_valid() {
    let o = observation();
    validate_read_binding(&o, &job(), &reader(), time("2026-09-27T10:01:01Z")).unwrap();
    let wire = serde_json::to_value(&o).unwrap();
    assert!(wire.get("grant_token").is_none());
    assert!(wire.get("operation_id").is_none());
    let mut forged = wire;
    forged
        .as_object_mut()
        .unwrap()
        .insert("grant_token".into(), serde_json::json!("f".repeat(32)));
    assert!(serde_json::from_value::<ReadObservationV1>(forged).is_err());
}

#[test]
fn read_admission_rejects_identity_kind_time_and_resource_drift() {
    let base = observation();
    let j = job();
    let admission = time("2026-09-27T10:01:01Z");
    let mut o = base.clone();
    o.reader.native_task_ref = text("other");
    assert_eq!(
        validate_read_binding(&o, &j, &reader(), admission.clone())
            .unwrap_err()
            .code,
        ErrorCode::IdentityMismatch
    );
    let mut o = base.clone();
    o.input_digest = InputDigest("c".repeat(64));
    assert_eq!(
        validate_read_binding(&o, &j, &reader(), admission.clone())
            .unwrap_err()
            .code,
        ErrorCode::DigestMismatch
    );
    let mut o = base.clone();
    o.body = ReadBody::Slots {
        delivery_context: RetailerDeliveryContext {
            retailer: Retailer::Tesco,
            account_ref: text("account"),
            context_ref: text("delivery"),
            delivery_date: Nullable(None),
            slot_ref: Nullable(None),
        },
        items: BoundedVec(vec![]),
    };
    assert_eq!(
        validate_read_binding(&o, &j, &reader(), admission.clone())
            .unwrap_err()
            .code,
        ErrorCode::ScopeViolation
    );
    let mut o = base.clone();
    o.observed_at = time("2026-09-27T10:20:00Z");
    assert_eq!(
        validate_read_binding(&o, &j, &reader(), admission.clone())
            .unwrap_err()
            .code,
        ErrorCode::InvalidValue
    );
    let mut o = base;
    o.resource = ExternalResource::Basket {
        key: BasketKey {
            account_ref: text("other"),
            ..key()
        },
    };
    assert_eq!(
        validate_read_binding(&o, &j, &reader(), admission)
            .unwrap_err()
            .code,
        ErrorCode::IdentityMismatch
    );
}

#[test]
fn basket_read_rejects_duplicate_lines_and_unrequested_products() {
    let mut o = observation();
    let mut j = job();
    if let ActionScope::Shop { read_scope, .. } = &mut j.scope {
        read_scope.allowed_kinds.0.push(ReadKind::Basket);
    }
    if let JobPayload::ProductObservation { read_scope, .. } = &mut j.payload {
        read_scope.allowed_kinds.0.push(ReadKind::Basket);
    }
    let line = BasketLine {
        product: product(),
        quantity: 1,
        unit_price: Price::Unknown {
            reason: UnknownReason::NotExposed,
        },
    };
    o.body = ReadBody::Basket {
        snapshot: BasketSnapshot {
            key: key(),
            delivery_context: RetailerDeliveryContext {
                retailer: Retailer::Tesco,
                account_ref: text("account"),
                context_ref: text("delivery"),
                delivery_date: Nullable(None),
                slot_ref: Nullable(None),
            },
            observed_at: time("2026-09-27T10:01:00Z"),
            lines: BoundedVec(vec![line.clone(), line.clone()]),
            delivery_charge: Price::Unknown {
                reason: UnknownReason::Missing,
            },
        },
    };
    assert_eq!(
        validate_read_shape(&o).unwrap_err().code,
        ErrorCode::InvalidValue
    );
    if let ReadBody::Basket { snapshot } = &mut o.body {
        snapshot.lines.0.pop();
    }
    validate_read_binding(&o, &j, &reader(), time("2026-09-27T10:01:01Z")).unwrap();
    if let ActionScope::Shop { read_scope, .. } = &mut j.scope {
        read_scope.products.0.push(ProductRef {
            product_id: text("other"),
            ..product()
        });
    }
    if let JobPayload::ProductObservation { read_scope, .. } = &mut j.payload {
        read_scope.products.0.push(ProductRef {
            product_id: text("other"),
            ..product()
        });
    }
    assert_eq!(
        validate_read_binding(&o, &j, &reader(), time("2026-09-27T10:01:01Z"))
            .unwrap_err()
            .code,
        ErrorCode::ScopeViolation
    );
}

fn basket_action(delta: i32) -> ExternalAction {
    ExternalAction::BasketSetQuantity {
        product_ref: product(),
        expected_observed_quantity: 4,
        target_quantity: (4_i32 + delta) as u32,
        planner_delta: delta,
        product_evidence_digest: digest(),
        price_approval_digest: digest(),
        baseline_digest: digest(),
    }
}
fn approval(removal: bool, attribution: bool) -> ApprovedContributionRef {
    ApprovedContributionRef {
        approval_id: ApprovalId(id("au_")),
        derived_digest: PayloadDigest("d".repeat(64)),
        product_evidence_digest: digest(),
        price_approval_digest: digest(),
        lines: BoundedVec(vec![ApprovedProductLine {
            product: product(),
            expected_quantity: 4,
            target_quantity: 3,
            removal_approved: removal,
            planner_attribution_digest: Nullable(attribution.then(digest)),
        }]),
    }
}

#[test]
fn negative_delta_requires_explicit_removal_and_attribution() {
    let action = basket_action(-1);
    validate_external_action(&action, &ExternalResource::Basket { key: key() }).unwrap();
    assert_eq!(
        validate_operation_approval(&action, &approval(false, true))
            .unwrap_err()
            .code,
        ErrorCode::AuthorityDenied
    );
    assert_eq!(
        validate_operation_approval(&action, &approval(true, false))
            .unwrap_err()
            .code,
        ErrorCode::AuthorityDenied
    );
    validate_operation_approval(&action, &approval(true, true)).unwrap();
    let mut malformed = action;
    if let ExternalAction::BasketSetQuantity {
        target_quantity, ..
    } = &mut malformed
    {
        *target_quantity = 2;
    }
    assert_eq!(
        validate_external_action(&malformed, &ExternalResource::Basket { key: key() })
            .unwrap_err()
            .code,
        ErrorCode::InvalidValue
    );
    assert_eq!(
        validate_operation_approval(&malformed, &approval(true, true))
            .unwrap_err()
            .code,
        ErrorCode::InvalidValue
    );
}

fn lock() -> BasketLock {
    let w = worker();
    BasketLock {
        key: key(),
        worker: w.clone(),
        state: BasketLockState::Quarantined,
        outstanding_operation: Nullable(None),
        stop_evidence: Nullable(Some(WorkerStopEvidence {
            schema_version: SchemaVersion,
            worker: w.clone(),
            qualification_id: EvidenceId(id("ev_")),
            source: StopSource::NativeTerminal {
                status: NativeTerminalStatus::Completed,
                provider_evidence_id: EvidenceId(id("ev_")),
                observed_at: time("2026-09-27T10:00:00Z"),
            },
            recorded_at: time("2026-09-27T10:00:01Z"),
        })),
        last_reconciliation: Nullable(Some(ReconciliationEvidence {
            evidence_id: EvidenceId(id("ev_")),
            key: key(),
            old_worker: w,
            observed_at: time("2026-09-27T10:00:02Z"),
            basket_digest: digest(),
            unresolved_operation_ids: BoundedVec(vec![]),
            manual_conflicts: false,
        })),
    }
}
fn policy() -> PrivacyPolicy {
    PrivacyPolicy {
        revision: Revision(1),
        use_ai_features: true,
        include_household_requirements: true,
        confirmed_at: Nullable(Some(time("2026-09-27T09:00:00Z"))),
    }
}
fn resume_facts<'a>(
    lock: &'a BasketLock,
    context: &'a ContextStamp,
    policy: &'a PrivacyPolicy,
) -> ResumeFacts<'a> {
    ResumeFacts {
        lock,
        stop_source_trust: StopSourceTrust::QualifiedProvider,
        qualification_matches: true,
        reconciliation: lock.last_reconciliation.0.as_ref(),
        current_context: context,
        expected_context: context,
        policy,
        approval_current: true,
    }
}

#[test]
fn recovery_needs_qualified_stop_then_fresh_clean_reconciliation() {
    let c = stamp();
    let p = policy();
    let mut l = lock();
    may_resume_basket(&resume_facts(&l, &c, &p)).unwrap();
    let mut f = resume_facts(&l, &c, &p);
    f.stop_source_trust = StopSourceTrust::Untrusted;
    assert_eq!(
        may_resume_basket(&f).unwrap_err().code,
        ErrorCode::WorkerNotStopped
    );
    l.last_reconciliation.0.as_mut().unwrap().observed_at = time("2026-09-27T10:00:01Z");
    assert_eq!(
        may_resume_basket(&resume_facts(&l, &c, &p))
            .unwrap_err()
            .code,
        ErrorCode::ReconciliationRequired
    );
    l.last_reconciliation.0.as_mut().unwrap().observed_at = time("2026-09-27T10:00:02Z");
    l.last_reconciliation.0.as_mut().unwrap().manual_conflicts = true;
    assert_eq!(
        may_resume_basket(&resume_facts(&l, &c, &p))
            .unwrap_err()
            .code,
        ErrorCode::ReconciliationRequired
    );
    l.last_reconciliation.0.as_mut().unwrap().manual_conflicts = false;
    let mut f = resume_facts(&l, &c, &p);
    f.approval_current = false;
    assert_eq!(
        may_resume_basket(&f).unwrap_err().code,
        ErrorCode::AuthorityDenied
    );
    let mut denied = p.clone();
    denied.use_ai_features = false;
    let f = resume_facts(&l, &c, &denied);
    assert_eq!(
        may_resume_basket(&f).unwrap_err().code,
        ErrorCode::PrivacyDenied
    );
}

fn other_product() -> ProductRef {
    ProductRef {
        product_id: text("other-sku"),
        ..product()
    }
}
fn basket_read_case() -> (JobRequestV1, ReadObservationV1) {
    let mut j = job();
    if let ActionScope::Shop { read_scope, .. } = &mut j.scope {
        read_scope.allowed_kinds.0.push(ReadKind::Basket);
    }
    if let JobPayload::ProductObservation { read_scope, .. } = &mut j.payload {
        read_scope.allowed_kinds.0.push(ReadKind::Basket);
    }
    let mut o = observation();
    o.body = ReadBody::Basket {
        snapshot: BasketSnapshot {
            key: key(),
            delivery_context: RetailerDeliveryContext {
                retailer: Retailer::Tesco,
                account_ref: text("account"),
                context_ref: text("delivery"),
                delivery_date: Nullable(None),
                slot_ref: Nullable(None),
            },
            observed_at: time("2026-09-27T10:01:00Z"),
            lines: BoundedVec(vec![BasketLine {
                product: product(),
                quantity: 1,
                unit_price: Price::Unknown {
                    reason: UnknownReason::NotExposed,
                },
            }]),
            delivery_charge: Price::Unknown {
                reason: UnknownReason::NotExposed,
            },
        },
    };
    (j, o)
}
fn zone_moment(instant: &str) -> ZonedMoment {
    ZonedMoment {
        local_date: LocalDate("2026-09-27".into()),
        local_time: LocalTime(instant[11..19].into()),
        zone: ZoneId("Etc/UTC".into()),
        resolution: Resolution::Unambiguous,
        instant: time(instant),
    }
}
fn calendar_read_case() -> (JobRequestV1, ReadObservationV1) {
    let destination_id = CalendarDestinationId(id("cd_"));
    let event_id = CalendarEventId(id("ce_"));
    let mut scope = read_scope();
    scope.resource = ExternalResource::Calendar {
        destination_id: destination_id.clone(),
    };
    scope.allowed_kinds = BoundedVec(vec![ReadKind::CalendarEvents]);
    scope.event_ids = BoundedVec(vec![event_id.clone()]);
    let mut j = job();
    j.action = Action::FinishWeek;
    j.scope = ActionScope::Calendar {
        destination_id: destination_id.clone(),
        event_ids: BoundedVec(vec![event_id.clone()]),
        approval_id: ApprovalId(id("au_")),
        read_scope: scope.clone(),
    };
    j.payload = JobPayload::CalendarPublication {
        batch: EventBatch {
            week_id: WeekId(id("wk_")),
            order_id: OrderId(id("or_")),
            approved_plan_digest: PayloadDigest("d".repeat(64)),
            destination_id: destination_id.clone(),
            events: BoundedVec(vec![CalendarEvent {
                event_id: event_id.clone(),
                logical_key: CalendarEventLogicalKey {
                    week_id: WeekId(id("wk_")),
                    kind: CalendarEventKind::Delivery,
                    session_id: Nullable(None),
                    order_id: Nullable(Some(OrderId(id("or_")))),
                },
                payload_revision: Revision(1),
                start: zone_moment("2026-09-27T11:00:00Z"),
                end: zone_moment("2026-09-27T12:00:00Z"),
                title: Title("Delivery".into()),
                body: LongText("Delivery".into()),
                event_payload_digest: PayloadDigest("e".repeat(64)),
            }]),
        },
        read_scope: scope,
    };
    let mut o = observation();
    o.resource = ExternalResource::Calendar { destination_id };
    o.body = ReadBody::CalendarEvents {
        items: BoundedVec(vec![CalendarObservation {
            event_id,
            provider_event_ref: Nullable(None),
            observed_payload_digest: Nullable(None),
            state: CalendarObservationState::Absent,
            observed_at: time("2026-09-27T10:01:00Z"),
            user_changed: false,
        }]),
    };
    (j, o)
}

#[test]
fn named_read_binding_matrix() {
    const CASES: &[(&str, ErrorCode)] = &[
        ("read.job_identity", ErrorCode::IdentityMismatch),
        ("read.read_identity", ErrorCode::IdentityMismatch),
        ("read.input_digest", ErrorCode::DigestMismatch),
        ("read.reader_task", ErrorCode::IdentityMismatch),
        ("read.reader_provider", ErrorCode::IdentityMismatch),
        ("read.resource_key", ErrorCode::IdentityMismatch),
        ("read.account_context", ErrorCode::IdentityMismatch),
        ("read.product_scope", ErrorCode::ScopeViolation),
        ("read.event_scope", ErrorCode::ScopeViolation),
        ("read.time_before_scope", ErrorCode::InvalidValue),
        ("read.time_after_scope", ErrorCode::InvalidValue),
        ("read.future_clock_skew", ErrorCode::InvalidValue),
        ("read.kind_scope", ErrorCode::ScopeViolation),
        ("read.payload_scope_divergence", ErrorCode::ScopeViolation),
    ];
    for (name, expected) in CASES {
        let (mut j, mut o) = match *name {
            "read.product_scope" => basket_read_case(),
            "read.event_scope" => calendar_read_case(),
            _ => (job(), observation()),
        };
        let mut admission = time("2026-09-27T10:01:01Z");
        match *name {
            "read.job_identity" => o.job_id = JobId(format!("jb_{:032x}", 2)),
            "read.read_identity" => o.read_id = ReadId(format!("rd_{:032x}", 2)),
            "read.input_digest" => o.input_digest = InputDigest("c".repeat(64)),
            "read.reader_task" => o.reader.native_task_ref = text("other-task"),
            "read.reader_provider" => o.reader.provider = Provider::ClaudeCowork,
            "read.resource_key" => {
                o.resource = ExternalResource::Basket {
                    key: BasketKey {
                        basket_id: BasketId(format!("bk_{:032x}", 2)),
                        ..key()
                    },
                }
            }
            "read.account_context" => {
                if let ReadBody::Products {
                    delivery_context, ..
                } = &mut o.body
                {
                    delivery_context.account_ref = text("other-account");
                }
            }
            "read.product_scope" => {
                if let ActionScope::Shop { read_scope, .. } = &mut j.scope {
                    read_scope.products.0.push(other_product());
                }
                if let JobPayload::ProductObservation { read_scope, .. } = &mut j.payload {
                    read_scope.products.0.push(other_product());
                }
            }
            "read.event_scope" => {
                if let ReadBody::CalendarEvents { items } = &mut o.body {
                    items.0[0].event_id = CalendarEventId(format!("ce_{:032x}", 2));
                }
            }
            "read.time_before_scope" => o.observed_at = time("2026-09-27T09:59:59Z"),
            "read.time_after_scope" => o.observed_at = time("2026-09-27T10:10:01Z"),
            "read.future_clock_skew" => {
                o.observed_at = time("2026-09-27T10:08:00Z");
                admission = time("2026-09-27T10:01:00Z");
            }
            "read.kind_scope" => {
                o.body = ReadBody::Slots {
                    delivery_context: RetailerDeliveryContext {
                        retailer: Retailer::Tesco,
                        account_ref: text("account"),
                        context_ref: text("delivery"),
                        delivery_date: Nullable(None),
                        slot_ref: Nullable(None),
                    },
                    items: BoundedVec(vec![]),
                }
            }
            "read.payload_scope_divergence" => {
                if let JobPayload::ProductObservation { read_scope, .. } = &mut j.payload {
                    read_scope.purpose = ReadPurpose::Verification;
                }
            }
            _ => unreachable!(),
        }
        let actual = validate_read_binding(&o, &j, &reader(), admission)
            .unwrap_err()
            .code;
        assert_eq!(actual, *expected, "{name}");
        println!("PASS {name}");
    }
    println!("READ_MATRIX_CASES {}", CASES.len());
}

#[test]
fn named_recovery_matrix() {
    const CASES: &[(&str, ErrorCode)] = &[
        ("resume.untrusted_source", ErrorCode::WorkerNotStopped),
        ("resume.wrong_source_trust", ErrorCode::WorkerNotStopped),
        ("resume.no_stop", ErrorCode::WorkerNotStopped),
        ("resume.wrong_worker", ErrorCode::WorkerNotStopped),
        ("resume.wrong_generation", ErrorCode::WorkerNotStopped),
        ("resume.wrong_provider_build", ErrorCode::WorkerNotStopped),
        ("resume.qualification_mismatch", ErrorCode::WorkerNotStopped),
        (
            "resume.no_reconciliation",
            ErrorCode::ReconciliationRequired,
        ),
        (
            "resume.prestop_reconciliation",
            ErrorCode::ReconciliationRequired,
        ),
        (
            "resume.wrong_reconciliation_worker",
            ErrorCode::ReconciliationRequired,
        ),
        (
            "resume.unresolved_operation",
            ErrorCode::ReconciliationRequired,
        ),
        ("resume.manual_conflict", ErrorCode::ReconciliationRequired),
        (
            "resume.outstanding_operation",
            ErrorCode::ReconciliationRequired,
        ),
        ("resume.policy_disabled", ErrorCode::PrivacyDenied),
        ("resume.policy_revision", ErrorCode::ContextStale),
        ("resume.context_changed", ErrorCode::ContextStale),
        ("resume.approval_stale", ErrorCode::AuthorityDenied),
        ("resume.lock_active", ErrorCode::DomainConflict),
    ];
    for (name, expected_code) in CASES {
        let mut l = lock();
        let mut current = stamp();
        let expected = stamp();
        let mut p = policy();
        match *name {
            "resume.no_stop" => l.stop_evidence = Nullable(None),
            "resume.wrong_worker" => {
                l.stop_evidence.0.as_mut().unwrap().worker.native_task_ref = text("other-task")
            }
            "resume.wrong_generation" => {
                l.stop_evidence.0.as_mut().unwrap().worker.generation = Generation(2)
            }
            "resume.wrong_provider_build" => {
                l.stop_evidence.0.as_mut().unwrap().worker.provider_build = text("other-build")
            }
            "resume.prestop_reconciliation" => {
                l.last_reconciliation.0.as_mut().unwrap().observed_at = time("2026-09-27T10:00:01Z")
            }
            "resume.wrong_reconciliation_worker" => {
                l.last_reconciliation
                    .0
                    .as_mut()
                    .unwrap()
                    .old_worker
                    .native_task_ref = text("other-task")
            }
            "resume.unresolved_operation" => l
                .last_reconciliation
                .0
                .as_mut()
                .unwrap()
                .unresolved_operation_ids
                .0
                .push(OperationId(id("op_"))),
            "resume.manual_conflict" => {
                l.last_reconciliation.0.as_mut().unwrap().manual_conflicts = true
            }
            "resume.outstanding_operation" => {
                l.outstanding_operation = Nullable(Some(OperationId(id("op_"))))
            }
            "resume.policy_disabled" => p.use_ai_features = false,
            "resume.policy_revision" => p.revision = Revision(2),
            "resume.context_changed" => current.catalogue_revision = Revision(2),
            "resume.lock_active" => l.state = BasketLockState::Active,
            _ => {}
        }
        let mut facts = resume_facts(&l, &current, &p);
        facts.expected_context = &expected;
        match *name {
            "resume.untrusted_source" => facts.stop_source_trust = StopSourceTrust::Untrusted,
            "resume.wrong_source_trust" => facts.stop_source_trust = StopSourceTrust::LocalUser,
            "resume.qualification_mismatch" => facts.qualification_matches = false,
            "resume.no_reconciliation" => facts.reconciliation = None,
            "resume.approval_stale" => facts.approval_current = false,
            _ => {}
        }
        let actual = may_resume_basket(&facts).unwrap_err().code;
        assert_eq!(actual, *expected_code, "{name}");
        println!("PASS {name}");
    }
    println!("RECOVERY_MATRIX_CASES {}", CASES.len());
}

#[test]
fn named_basket_operation_matrix() {
    const CASES: &[(&str, bool, ErrorCode)] = &[
        (
            "operation.delta_target_mismatch",
            false,
            ErrorCode::InvalidValue,
        ),
        (
            "operation.negative_underflow",
            false,
            ErrorCode::InvalidValue,
        ),
        (
            "operation.expected_quantity_limit",
            false,
            ErrorCode::LimitExceeded,
        ),
        (
            "operation.target_quantity_limit",
            false,
            ErrorCode::LimitExceeded,
        ),
        ("operation.delta_limit", false, ErrorCode::LimitExceeded),
        (
            "operation.resource_mismatch",
            false,
            ErrorCode::ScopeViolation,
        ),
        ("approval.removal_missing", true, ErrorCode::AuthorityDenied),
        (
            "approval.attribution_missing",
            true,
            ErrorCode::AuthorityDenied,
        ),
        (
            "approval.product_mismatch",
            true,
            ErrorCode::AuthorityDenied,
        ),
        (
            "approval.expected_quantity_mismatch",
            true,
            ErrorCode::AuthorityDenied,
        ),
        (
            "approval.target_quantity_mismatch",
            true,
            ErrorCode::AuthorityDenied,
        ),
        (
            "approval.product_digest_mismatch",
            true,
            ErrorCode::DigestMismatch,
        ),
        (
            "approval.price_digest_mismatch",
            true,
            ErrorCode::DigestMismatch,
        ),
        ("approval.duplicate_lines", true, ErrorCode::InvalidValue),
        (
            "approval.line_quantity_limit",
            true,
            ErrorCode::LimitExceeded,
        ),
    ];
    for (name, check_approval, expected_code) in CASES {
        let mut action = basket_action(-1);
        let mut approved = approval(true, true);
        let mut resource = ExternalResource::Basket { key: key() };
        let ExternalAction::BasketSetQuantity {
            expected_observed_quantity,
            target_quantity,
            planner_delta,
            product_evidence_digest,
            price_approval_digest,
            ..
        } = &mut action
        else {
            unreachable!()
        };
        match *name {
            "operation.delta_target_mismatch" => *target_quantity = 2,
            "operation.negative_underflow" => {
                *expected_observed_quantity = 0;
                *target_quantity = 0;
            }
            "operation.expected_quantity_limit" => *expected_observed_quantity = 10_001,
            "operation.target_quantity_limit" => *target_quantity = 10_001,
            "operation.delta_limit" => *planner_delta = -10_001,
            "operation.resource_mismatch" => {
                resource = ExternalResource::Calendar {
                    destination_id: CalendarDestinationId(id("cd_")),
                }
            }
            "approval.removal_missing" => approved.lines.0[0].removal_approved = false,
            "approval.attribution_missing" => {
                approved.lines.0[0].planner_attribution_digest = Nullable(None)
            }
            "approval.product_mismatch" => approved.lines.0[0].product = other_product(),
            "approval.expected_quantity_mismatch" => approved.lines.0[0].expected_quantity = 5,
            "approval.target_quantity_mismatch" => approved.lines.0[0].target_quantity = 2,
            "approval.product_digest_mismatch" => *product_evidence_digest = Digest("b".repeat(64)),
            "approval.price_digest_mismatch" => *price_approval_digest = Digest("b".repeat(64)),
            "approval.duplicate_lines" => approved.lines.0.push(approved.lines.0[0].clone()),
            "approval.line_quantity_limit" => approved.lines.0[0].expected_quantity = 10_001,
            _ => unreachable!(),
        }
        let actual = if *check_approval {
            validate_operation_approval(&action, &approved)
        } else {
            validate_external_action(&action, &resource)
        }
        .unwrap_err()
        .code;
        assert_eq!(actual, *expected_code, "{name}");
        println!("PASS {name}");
    }
    println!("OPERATION_MATRIX_CASES {}", CASES.len());
}

#[test]
fn concrete_authority_fixtures_execute_predicates() {
    let base =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/contracts");
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(base.join("ruling005-current-manifest.json")).unwrap(),
    )
    .unwrap();
    let mut executed = 0;
    let mut failures = Vec::new();
    for case in manifest["cases"].as_array().unwrap() {
        let stage = case["stage"].as_str().unwrap();
        if ![
            "read_binding",
            "resume",
            "operation_shape",
            "operation_approval",
        ]
        .contains(&stage)
        {
            continue;
        }
        let id = case["id"].as_str().unwrap();
        let file = case["file"].as_str().unwrap();
        let fixture: Value =
            serde_json::from_slice(&std::fs::read(base.join(file)).unwrap()).unwrap();
        let result = match stage {
            "read_binding" => {
                let request: JobRequestV1 =
                    serde_json::from_value(fixture["request"].clone()).unwrap();
                let raw = serde_json::to_vec(&fixture["observation"]).unwrap();
                match parse_document::<ReadObservationV1>(&raw) {
                    Ok(observation) => {
                        let reader: ReadActor =
                            serde_json::from_value(fixture["expected_reader"].clone()).unwrap();
                        let admission: Instant =
                            serde_json::from_value(fixture["admission_time"].clone()).unwrap();
                        validate_read_binding(&observation, &request, &reader, admission)
                    }
                    Err(error) => Err(error),
                }
            }
            "resume" => {
                let lock: BasketLock = serde_json::from_value(fixture["lock"].clone()).unwrap();
                let reconciliation: Option<ReconciliationEvidence> =
                    serde_json::from_value(fixture["reconciliation"].clone()).unwrap();
                let current: ContextStamp =
                    serde_json::from_value(fixture["current_context"].clone()).unwrap();
                let expected: ContextStamp =
                    serde_json::from_value(fixture["expected_context"].clone()).unwrap();
                let policy: PrivacyPolicy =
                    serde_json::from_value(fixture["policy"].clone()).unwrap();
                let trust = match fixture["stop_source_trust"].as_str().unwrap() {
                    "untrusted" => StopSourceTrust::Untrusted,
                    "local_user" => StopSourceTrust::LocalUser,
                    "qualified_provider" => StopSourceTrust::QualifiedProvider,
                    other => panic!("{id}: unknown stop source trust {other}"),
                };
                may_resume_basket(&ResumeFacts {
                    lock: &lock,
                    stop_source_trust: trust,
                    qualification_matches: fixture["qualification_matches"].as_bool().unwrap(),
                    reconciliation: reconciliation.as_ref(),
                    current_context: &current,
                    expected_context: &expected,
                    policy: &policy,
                    approval_current: fixture["approval_current"].as_bool().unwrap(),
                })
            }
            "operation_shape" => {
                let request: OperationRequest =
                    serde_json::from_value(fixture["request"].clone()).unwrap();
                validate_operation_shape(&request)
            }
            "operation_approval" => {
                let request: OperationRequest =
                    serde_json::from_value(fixture["request"].clone()).unwrap();
                let approval: ApprovedContributionRef =
                    serde_json::from_value(fixture["approval"].clone()).unwrap();
                validate_operation_approval(&request.action, &approval)
            }
            _ => unreachable!(),
        };
        let actual = result
            .err()
            .map(|e| serde_json::to_value(e.code).unwrap())
            .unwrap_or(Value::String("ok".into()));
        if actual != case["expected"] {
            failures.push(format!("{id}: expected {}, got {actual}", case["expected"]));
        }
        executed += 1;
    }
    println!("AUTHORITY_FIXTURE_SUBCASES {executed}");
    assert!(executed >= 60, "authority corpus incomplete: {executed}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_command_variant_has_all_three_caller_rules() {
    let base =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/contracts");
    let load = |name: &str| -> Value {
        serde_json::from_slice(&std::fs::read(base.join(name)).unwrap()).unwrap()
    };
    let draft: HouseholdDraft =
        serde_json::from_value(load("ruling005-current--household-draft.json")).unwrap();
    let profile: HouseholdProfile = serde_json::from_value(load("household-profile.json")).unwrap();
    let week: Week = serde_json::from_value(load("week.json")).unwrap();
    let request: JobRequestV1 = serde_json::from_value(load(
        "ruling004-current--job-request-product-observation.json",
    ))
    .unwrap();
    let calendar_request: JobRequestV1 = serde_json::from_value(load(
        "ruling004-current--job-request-calendar-publication.json",
    ))
    .unwrap();
    let operation: OperationRequest =
        serde_json::from_value(load("ruling004-current--operation-request.json")).unwrap();
    let policy = policy();
    let calendar_event = match calendar_request.payload {
        JobPayload::CalendarPublication { batch, .. } => batch.events.0[0].clone(),
        _ => unreachable!(),
    };
    let commands = [
        AppCommand::SaveHouseholdDraft {
            restriction_issue_edits: BoundedVec(vec![]),
            draft,
        },
        AppCommand::UpdateProfile {
            profile,
            expected_profile_revision: Revision(1),
        },
        AppCommand::SetPrivacy {
            policy,
            expected_policy_revision: Revision(1),
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
            context_stamp: stamp(),
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
                confirmed_at: time("2026-09-27T10:00:00Z"),
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
                confirmed_at: time("2026-09-27T10:00:00Z"),
                source: OrderConfirmationSource::UserConfirmation {},
            },
        },
        AppCommand::RequestJob {
            action: request.action,
            scope: request.scope,
        },
        AppCommand::CancelJob {
            job_id: JobId(id("jb_")),
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
            event_batch_digest: digest(),
        },
    ];
    let mut seen = std::collections::BTreeSet::new();
    let mut cells = 0;
    for command in &commands {
        let kind = command.kind();
        assert!(seen.insert(format!("{kind:?}")), "duplicate command kind");
        for caller in [
            CallerClass::LocalUi,
            CallerClass::ExchangeImport,
            CallerClass::QualifiedAdapter,
        ] {
            let permitted = matches!(caller, CallerClass::LocalUi)
                || matches!(caller, CallerClass::QualifiedAdapter)
                    && kind == CommandKind::ConfirmOrder;
            let actual = authorize_command(kind, caller);
            if permitted {
                assert!(actual.is_ok(), "{kind:?}/{caller:?}: {actual:?}");
            } else {
                assert_eq!(
                    actual.unwrap_err().code,
                    ErrorCode::AuthorityDenied,
                    "{kind:?}/{caller:?}"
                );
            }
            cells += 1;
        }
    }
    assert_eq!(seen.len(), CommandKind::ALL.len());
    assert_eq!(cells, 36);
    println!("COMMAND_AUTHORITY_CELLS {cells}");
}

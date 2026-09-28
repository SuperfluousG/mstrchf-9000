//! Grant-independent external reads. Admission checks correlation, not truth.
use crate::*;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

string_enum! { pub enum ReadPurpose { Initial, Reconciliation, Verification, SlotDiscovery } }
string_enum! { pub enum ReadKind { Products, Basket, CalendarEvents, Slots } }
string_enum! { pub enum UnknownReason { Missing, Unreadable, NotExposed } }
string_enum! { pub enum Availability { InStock, OutOfStock, Unknown } }
string_enum! { pub enum OfferKind { UnitPrice, Multibuy } }
string_enum! { pub enum DeliveryApplicable { Yes, No, Unknown } }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub enum Currency {
    #[serde(rename = "GBP")]
    #[ts(rename = "GBP")]
    Gbp,
}

/// A JSON integer in 1..=10000, used for observed packs and offer thresholds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[serde(transparent)]
#[ts(type = "number")]
pub struct BoundedCount(pub u16);
impl<'de> Deserialize<'de> for BoundedCount {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = u16::deserialize(d)?;
        if !(1..=10_000).contains(&value) {
            return Err(serde::de::Error::custom("CODE:limit_exceeded"));
        }
        Ok(Self(value))
    }
}
impl JsonSchema for BoundedCount {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "BoundedCount".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({"type":"integer","minimum":1,"maximum":10000})
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Fact<T> {
    Known { value: T },
    Unknown { reason: UnknownReason },
    Conflicting { values: BoundedVec<T, 2, 8> },
}
tagged! { pub enum Price {
    Observed { pence: DecimalU64, currency: Currency },
    Estimated { pence: DecimalU64, currency: Currency, basis: Text },
    Unknown { reason: UnknownReason }
} }
record! { pub struct ReadScope {
    pub read_id: ReadId, pub resource: ExternalResource, pub purpose: ReadPurpose,
    pub allowed_kinds: BoundedVec<ReadKind, 1, 4>,
    pub products: BoundedVec<ProductRef, 0, 256>,
    pub event_ids: BoundedVec<CalendarEventId, 0, 256>,
    pub not_before: Instant, pub expires_at: Instant
} }
record! { pub struct ReadActor {
    pub provider: Provider, pub native_task_ref: Text
} }
record! { pub struct RetailerDeliveryContext {
    pub retailer: Retailer, pub account_ref: Text, pub context_ref: Text,
    pub delivery_date: Nullable<LocalDate>, pub slot_ref: Nullable<Text>
} }
record! { pub struct PackObservation {
    pub count: Fact<BoundedCount>, pub net: Fact<Quantity>,
    pub drained: Fact<Quantity>
} }
record! { pub struct OfferObservation {
    pub offer_ref: Text, pub kind: OfferKind, pub minimum_packs: BoundedCount,
    pub total_pence: DecimalU64, pub loyalty_required: bool,
    pub valid_from: LocalDate, pub valid_to: LocalDate,
    pub delivery_applicable: DeliveryApplicable
} }
record! { pub struct ProductObservation {
    pub product: ProductRef, pub url: HttpUrl, pub title: Text,
    pub pack: PackObservation, pub price: Price,
    pub offers: BoundedVec<OfferObservation, 0, 64>,
    pub availability: Availability, pub ingredients: Fact<Text>,
    pub allergens: Fact<Text>, pub storage: Fact<Text>,
    pub delivery_context: RetailerDeliveryContext, pub observed_at: Instant
} }
record! { pub struct BasketLine {
    pub product: ProductRef, pub quantity: u32, pub unit_price: Price
} }
record! { pub struct BasketSnapshot {
    pub key: BasketKey, pub delivery_context: RetailerDeliveryContext,
    pub observed_at: Instant, pub lines: BoundedVec<BasketLine, 0, 512>,
    pub delivery_charge: Price
} }
record! { pub struct SlotObservation {
    pub slot_ref: Text, pub start: ZonedMoment, pub end: ZonedMoment,
    pub available: bool, pub charge: Price,
    pub hold_expires_at: Nullable<Instant>,
    pub financial_commitment: FinancialCommitment
} }
tagged! { pub enum ReadBody {
    Products { delivery_context: RetailerDeliveryContext,
        items: BoundedVec<ProductObservation, 0, 256> },
    Basket { snapshot: BasketSnapshot },
    CalendarEvents { items: BoundedVec<CalendarObservation, 0, 256> },
    Slots { delivery_context: RetailerDeliveryContext,
        items: BoundedVec<SlotObservation, 0, 256> }
} }
record! { pub struct ReadObservationV1 {
    pub schema_version: SchemaVersion, pub observation_id: ReadObservationId,
    pub read_id: ReadId, pub job_id: JobId, pub input_digest: InputDigest,
    pub reader: ReadActor, pub resource: ExternalResource,
    pub observed_at: Instant, pub body: ReadBody
} }
record! { pub struct AcceptedReadRef {
    pub evidence_id: EvidenceId, pub job_id: JobId, pub read_id: ReadId,
    pub observation_id: ReadObservationId, pub digest: Digest,
    pub resource: ExternalResource, pub observed_at: Instant,
    pub admitted_at: Instant
} }

fn distinct<T: PartialEq>(items: &[T]) -> bool {
    items
        .iter()
        .enumerate()
        .all(|(i, item)| !items[..i].contains(item))
}
fn check_fact<T: PartialEq>(fact: &Fact<T>, path: &str) -> Result<(), DomainError> {
    if let Fact::Conflicting { values } = fact {
        if values.len() > 8 {
            return Err(DomainError::new(ErrorCode::LimitExceeded, path));
        }
        if values.len() < 2 || !distinct(values) {
            return Err(DomainError::new(ErrorCode::InvalidValue, path));
        }
    }
    Ok(())
}
fn check_count_fact(fact: &Fact<BoundedCount>, path: &str) -> Result<(), DomainError> {
    check_fact(fact, path)?;
    let valid = |count: &BoundedCount| (1..=10_000).contains(&count.0);
    let good = match fact {
        Fact::Known { value } => valid(value),
        Fact::Unknown { .. } => true,
        Fact::Conflicting { values } => values.iter().all(valid),
    };
    if good {
        Ok(())
    } else {
        Err(DomainError::new(ErrorCode::LimitExceeded, path))
    }
}
fn check_price(price: &Price, path: &str) -> Result<(), DomainError> {
    let pence = match price {
        Price::Observed { pence, .. } | Price::Estimated { pence, .. } => Some(pence.0),
        Price::Unknown { .. } => None,
    };
    if pence.is_some_and(|n| n > 1_000_000_000_000) {
        return Err(DomainError::new(ErrorCode::LimitExceeded, path));
    }
    Ok(())
}
fn check_delivery_context(
    ctx: &RetailerDeliveryContext,
    resource: &ExternalResource,
    path: &str,
) -> Result<(), DomainError> {
    if let ExternalResource::Basket { key } = resource
        && (ctx.retailer != key.retailer || ctx.account_ref != key.account_ref)
    {
        return Err(DomainError::new(ErrorCode::IdentityMismatch, path));
    }
    Ok(())
}
fn check_observed_at(time: &Instant, outer: &Instant, path: &str) -> Result<(), DomainError> {
    if time.timestamp_seconds() > outer.timestamp_seconds() {
        return Err(DomainError::new(ErrorCode::InvalidValue, path));
    }
    Ok(())
}
fn scope_for_job(request: &JobRequestV1) -> Result<&ReadScope, DomainError> {
    match &request.scope {
        ActionScope::Shop { read_scope, .. }
        | ActionScope::Calendar { read_scope, .. }
        | ActionScope::Booking { read_scope, .. } => Ok(read_scope),
        _ => Err(DomainError::new(ErrorCode::ScopeViolation, "/scope")),
    }
}

pub fn validate_read_scope(scope: &ReadScope) -> Result<(), DomainError> {
    if scope.allowed_kinds.len() > 4 || scope.products.len() > 256 || scope.event_ids.len() > 256 {
        return Err(DomainError::new(ErrorCode::LimitExceeded, "/read_scope"));
    }
    if scope.allowed_kinds.is_empty()
        || scope.allowed_kinds.len() > 4
        || !distinct(&scope.allowed_kinds)
        || scope.products.len() > 256
        || !distinct(&scope.products)
        || scope.event_ids.len() > 256
        || !distinct(&scope.event_ids)
    {
        return Err(DomainError::new(ErrorCode::InvalidValue, "/read_scope"));
    }
    if scope.not_before.timestamp_seconds() > scope.expires_at.timestamp_seconds() {
        return Err(DomainError::new(
            ErrorCode::InvalidValue,
            "/read_scope/expires_at",
        ));
    }
    if scope.allowed_kinds.iter().any(|kind| {
        !matches!(
            (&scope.resource, kind),
            (
                ExternalResource::Basket { .. },
                ReadKind::Products | ReadKind::Basket | ReadKind::Slots
            ) | (ExternalResource::Calendar { .. }, ReadKind::CalendarEvents)
        )
    }) || matches!(&scope.resource, ExternalResource::Basket { .. })
        && !scope.event_ids.is_empty()
        || matches!(&scope.resource, ExternalResource::Calendar { .. })
            && !scope.products.is_empty()
    {
        return Err(DomainError::new(ErrorCode::ScopeViolation, "/read_scope"));
    }
    Ok(())
}

/// Checks semantic constraints that remain possible to violate when an in-memory
/// record is constructed without passing through strict deserialisation.
pub fn validate_read_shape(observation: &ReadObservationV1) -> Result<(), DomainError> {
    match (&observation.resource, &observation.body) {
        (
            ExternalResource::Basket { .. },
            ReadBody::Products { .. } | ReadBody::Basket { .. } | ReadBody::Slots { .. },
        )
        | (ExternalResource::Calendar { .. }, ReadBody::CalendarEvents { .. }) => {}
        _ => return Err(DomainError::new(ErrorCode::ScopeViolation, "/body/kind")),
    }
    match &observation.body {
        ReadBody::Products {
            delivery_context,
            items,
        } => {
            if items.len() > 256 {
                return Err(DomainError::new(
                    ErrorCode::LimitExceeded,
                    "/body/data/items",
                ));
            }
            check_delivery_context(
                delivery_context,
                &observation.resource,
                "/body/data/delivery_context",
            )?;
            if items.len() > 256 || !distinct(&items.iter().map(|x| &x.product).collect::<Vec<_>>())
            {
                return Err(DomainError::new(
                    ErrorCode::InvalidValue,
                    "/body/data/items",
                ));
            }
            for item in items.iter() {
                if item.offers.len() > 64 {
                    return Err(DomainError::new(
                        ErrorCode::LimitExceeded,
                        "/body/data/items/offers",
                    ));
                }
                if item.delivery_context != *delivery_context
                    || item.offers.len() > 64
                    || !distinct(&item.offers.iter().map(|x| &x.offer_ref).collect::<Vec<_>>())
                {
                    return Err(DomainError::new(
                        ErrorCode::InvalidValue,
                        "/body/data/items",
                    ));
                }
                check_observed_at(
                    &item.observed_at,
                    &observation.observed_at,
                    "/body/data/items/observed_at",
                )?;
                check_price(&item.price, "/body/data/items/price")?;
                check_count_fact(&item.pack.count, "/body/data/items/pack/count")?;
                check_fact(&item.pack.net, "/body/data/items/pack/net")?;
                check_fact(&item.pack.drained, "/body/data/items/pack/drained")?;
                check_fact(&item.ingredients, "/body/data/items/ingredients")?;
                check_fact(&item.allergens, "/body/data/items/allergens")?;
                check_fact(&item.storage, "/body/data/items/storage")?;
                for offer in item.offers.iter() {
                    if !(1..=10_000).contains(&offer.minimum_packs.0)
                        || offer.total_pence.0 > 1_000_000_000_000
                        || offer.valid_from > offer.valid_to
                    {
                        return Err(DomainError::new(
                            ErrorCode::InvalidValue,
                            "/body/data/items/offers",
                        ));
                    }
                }
            }
        }
        ReadBody::Basket { snapshot } => {
            if snapshot.lines.len() > 512 {
                return Err(DomainError::new(
                    ErrorCode::LimitExceeded,
                    "/body/data/snapshot/lines",
                ));
            }
            if observation.resource
                != (ExternalResource::Basket {
                    key: snapshot.key.clone(),
                })
            {
                return Err(DomainError::new(
                    ErrorCode::IdentityMismatch,
                    "/body/data/snapshot/key",
                ));
            }
            check_delivery_context(
                &snapshot.delivery_context,
                &observation.resource,
                "/body/data/snapshot/delivery_context",
            )?;
            check_observed_at(
                &snapshot.observed_at,
                &observation.observed_at,
                "/body/data/snapshot/observed_at",
            )?;
            check_price(
                &snapshot.delivery_charge,
                "/body/data/snapshot/delivery_charge",
            )?;
            if snapshot.lines.len() > 512
                || !distinct(
                    &snapshot
                        .lines
                        .iter()
                        .map(|x| &x.product)
                        .collect::<Vec<_>>(),
                )
            {
                return Err(DomainError::new(
                    ErrorCode::InvalidValue,
                    "/body/data/snapshot/lines",
                ));
            }
            for line in snapshot.lines.iter() {
                if !(1..=10_000).contains(&line.quantity) {
                    return Err(DomainError::new(
                        ErrorCode::LimitExceeded,
                        "/body/data/snapshot/lines/quantity",
                    ));
                }
                check_price(&line.unit_price, "/body/data/snapshot/lines/unit_price")?;
            }
        }
        ReadBody::CalendarEvents { items } => {
            if items.len() > 256 {
                return Err(DomainError::new(
                    ErrorCode::LimitExceeded,
                    "/body/data/items",
                ));
            }
            if items.len() > 256
                || !distinct(&items.iter().map(|x| &x.event_id).collect::<Vec<_>>())
            {
                return Err(DomainError::new(
                    ErrorCode::InvalidValue,
                    "/body/data/items",
                ));
            }
            for item in items.iter() {
                check_observed_at(
                    &item.observed_at,
                    &observation.observed_at,
                    "/body/data/items/observed_at",
                )?;
            }
        }
        ReadBody::Slots {
            delivery_context,
            items,
        } => {
            if items.len() > 256 {
                return Err(DomainError::new(
                    ErrorCode::LimitExceeded,
                    "/body/data/items",
                ));
            }
            check_delivery_context(
                delivery_context,
                &observation.resource,
                "/body/data/delivery_context",
            )?;
            if items.len() > 256
                || !distinct(&items.iter().map(|x| &x.slot_ref).collect::<Vec<_>>())
            {
                return Err(DomainError::new(
                    ErrorCode::InvalidValue,
                    "/body/data/items",
                ));
            }
            for item in items.iter() {
                if item.end.instant.timestamp_seconds() <= item.start.instant.timestamp_seconds() {
                    return Err(DomainError::new(
                        ErrorCode::InvalidValue,
                        "/body/data/items/end",
                    ));
                }
                check_price(&item.charge, "/body/data/items/charge")?;
            }
        }
    }
    Ok(())
}

/// Correlate an untrusted external observation to an app-created read scope.
/// The caller supplies a locally correlated running task and local admission time.
pub fn validate_read_binding(
    observation: &ReadObservationV1,
    request: &JobRequestV1,
    expected_reader: &ReadActor,
    admission_time: Instant,
) -> Result<(), DomainError> {
    validate_read_shape(observation)?;
    let scope = scope_for_job(request)?;
    validate_read_scope(scope)?;
    if observation.job_id != request.job_id || observation.read_id != scope.read_id {
        return Err(DomainError::new(ErrorCode::IdentityMismatch, "/job_id"));
    }
    if observation.input_digest != request.input_digest {
        return Err(DomainError::new(ErrorCode::DigestMismatch, "/input_digest"));
    }
    if observation.reader != *expected_reader || observation.resource != scope.resource {
        return Err(DomainError::new(ErrorCode::IdentityMismatch, "/reader"));
    }
    let start = scope.not_before.timestamp_seconds();
    let end = scope.expires_at.timestamp_seconds();
    let observed = observation.observed_at.timestamp_seconds();
    let admitted = admission_time.timestamp_seconds();
    if start > end || observed < start || observed > end || observed > admitted.saturating_add(300)
    {
        return Err(DomainError::new(ErrorCode::InvalidValue, "/observed_at"));
    }
    let kind = match &observation.body {
        ReadBody::Products { .. } => ReadKind::Products,
        ReadBody::Basket { .. } => ReadKind::Basket,
        ReadBody::CalendarEvents { .. } => ReadKind::CalendarEvents,
        ReadBody::Slots { .. } => ReadKind::Slots,
    };
    if !scope.allowed_kinds.contains(&kind) {
        return Err(DomainError::new(ErrorCode::ScopeViolation, "/body/kind"));
    }
    let resource_matches = matches!(
        (&scope.resource, kind),
        (
            ExternalResource::Basket { .. },
            ReadKind::Products | ReadKind::Basket | ReadKind::Slots
        ) | (ExternalResource::Calendar { .. }, ReadKind::CalendarEvents)
    );
    if !resource_matches {
        return Err(DomainError::new(ErrorCode::ScopeViolation, "/body/kind"));
    }
    match (&request.scope, &request.payload) {
        (
            ActionScope::Shop {
                key, read_scope, ..
            },
            JobPayload::ProductObservation { read_scope: p, .. },
        ) if read_scope == p && scope.resource == ExternalResource::Basket { key: key.clone() } => {
        }
        (
            ActionScope::Calendar {
                destination_id,
                read_scope,
                ..
            },
            JobPayload::CalendarPublication { read_scope: p, .. },
        ) if read_scope == p
            && scope.resource
                == ExternalResource::Calendar {
                    destination_id: destination_id.clone(),
                } => {}
        (
            ActionScope::Booking { read_scope, .. },
            JobPayload::BookingCheck { read_scope: p, .. },
        ) if read_scope == p => {}
        _ => {
            return Err(DomainError::new(
                ErrorCode::ScopeViolation,
                "/payload/read_scope",
            ));
        }
    }
    match &observation.body {
        ReadBody::Products {
            delivery_context,
            items,
        } => {
            if items.len() > 256 || !distinct(&items.iter().map(|x| &x.product).collect::<Vec<_>>())
            {
                return Err(DomainError::new(
                    ErrorCode::InvalidValue,
                    "/body/data/items",
                ));
            }
            check_delivery_context(
                delivery_context,
                &scope.resource,
                "/body/data/delivery_context",
            )?;
            for item in items.iter() {
                let payload_allows = match &request.payload {
                    JobPayload::ProductObservation { products, .. } => {
                        products.is_empty() || products.contains(&item.product)
                    }
                    _ => false,
                };
                if item.delivery_context != *delivery_context
                    || (!scope.products.is_empty() && !scope.products.contains(&item.product))
                    || !payload_allows
                {
                    return Err(DomainError::new(
                        ErrorCode::ScopeViolation,
                        "/body/data/items/product",
                    ));
                }
                check_observed_at(
                    &item.observed_at,
                    &observation.observed_at,
                    "/body/data/items/observed_at",
                )?;
                if item.observed_at.timestamp_seconds() < start {
                    return Err(DomainError::new(
                        ErrorCode::InvalidValue,
                        "/body/data/items/observed_at",
                    ));
                }
                check_price(&item.price, "/body/data/items/price")?;
                check_count_fact(&item.pack.count, "/body/data/items/pack/count")?;
                check_fact(&item.pack.net, "/body/data/items/pack/net")?;
                check_fact(&item.pack.drained, "/body/data/items/pack/drained")?;
                check_fact(&item.ingredients, "/body/data/items/ingredients")?;
                check_fact(&item.allergens, "/body/data/items/allergens")?;
                check_fact(&item.storage, "/body/data/items/storage")?;
                if item.offers.len() > 64
                    || !distinct(&item.offers.iter().map(|x| &x.offer_ref).collect::<Vec<_>>())
                {
                    return Err(DomainError::new(
                        ErrorCode::InvalidValue,
                        "/body/data/items/offers",
                    ));
                }
                for offer in item.offers.iter() {
                    if offer.minimum_packs.0 == 0
                        || offer.minimum_packs.0 > 10_000
                        || offer.total_pence.0 > 1_000_000_000_000
                        || offer.valid_from > offer.valid_to
                    {
                        return Err(DomainError::new(
                            ErrorCode::InvalidValue,
                            "/body/data/items/offers",
                        ));
                    }
                }
            }
        }
        ReadBody::Basket { snapshot } => {
            if snapshot.lines.len() > 512
                || !distinct(
                    &snapshot
                        .lines
                        .iter()
                        .map(|x| &x.product)
                        .collect::<Vec<_>>(),
                )
            {
                return Err(DomainError::new(
                    ErrorCode::InvalidValue,
                    "/body/data/snapshot/lines",
                ));
            }
            if scope.resource
                != (ExternalResource::Basket {
                    key: snapshot.key.clone(),
                })
            {
                return Err(DomainError::new(
                    ErrorCode::IdentityMismatch,
                    "/body/data/snapshot/key",
                ));
            }
            check_delivery_context(
                &snapshot.delivery_context,
                &scope.resource,
                "/body/data/snapshot/delivery_context",
            )?;
            check_observed_at(
                &snapshot.observed_at,
                &observation.observed_at,
                "/body/data/snapshot/observed_at",
            )?;
            if snapshot.observed_at.timestamp_seconds() < start {
                return Err(DomainError::new(
                    ErrorCode::InvalidValue,
                    "/body/data/snapshot/observed_at",
                ));
            }
            check_price(
                &snapshot.delivery_charge,
                "/body/data/snapshot/delivery_charge",
            )?;
            for line in snapshot.lines.iter() {
                if line.quantity == 0
                    || line.quantity > 10_000
                    || (!scope.products.is_empty() && !scope.products.contains(&line.product))
                {
                    return Err(DomainError::new(
                        ErrorCode::ScopeViolation,
                        "/body/data/snapshot/lines",
                    ));
                }
                check_price(&line.unit_price, "/body/data/snapshot/lines/unit_price")?;
            }
        }
        ReadBody::CalendarEvents { items } => {
            if items.len() > 256
                || !distinct(&items.iter().map(|x| &x.event_id).collect::<Vec<_>>())
            {
                return Err(DomainError::new(
                    ErrorCode::InvalidValue,
                    "/body/data/items",
                ));
            }
            for item in items.iter() {
                if !scope.event_ids.is_empty() && !scope.event_ids.contains(&item.event_id) {
                    return Err(DomainError::new(
                        ErrorCode::ScopeViolation,
                        "/body/data/items/event_id",
                    ));
                }
                if let ActionScope::Calendar { event_ids, .. } = &request.scope
                    && !event_ids.contains(&item.event_id)
                {
                    return Err(DomainError::new(
                        ErrorCode::ScopeViolation,
                        "/body/data/items/event_id",
                    ));
                }
                check_observed_at(
                    &item.observed_at,
                    &observation.observed_at,
                    "/body/data/items/observed_at",
                )?;
                if item.observed_at.timestamp_seconds() < start {
                    return Err(DomainError::new(
                        ErrorCode::InvalidValue,
                        "/body/data/items/observed_at",
                    ));
                }
            }
        }
        ReadBody::Slots {
            delivery_context,
            items,
        } => {
            if items.len() > 256
                || !distinct(&items.iter().map(|x| &x.slot_ref).collect::<Vec<_>>())
            {
                return Err(DomainError::new(
                    ErrorCode::InvalidValue,
                    "/body/data/items",
                ));
            }
            check_delivery_context(
                delivery_context,
                &scope.resource,
                "/body/data/delivery_context",
            )?;
            for item in items.iter() {
                if item.end.instant.timestamp_seconds() <= item.start.instant.timestamp_seconds() {
                    return Err(DomainError::new(
                        ErrorCode::InvalidValue,
                        "/body/data/items/end",
                    ));
                }
                check_price(&item.charge, "/body/data/items/charge")?;
            }
        }
    }
    Ok(())
}

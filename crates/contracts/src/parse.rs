use crate::*;
use serde::{
    Deserializer, Serialize,
    de::{DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess, Visitor},
};
use serde_json::Value;
use std::collections::BTreeMap;
pub trait ValidateShape: Serialize + schemars::JsonSchema {
    fn validate_shape(&self) -> Result<(), Vec<ValidationIssue>>;
    fn validate_wire(_value: &Value) -> Result<(), Vec<ValidationIssue>> {
        Ok(())
    }
}
impl<T: Serialize + schemars::JsonSchema> ValidateShape for T {
    fn validate_shape(&self) -> Result<(), Vec<ValidationIssue>> {
        let v = serde_json::to_value(self).map_err(|_| vec![issue("", ErrorCode::InvalidValue)])?;
        Self::validate_wire(&v)
    }
    fn validate_wire(v: &Value) -> Result<(), Vec<ValidationIssue>> {
        let settings = schemars::generate::SchemaSettings::draft2020_12();
        let schema = settings
            .into_generator()
            .into_root_schema_for::<T>()
            .to_value();
        let mut errors = Vec::new();
        walk_schema(&schema, &schema, v, "", &T::schema_name(), &mut errors);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
fn issue(path: &str, code: ErrorCode) -> ValidationIssue {
    ValidationIssue {
        path: path.into(),
        code,
        entity_id: Nullable(None),
    }
}
fn pointer(base: &str, key: &str) -> String {
    format!("{base}/{}", key.replace('~', "~0").replace('/', "~1"))
}
pub(crate) fn shape_error(issues: Vec<ValidationIssue>) -> DomainError {
    DomainError {
        code: issues.first().map_or(ErrorCode::InvalidValue, |e| e.code),
        truncated: issues.len() > 100,
        issues: BoundedVec(issues.into_iter().take(100).collect()),
        retry: Retry::Never,
    }
}
pub fn parse_document<T: DeserializeOwned + ValidateShape>(bytes: &[u8]) -> Result<T, DomainError> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(DomainError::new(ErrorCode::LimitExceeded, ""));
    }
    let schema = crate::schema::schema_for::<T>();
    let bounds = CollectionBounds::from_schema(&schema);
    let value = parse_value_with_bounds(bytes, &bounds)?;
    T::validate_wire(&value).map_err(shape_error)?;
    let result: T = serde_json::from_value(value).map_err(|e| decode_error(&e.to_string()))?;
    Ok(result)
}
fn walk_schema(
    root: &Value,
    s: &Value,
    v: &Value,
    path: &str,
    name: &str,
    out: &mut Vec<ValidationIssue>,
) {
    if out.len() > 100 {
        return;
    }
    if let Some(r) = s.get("$ref").and_then(Value::as_str) {
        if let Some(child) = root.pointer(r.trim_start_matches('#')) {
            walk_schema(
                root,
                child,
                v,
                path,
                r.rsplit('/').next().unwrap_or(""),
                out,
            );
        } else {
            out.push(issue(path, ErrorCode::MissingReference));
        }
        return;
    }
    if let Some(branches) = s
        .get("anyOf")
        .or_else(|| s.get("oneOf"))
        .and_then(Value::as_array)
    {
        let mut best: Option<Vec<ValidationIssue>> = None;
        let kind = v.get("kind");
        let selected = branches
            .iter()
            .find(|b| kind.is_some() && b.pointer("/properties/kind/const") == kind);
        if let Some(b) = selected {
            walk_schema(root, b, v, path, "", out);
            if !name.is_empty()
                && let Err(e) = validate_named(name, v)
            {
                for mut i in e.issues.0 {
                    i.path = format!("{path}{}", i.path);
                    out.push(i);
                }
            }
            return;
        }
        for b in branches {
            let mut trial = Vec::new();
            walk_schema(root, b, v, path, "", &mut trial);
            if trial.is_empty() {
                if !name.is_empty()
                    && let Err(e) = validate_named(name, v)
                {
                    for mut i in e.issues.0 {
                        i.path = format!("{path}{}", i.path);
                        out.push(i);
                    }
                }
                return;
            }
            if best.as_ref().is_none_or(|p| trial.len() < p.len()) {
                best = Some(trial);
            }
        }
        out.extend(best.unwrap_or_else(|| vec![issue(path, ErrorCode::InvalidValue)]));
        return;
    }
    if let Some(branches) = s.get("allOf").and_then(Value::as_array) {
        for b in branches {
            walk_schema(root, b, v, path, name, out);
        }
    }
    if let Some(c) = s.get("const")
        && c != v
    {
        out.push(issue(
            path,
            if path.ends_with("/schema_version") && v.is_number() {
                ErrorCode::UnsupportedVersion
            } else {
                ErrorCode::InvalidValue
            },
        ));
        return;
    }
    if let Some(e) = s.get("enum").and_then(Value::as_array)
        && !e.contains(v)
    {
        out.push(issue(path, ErrorCode::InvalidValue));
        return;
    }
    let matches_type = |t: &str| match t {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "boolean" => v.is_boolean(),
        "integer" => v.as_i64().is_some() || v.as_u64().is_some(),
        "number" => v.is_number(),
        "null" => v.is_null(),
        _ => false,
    };
    if let Some(t) = s.get("type") {
        let valid = if let Some(t) = t.as_str() {
            matches_type(t)
        } else {
            t.as_array()
                .is_some_and(|a| a.iter().any(|x| x.as_str().is_some_and(matches_type)))
        };
        if !valid {
            out.push(issue(path, ErrorCode::InvalidValue));
            return;
        }
    }
    let before = out.len();
    if let Some(obj) = v.as_object() {
        let props = s.get("properties").and_then(Value::as_object);
        if let Some(required) = s.get("required").and_then(Value::as_array) {
            for k in required.iter().filter_map(Value::as_str) {
                if !obj.contains_key(k) {
                    out.push(issue(&pointer(path, k), ErrorCode::MissingField));
                }
            }
        }
        for (k, x) in obj {
            if let Some(p) = props.and_then(|p| p.get(k)) {
                walk_schema(root, p, x, &pointer(path, k), "", out);
            } else if s.get("additionalProperties") == Some(&Value::Bool(false)) {
                out.push(issue(&pointer(path, k), ErrorCode::UnknownField));
            }
        }
    }
    if let Some(a) = v.as_array() {
        if s.get("maxItems")
            .and_then(Value::as_u64)
            .is_some_and(|n| a.len() > n as usize)
        {
            out.push(issue(path, ErrorCode::LimitExceeded));
            return;
        }
        if s.get("minItems")
            .and_then(Value::as_u64)
            .is_some_and(|n| a.len() < n as usize)
        {
            out.push(issue(path, ErrorCode::InvalidValue));
            return;
        }
        if let Some(item) = s.get("items") {
            for (i, x) in a.iter().enumerate() {
                walk_schema(root, item, x, &pointer(path, &i.to_string()), "", out);
            }
        }
        if let Some(items) = s.get("prefixItems").and_then(Value::as_array) {
            for (i, (p, x)) in items.iter().zip(a).enumerate() {
                walk_schema(root, p, x, &pointer(path, &i.to_string()), "", out);
            }
        }
    }
    if let Some(n) = v.as_i64()
        && (s
            .get("minimum")
            .and_then(Value::as_i64)
            .is_some_and(|m| n < m)
            || s.get("maximum")
                .and_then(Value::as_i64)
                .is_some_and(|m| n > m))
    {
        out.push(issue(path, ErrorCode::LimitExceeded));
    }
    if let Some(t) = v.as_str() {
        if s.get("maxLength")
            .and_then(Value::as_u64)
            .is_some_and(|n| t.chars().count() > n as usize)
        {
            out.push(issue(path, ErrorCode::LimitExceeded));
        }
        if s.get("minLength")
            .and_then(Value::as_u64)
            .is_some_and(|n| t.chars().count() < n as usize)
        {
            out.push(issue(path, ErrorCode::InvalidValue));
        }
    }
    if out.len() == before
        && let Err(e) = validate_named(name, v)
    {
        for mut i in e.issues.0 {
            i.path = format!("{path}{}", i.path);
            out.push(i);
        }
    }
}
fn validate_named(name: &str, v: &Value) -> Result<(), DomainError> {
    macro_rules! primitive {($($t:ty),*$(,)?)=>{$(if name==stringify!($t){return serde_json::from_value::<$t>(v.clone()).map(|_|()).map_err(|e|decode_error(&e.to_string()));})*};}
    primitive!(
        HouseholdId,
        DinerId,
        GuestId,
        OccasionId,
        WeekId,
        RecipeId,
        IngredientId,
        ExtraId,
        BatchId,
        ComponentId,
        AllocationId,
        PrepSessionId,
        DemandLineId,
        EvidenceId,
        AssetId,
        JobId,
        ResultId,
        OperationId,
        ArtefactId,
        BasketId,
        OrderId,
        CalendarEventId,
        CalendarDestinationId,
        ApprovalId,
        CommandId,
        ReadId,
        ReadObservationId,
        Digest,
        InputDigest,
        RecipeDigest,
        ResultDigest,
        PayloadDigest,
        AssetDigest,
        PlanInputDigest,
        DemandDigest,
        GrantToken,
        Text,
        Title,
        NameText,
        HttpUrl,
        Revision,
        Generation,
        DecimalU64,
        Rational,
        PositiveRational,
        LocalDate,
        LocalTime,
        ZoneId,
        Instant,
        ZonedMoment,
        BoundedCount
    );
    if name == "LongText" {
        let s = v
            .as_str()
            .ok_or_else(|| DomainError::new(ErrorCode::InvalidValue, ""))?;
        if s.len() > 65536 {
            return Err(DomainError::new(ErrorCode::LimitExceeded, ""));
        }
        serde_json::from_value::<LongText>(v.clone()).map_err(|e| decode_error(&e.to_string()))?;
    }
    crate::context::validate_local(name, v)
}
pub(crate) fn decode_error(message: &str) -> DomainError {
    let code = if message.contains("CODE:duplicate_key") {
        ErrorCode::DuplicateKey
    } else if message.contains("CODE:limit_exceeded") {
        ErrorCode::LimitExceeded
    } else if message.contains("CODE:noncanonical_value") {
        ErrorCode::NoncanonicalValue
    } else if message.contains("CODE:unsupported_version") {
        ErrorCode::UnsupportedVersion
    } else if message.contains("missing field") {
        ErrorCode::MissingField
    } else if message.contains("unknown field") {
        ErrorCode::UnknownField
    } else {
        ErrorCode::InvalidValue
    };
    let path = message
        .split("CODE:limit_exceeded@")
        .nth(1)
        .map(|s| s.split(" at line ").next().unwrap_or(s).to_owned())
        .or_else(|| message.split('`').nth(1).map(|s| format!("/{s}")))
        .unwrap_or_default();
    DomainError::new(code, &path)
}
fn parse_value_with_bounds(bytes: &[u8], bounds: &CollectionBounds) -> Result<Value, DomainError> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(DomainError::new(ErrorCode::LimitExceeded, ""));
    }
    let mut d = serde_json::Deserializer::from_slice(bytes);
    let v = BoundedValue {
        depth: 0,
        bounds,
        path: String::new(),
    }
    .deserialize(&mut d)
    .map_err(|e| {
        let s = e.to_string();
        if s.contains("CODE:") {
            decode_error(&s)
        } else {
            DomainError::new(ErrorCode::MalformedJson, "")
        }
    })?;
    d.end()
        .map_err(|_| DomainError::new(ErrorCode::MalformedJson, ""))?;
    Ok(v)
}

/// A compact collection-bound view of the generated wire schema. It is built
/// before consuming untrusted JSON, so object field order does not determine
/// when a nested array limit becomes available. All branches of a tagged union
/// contribute their data fields, including when `data` precedes `kind`.
#[derive(Default)]
struct CollectionBounds {
    max_items: Option<usize>,
    conflicting_max: bool,
    properties: BTreeMap<String, CollectionBounds>,
    items: Option<Box<CollectionBounds>>,
    prefix_items: Vec<CollectionBounds>,
}
impl CollectionBounds {
    fn from_schema(schema: &Value) -> Self {
        let mut result = Self::default();
        result.merge_schema(schema, schema, &mut Vec::new());
        result
    }
    fn merge_schema(&mut self, root: &Value, schema: &Value, refs: &mut Vec<String>) {
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str)
            && !refs.iter().any(|seen| seen == reference)
            && let Some(target) = root.pointer(reference.trim_start_matches('#'))
        {
            refs.push(reference.into());
            self.merge_schema(root, target, refs);
            refs.pop();
        }
        if let Some(maximum) = schema.get("maxItems").and_then(Value::as_u64) {
            let maximum = usize::try_from(maximum).unwrap_or(usize::MAX);
            if self.max_items.is_some_and(|old| old != maximum) {
                self.conflicting_max = true;
            }
            self.max_items = Some(self.max_items.map_or(maximum, |old| old.max(maximum)));
        }
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            for (name, child) in properties {
                self.properties
                    .entry(name.clone())
                    .or_default()
                    .merge_schema(root, child, refs);
            }
        }
        if let Some(item) = schema.get("items") {
            self.items
                .get_or_insert_with(Default::default)
                .merge_schema(root, item, refs);
        }
        if let Some(items) = schema.get("prefixItems").and_then(Value::as_array) {
            if self.prefix_items.len() < items.len() {
                self.prefix_items.resize_with(items.len(), Self::default);
            }
            for (bound, item) in self.prefix_items.iter_mut().zip(items) {
                bound.merge_schema(root, item, refs);
            }
        }
        for keyword in ["allOf", "anyOf", "oneOf"] {
            if let Some(branches) = schema.get(keyword).and_then(Value::as_array) {
                for branch in branches {
                    self.merge_schema(root, branch, refs);
                }
            }
        }
    }
    fn item(&self, index: usize) -> &Self {
        self.prefix_items
            .get(index)
            .or(self.items.as_deref())
            .unwrap_or(&EMPTY_BOUNDS)
    }
    #[cfg(test)]
    fn collect_conflicts(&self, path: &str, out: &mut Vec<String>) {
        if self.conflicting_max {
            out.push(path.into());
        }
        for (key, child) in &self.properties {
            child.collect_conflicts(&pointer(path, key), out);
        }
        if let Some(child) = &self.items {
            child.collect_conflicts(&pointer(path, "*"), out);
        }
        for (index, child) in self.prefix_items.iter().enumerate() {
            child.collect_conflicts(&pointer(path, &index.to_string()), out);
        }
    }
}
static EMPTY_BOUNDS: CollectionBounds = CollectionBounds {
    max_items: None,
    conflicting_max: false,
    properties: BTreeMap::new(),
    items: None,
    prefix_items: Vec::new(),
};

struct BoundedValue<'a> {
    depth: usize,
    bounds: &'a CollectionBounds,
    path: String,
}
impl<'de> DeserializeSeed<'de> for BoundedValue<'_> {
    type Value = Value;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        if self.depth > MAX_DEPTH {
            return Err(serde::de::Error::custom("CODE:limit_exceeded"));
        }
        d.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for BoundedValue<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "bounded JSON")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Value, M::Error> {
        let mut out = serde_json::Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if out.contains_key(&key) {
                return Err(serde::de::Error::custom("CODE:duplicate_key"));
            }
            if out.len() >= 256 {
                return Err(serde::de::Error::custom("CODE:limit_exceeded"));
            }
            let value = map.next_value_seed(BoundedValue {
                depth: self.depth + 1,
                bounds: self.bounds.properties.get(&key).unwrap_or(&EMPTY_BOUNDS),
                path: pointer(&self.path, &key),
            })?;
            out.insert(key, value);
        }
        Ok(Value::Object(out))
    }
    fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<Value, S::Error> {
        let mut out = Vec::new();
        let maximum = self.bounds.max_items.unwrap_or(4096).min(4096);
        loop {
            if out.len() == maximum {
                // SeqAccess returns None at `]` without invoking this seed.
                // If another element exists, the seed refuses it before its
                // body is decoded or allocated.
                seq.next_element_seed(RejectElement { path: self.path })?;
                break;
            }
            let Some(v) = seq.next_element_seed(BoundedValue {
                depth: self.depth + 1,
                bounds: self.bounds.item(out.len()),
                path: pointer(&self.path, &out.len().to_string()),
            })?
            else {
                break;
            };
            out.push(v);
        }
        Ok(Value::Array(out))
    }
    fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Value, E> {
        Ok(Value::String(s.into()))
    }
    fn visit_string<E: serde::de::Error>(self, s: String) -> Result<Value, E> {
        Ok(Value::String(s))
    }
    fn visit_bool<E: serde::de::Error>(self, b: bool) -> Result<Value, E> {
        Ok(Value::Bool(b))
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_i64<E: serde::de::Error>(self, n: i64) -> Result<Value, E> {
        Ok(Value::Number(n.into()))
    }
    fn visit_u64<E: serde::de::Error>(self, n: u64) -> Result<Value, E> {
        Ok(Value::Number(n.into()))
    }
    fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Value, E> {
        Err(E::custom("CODE:invalid_value"))
    }
}
struct RejectElement {
    path: String,
}
impl<'de> DeserializeSeed<'de> for RejectElement {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, _deserializer: D) -> Result<Self::Value, D::Error> {
        Err(serde::de::Error::custom(format!(
            "CODE:limit_exceeded@{}",
            self.path
        )))
    }
}

#[cfg(test)]
mod collection_bounds_tests {
    use super::*;

    #[test]
    fn registered_roots_have_no_conflicting_union_array_maxima() {
        let generated = crate::schema::render_contracts().unwrap();
        for root in crate::schema::ROOT_NAMES {
            let bytes = &generated[&format!("contracts/v1/{root}.schema.json")];
            let schema: Value = serde_json::from_slice(bytes).unwrap();
            let bounds = CollectionBounds::from_schema(&schema);
            let mut conflicts = Vec::new();
            bounds.collect_conflicts("", &mut conflicts);
            assert!(conflicts.is_empty(), "{root}: {conflicts:?}");
        }
    }
}

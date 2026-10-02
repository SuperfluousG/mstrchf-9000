//! Frozen bounded parser and schema walk, plus closed semantic predicates.
use super::{Code, Error, ReceiptBinding};
use serde::{
    Deserializer, Serialize,
    de::{DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess, Visitor},
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
const MAX_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;
const MAX_DEPTH: usize = 32;
struct ValidationIssue {
    path: String,
    code: Code,
}
fn issue(path: &str, code: Code) -> ValidationIssue {
    ValidationIssue {
        path: path.into(),
        code,
    }
}
fn pointer(base: &str, key: &str) -> String {
    format!("{base}/{}", key.replace('~', "~0").replace('/', "~1"))
}

pub(super) fn decode<T: DeserializeOwned + Serialize>(
    bytes: &[u8],
    schema_text: &str,
    name: &str,
) -> Result<T, Error> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(Error::new(Code::LimitExceeded, ""));
    }
    let mut schema: Value = serde_json::from_str(schema_text).expect("frozen embedded schema");
    schema["$ref"] = Value::String(format!("#/$defs/{name}"));
    let bounds = CollectionBounds::from_schema(&schema);
    let value = parse_value_with_bounds(bytes, &bounds)?;
    // Classify the explicitly closed content before inspecting excluded graphs.
    if name == "Snapshot"
        && ["weeks", "conflicts"]
            .iter()
            .any(|k| value[*k].as_array().is_some_and(|a| !a.is_empty()))
    {
        return Err(Error::new(Code::UnknownClosure, ""));
    }
    if name == "AppCommandV1"
        && value
            .pointer("/command/kind")
            .and_then(Value::as_str)
            .is_some_and(|k| !matches!(k, "save_household_draft" | "set_privacy" | "cancel_job"))
    {
        return Err(Error::new(Code::UnknownClosure, "/command"));
    }
    let mut errors = Vec::new();
    walk_schema(&schema, &schema, &value, "", "", &mut errors);
    if let Some(e) = errors.into_iter().next() {
        return Err(Error::new(e.code, &e.path));
    }
    let typed: T = serde_json::from_value(value).map_err(|e| Error::from_decode(&e.to_string()))?;
    if canonical(&typed)? != bytes {
        return Err(Error::new(Code::NoncanonicalValue, ""));
    }
    Ok(typed)
}

// JCS equivalence is restricted to this typed graph: ASCII fixed keys, no
// floats, numeric schema version/u16 only; all large numbers are strings.
// Explicit UTF-16 key sorting avoids serde_json map-feature dependence.
pub(super) fn canonical<T: Serialize>(typed: &T) -> Result<Vec<u8>, Error> {
    fn emit(v: &Value, out: &mut Vec<u8>) -> Result<(), Error> {
        match v {
            Value::Object(m) => {
                out.push(b'{');
                let mut keys: Vec<_> = m.keys().collect();
                keys.sort_by_key(|k| k.encode_utf16().collect::<Vec<_>>());
                for (i, k) in keys.into_iter().enumerate() {
                    if i > 0 {
                        out.push(b',');
                    }
                    out.extend(
                        serde_json::to_vec(k).map_err(|_| Error::new(Code::InvalidValue, ""))?,
                    );
                    out.push(b':');
                    emit(&m[k], out)?;
                }
                out.push(b'}');
            }
            Value::Array(a) => {
                out.push(b'[');
                for (i, x) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(b',');
                    }
                    emit(x, out)?;
                }
                out.push(b']');
            }
            _ => out.extend(serde_json::to_vec(v).map_err(|_| Error::new(Code::InvalidValue, ""))?),
        }
        Ok(())
    }
    let v = serde_json::to_value(typed).map_err(|_| Error::new(Code::InvalidValue, ""))?;
    let mut out = Vec::new();
    emit(&v, &mut out)?;
    Ok(out)
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
            out.push(issue(path, Code::MissingReference));
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
                for mut i in [ValidationIssue {
                    path: e.path,
                    code: e.code,
                }] {
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
                    for mut i in [ValidationIssue {
                        path: e.path,
                        code: e.code,
                    }] {
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
        out.extend(best.unwrap_or_else(|| vec![issue(path, Code::InvalidValue)]));
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
                Code::UnsupportedVersion
            } else {
                Code::InvalidValue
            },
        ));
        return;
    }
    if let Some(e) = s.get("enum").and_then(Value::as_array)
        && !e.contains(v)
    {
        out.push(issue(path, Code::InvalidValue));
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
            out.push(issue(path, Code::InvalidValue));
            return;
        }
    }
    let before = out.len();
    if let Some(obj) = v.as_object() {
        let props = s.get("properties").and_then(Value::as_object);
        if let Some(required) = s.get("required").and_then(Value::as_array) {
            for k in required.iter().filter_map(Value::as_str) {
                if !obj.contains_key(k) {
                    out.push(issue(&pointer(path, k), Code::MissingField));
                }
            }
        }
        for (k, x) in obj {
            if let Some(p) = props.and_then(|p| p.get(k)) {
                walk_schema(root, p, x, &pointer(path, k), "", out);
            } else if s.get("additionalProperties") == Some(&Value::Bool(false)) {
                // Unknown keys are untrusted stored text, not safe diagnostics.
                out.push(issue(path, Code::UnknownField));
            }
        }
    }
    if let Some(a) = v.as_array() {
        if s.get("maxItems")
            .and_then(Value::as_u64)
            .is_some_and(|n| a.len() > n as usize)
        {
            out.push(issue(path, Code::LimitExceeded));
            return;
        }
        if s.get("minItems")
            .and_then(Value::as_u64)
            .is_some_and(|n| a.len() < n as usize)
        {
            out.push(issue(path, Code::InvalidValue));
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
        out.push(issue(path, Code::LimitExceeded));
    }
    if let Some(t) = v.as_str() {
        if s.get("maxLength")
            .and_then(Value::as_u64)
            .is_some_and(|n| t.chars().count() > n as usize)
        {
            out.push(issue(path, Code::LimitExceeded));
        }
        if s.get("minLength")
            .and_then(Value::as_u64)
            .is_some_and(|n| t.chars().count() < n as usize)
        {
            out.push(issue(path, Code::InvalidValue));
        }
    }
    if out.len() == before
        && let Err(e) = validate_named(name, v)
    {
        for mut i in [ValidationIssue {
            path: e.path,
            code: e.code,
        }] {
            i.path = format!("{path}{}", i.path);
            out.push(i);
        }
    }
}
fn parse_value_with_bounds(bytes: &[u8], bounds: &CollectionBounds) -> Result<Value, Error> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(Error::new(Code::LimitExceeded, ""));
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
            Error::from_decode(&s)
        } else {
            Error::new(Code::MalformedJson, "")
        }
    })?;
    d.end().map_err(|_| Error::new(Code::MalformedJson, ""))?;
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
        // These are the only zero-bound arrays in the frozen closed graphs.
        // Refuse the unsupported content without reading its element body.
        if matches!(self.path.as_str(), "/weeks" | "/conflicts") {
            return Err(serde::de::Error::custom("CODE:unknown_closure"));
        }
        Err(serde::de::Error::custom(format!(
            "CODE:limit_exceeded@{}",
            self.path
        )))
    }
}

fn require(ok: bool, code: Code, path: &str) -> Result<(), Error> {
    if ok {
        Ok(())
    } else {
        Err(Error::new(code, path))
    }
}
fn positive(s: &str) -> Result<u64, Error> {
    require(
        !s.is_empty() && !s.starts_with('0') && s.bytes().all(|b| b.is_ascii_digit()),
        Code::NoncanonicalValue,
        "",
    )?;
    s.parse().map_err(|_| Error::new(Code::LimitExceeded, ""))
}
fn valid_id(s: &str, p: &str) -> bool {
    s.strip_prefix(p).is_some_and(|h| {
        h.len() == 32
            && h.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && h.bytes().any(|b| b != b'0')
    })
}
fn valid_digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn valid_time(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 8
        && b[2] == b':'
        && b[5] == b':'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 2 || i == 5 || c.is_ascii_digit())
        && &s[..2] <= "23"
        && &s[3..5] <= "59"
        && &s[6..] <= "59"
}
fn valid_instant(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 20
        || !s.is_ascii()
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[19] != b'Z'
        || !valid_time(&s[11..19])
    {
        return false;
    }
    if !b[..10]
        .iter()
        .enumerate()
        .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return false;
    }
    let y: u32 = s[..4].parse().unwrap();
    let m: usize = s[5..7].parse().unwrap();
    let d: u32 = s[8..10].parse().unwrap();
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    (1..=12).contains(&m) && d >= 1 && d <= days[m - 1]
}
fn key(v: &Value) -> String {
    String::from_utf8(canonical(v).expect("bounded JSON")).expect("UTF-8")
}
fn identity(field: &str, x: &Value) -> String {
    if field == "restriction_issue_edits" {
        return key(&x["data"]["diner_id"]);
    }
    let fields: &[&str] = match field {
        "diners" | "unresolved_restrictions" => &["diner_id"],
        "cells" => &["weekday", "meal_type"],
        "meal_times" | "meal_overrides" => &["meal_type"],
        "delivery_preferences" => &["weekday"],
        "jobs" => &["job_id"],
        _ => &[],
    };
    if fields.is_empty() {
        key(x)
    } else {
        key(&Value::Array(
            fields.iter().map(|f| x[*f].clone()).collect(),
        ))
    }
}
fn member_check(h: &Value) -> Result<(), Error> {
    if let Some(cells) = h.pointer("/usual_week/cells").and_then(Value::as_array) {
        let diners = h["diners"]
            .as_array()
            .ok_or_else(|| Error::new(Code::InvalidValue, "/diners"))?;
        for cell in cells {
            for id in cell["attendee_ids"]
                .as_array()
                .ok_or_else(|| Error::new(Code::InvalidValue, "/attendee_ids"))?
            {
                require(
                    diners.iter().any(|d| d["diner_id"] == *id),
                    Code::ForeignReference,
                    "/usual_week/cells/attendee_ids",
                )?;
            }
        }
    }
    Ok(())
}
fn validate_named(name: &str, v: &Value) -> Result<(), Error> {
    let str_value = || v.as_str().ok_or_else(|| Error::new(Code::InvalidValue, ""));
    let prefix = match name {
        "HouseholdId" => Some("hh_"),
        "DinerId" => Some("dn_"),
        "IngredientId" => Some("ig_"),
        "CalendarDestinationId" => Some("cd_"),
        "JobId" => Some("jb_"),
        "CommandId" => Some("cm_"),
        _ => None,
    };
    if let Some(p) = prefix {
        return require(valid_id(str_value()?, p), Code::InvalidValue, "");
    }
    match name {
        "Revision" => {
            positive(str_value()?)?;
            return Ok(());
        }
        "NameText" | "Text" => {
            let s = str_value()?;
            let (max, multi) = if name == "Text" {
                (4000, true)
            } else {
                (80, false)
            };
            return require(
                !s.is_empty()
                    && s.trim() == s
                    && s.chars().count() <= max
                    && !s
                        .chars()
                        .any(|c| c.is_control() && !(multi && (c == '\n' || c == '\t'))),
                Code::InvalidValue,
                "",
            );
        }
        "Instant" => return require(valid_instant(str_value()?), Code::InvalidValue, ""),
        "LocalTime" => return require(valid_time(str_value()?), Code::InvalidValue, ""),
        "ZoneId" => {
            return require(
                include_str!("zones.txt")
                    .lines()
                    .any(|z| z == str_value().unwrap_or("")),
                Code::InvalidValue,
                "",
            );
        }
        "PositiveRational" => {
            let n = positive(
                v["numerator"]
                    .as_str()
                    .ok_or_else(|| Error::new(Code::InvalidValue, "/numerator"))?,
            )?;
            let d = positive(
                v["denominator"]
                    .as_str()
                    .ok_or_else(|| Error::new(Code::InvalidValue, "/denominator"))?,
            )?;
            let (mut a, mut b) = (n, d);
            while b != 0 {
                (a, b) = (b, a % b);
            }
            require(a == 1, Code::NoncanonicalValue, "")?;
            return require(
                n <= 1_000_000_000_000 && d <= 1_000_000,
                Code::LimitExceeded,
                "",
            );
        }
        _ => {}
    }
    // Applies to unnamed tagged data as well as named records, exactly as the
    // source validator did. Structural validation runs before these checks.
    if let Some(obj) = v.as_object() {
        for (field, val) in obj {
            if let Some(a) = val.as_array() {
                if field != "day_enabled" {
                    let mut seen = BTreeSet::new();
                    require(
                        a.iter().all(|x| seen.insert(identity(field, x))),
                        Code::InvalidValue,
                        &format!("/{field}"),
                    )?;
                }
                if matches!(
                    field.as_str(),
                    "unresolved_restrictions" | "restriction_issue_edits"
                ) {
                    require(
                        a.windows(2)
                            .all(|w| identity(field, &w[0]) < identity(field, &w[1])),
                        Code::NoncanonicalValue,
                        &format!("/{field}"),
                    )?;
                }
                if matches!(
                    field.as_str(),
                    "attendee_ids"
                        | "dislikes"
                        | "equipment"
                        | "enabled_meal_types"
                        | "completed_stages"
                        | "channels"
                        | "prompts"
                ) {
                    require(
                        a.windows(2).all(|w| key(&w[0]) < key(&w[1])),
                        Code::NoncanonicalValue,
                        &format!("/{field}"),
                    )?;
                }
            }
        }
    }
    match name {
        "Snapshot" => {
            if let Some(c) = v
                .pointer("/household_draft/connections")
                .filter(|c| !c.is_null())
            {
                require(
                    c["privacy"] == v["privacy"],
                    Code::ContextStale,
                    "/household_draft/connections/privacy",
                )?;
            }
            if !v["profile"].is_null() {
                require(
                    v["profile"]["connections"]["privacy"] == v["privacy"],
                    Code::ContextStale,
                    "/profile/connections/privacy",
                )?;
                require(
                    v["profile"]["household_id"] == v["household_draft"]["household_id"],
                    Code::IdentityMismatch,
                    "/profile/household_id",
                )?;
            }
        }
        "HouseholdDraft" => {
            let diners = v["diners"]
                .as_array()
                .ok_or_else(|| Error::new(Code::InvalidValue, "/diners"))?;
            // Schema has already required the issue array in 005 and prohibited
            // it in retained-v1. This is no inference of operational format.
            let mut no_issues = true;
            if let Some(issues) = v.get("unresolved_restrictions").and_then(Value::as_array) {
                no_issues = issues.is_empty();
                for issue in issues {
                    let owner = diners
                        .iter()
                        .find(|d| d["diner_id"] == issue["diner_id"])
                        .ok_or_else(|| {
                            Error::new(Code::ForeignReference, "/unresolved_restrictions")
                        })?;
                    require(
                        owner["restrictions"]["kind"] == "unconfirmed",
                        Code::InvalidValue,
                        "/unresolved_restrictions",
                    )?;
                }
            }
            let confirmed = no_issues
                && !diners.is_empty()
                && diners
                    .iter()
                    .all(|d| d["restrictions"]["kind"] != "unconfirmed");
            let stages = v["completed_stages"]
                .as_array()
                .ok_or_else(|| Error::new(Code::InvalidValue, "/completed_stages"))?;
            for (stage, complete) in [
                ("household", confirmed),
                ("food_preferences", !v["preferences"].is_null()),
                ("usual_week", !v["usual_week"].is_null()),
                ("connections", !v["connections"].is_null()),
            ] {
                require(
                    stages.iter().any(|s| s == stage) == complete,
                    Code::InvalidValue,
                    "/completed_stages",
                )?;
            }
            member_check(v)?;
        }
        "HouseholdProfile" => {
            require(
                v["diners"]
                    .as_array()
                    .is_some_and(|a| a.iter().all(|d| d["restrictions"]["kind"] != "unconfirmed")),
                Code::InvalidValue,
                "/diners",
            )?;
            member_check(v)?;
        }
        "Preferences" => require(
            v["preferred_hands_on_minutes"]
                .as_u64()
                .is_some_and(|n| n <= 1440),
            Code::LimitExceeded,
            "/preferred_hands_on_minutes",
        )?,
        "ReminderSettings" => require(
            v["enabled"] == false
                || ["channels", "prompts"]
                    .iter()
                    .all(|k| v[*k].as_array().is_some_and(|a| !a.is_empty())),
            Code::InvalidValue,
            "/enabled",
        )?,
        "DeliveryPreference" => require(
            v["earliest"].as_str() < v["latest"].as_str(),
            Code::InvalidValue,
            "/latest",
        )?,
        "AppCommand" if v["kind"] == "save_household_draft" => validate_named("", &v["data"])?,
        _ => {}
    }
    Ok(())
}

fn terminal(job: &Value) -> bool {
    matches!(
        job["state"].as_str(),
        Some("needs_user" | "succeeded" | "failed" | "cancelled")
    )
}

#[cfg(test)]
mod parser_limits {
    use super::*;
    #[test]
    fn byte_depth_object_and_global_array_boundaries() {
        let bounds = CollectionBounds::default();
        let mut exact = b"null".to_vec();
        exact.resize(MAX_DOCUMENT_BYTES, b' ');
        assert!(parse_value_with_bounds(&exact, &bounds).is_ok());
        exact.push(b' ');
        assert_eq!(
            parse_value_with_bounds(&exact, &bounds).unwrap_err().code,
            Code::LimitExceeded
        );
        for (n, want) in [(32, true), (33, false)] {
            let s = format!("{}0{}", "[".repeat(n), "]".repeat(n));
            assert_eq!(parse_value_with_bounds(s.as_bytes(), &bounds).is_ok(), want);
        }
        for (n, want) in [(256, true), (257, false)] {
            let s = format!(
                "{{{}}}",
                (0..n)
                    .map(|i| format!("\"k{i}\":null"))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            assert_eq!(parse_value_with_bounds(s.as_bytes(), &bounds).is_ok(), want);
        }
        let a = format!("[{}]", vec!["null"; 4096].join(","));
        assert!(parse_value_with_bounds(a.as_bytes(), &bounds).is_ok());
        let bad = format!("[{},{{broken", vec!["null"; 4096].join(","));
        assert_eq!(
            parse_value_with_bounds(bad.as_bytes(), &bounds)
                .unwrap_err()
                .code,
            Code::LimitExceeded
        );
    }
    #[test]
    fn duplicate_keys_and_noninteger_tokens_refuse_without_losing_nested_keys() {
        let bounds = CollectionBounds::default();
        for s in [r#"{"x":{"a":1,"\u0061":2}}"#, r#"{"x":null,"x":null}"#] {
            assert_eq!(
                parse_value_with_bounds(s.as_bytes(), &bounds)
                    .unwrap_err()
                    .code,
                Code::DuplicateKey
            );
        }
        for s in ["1.0", "1e0", "null false", "{", "[NaN]", "\"\\ud800\""] {
            assert!(
                parse_value_with_bounds(s.as_bytes(), &bounds).is_err(),
                "{s}"
            );
        }
    }
}
pub(super) fn receipt_predicates<C: Serialize, R: Serialize>(
    command: &C,
    response: &R,
    cb: &[u8],
    rb: &[u8],
    b: &ReceiptBinding<'_>,
) -> Result<(), Error> {
    require(
        valid_id(b.command_id, "cm_") && valid_id(b.household_id, "hh_"),
        Code::Integrity,
        "",
    )?;
    let live = positive(b.live_revision)?;
    let projection = positive(b.response_revision)?;
    require(
        valid_digest(b.command_digest) && valid_digest(b.response_digest),
        Code::Integrity,
        "",
    )?;
    // The sole shared primitive is SHA-256 over original bytes (source-pinned,
    // literal-equivalence tested). No current type or validator is called.
    require(
        mstrchf_contracts::bytes_digest(cb).0 == b.command_digest
            && mstrchf_contracts::bytes_digest(rb).0 == b.response_digest,
        Code::DigestMismatch,
        "",
    )?;
    let c = serde_json::to_value(command).map_err(|_| Error::new(Code::Integrity, ""))?;
    let r = serde_json::to_value(response).map_err(|_| Error::new(Code::Integrity, ""))?;
    let expected = positive(
        c["expected_snapshot_revision"]
            .as_str()
            .ok_or_else(|| Error::new(Code::Integrity, ""))?,
    )?;
    let revision = positive(
        r["snapshot_revision"]
            .as_str()
            .ok_or_else(|| Error::new(Code::Integrity, ""))?,
    )?;
    require(
        c["command_id"] == b.command_id && r["household_draft"]["household_id"] == b.household_id,
        Code::IdentityMismatch,
        "",
    )?;
    require(
        expected.checked_add(1) == Some(revision) && revision == projection && revision <= live,
        Code::Integrity,
        "/snapshot_revision",
    )?;
    let data = &c["command"]["data"];
    let jobs = r["jobs"]
        .as_array()
        .ok_or_else(|| Error::new(Code::Integrity, "/jobs"))?;
    match c["command"]["kind"].as_str() {
        Some("save_household_draft") => {
            require(
                data["draft"]["household_id"] == b.household_id,
                Code::IdentityMismatch,
                "/draft/household_id",
            )?;
            let submitted = positive(
                data["draft"]["revision"]
                    .as_str()
                    .ok_or_else(|| Error::new(Code::Integrity, ""))?,
            )?;
            let returned = positive(
                r["household_draft"]["revision"]
                    .as_str()
                    .ok_or_else(|| Error::new(Code::Integrity, ""))?,
            )?;
            require(
                returned == submitted || submitted.checked_add(1) == Some(returned),
                Code::Integrity,
                "/draft/revision",
            )?;
            let mut content = r["household_draft"].clone();
            content["revision"] = data["draft"]["revision"].clone();
            require(content == data["draft"], Code::Integrity, "/draft")?;
            if !data["draft"]["connections"].is_null() {
                require(
                    data["draft"]["connections"]["privacy"] == r["privacy"],
                    Code::ContextStale,
                    "/draft/connections/privacy",
                )?;
            }
        }
        Some("set_privacy") => {
            let p = positive(
                data["policy"]["revision"]
                    .as_str()
                    .ok_or_else(|| Error::new(Code::Integrity, ""))?,
            )?;
            require(
                data["expected_policy_revision"] == data["policy"]["revision"],
                Code::Integrity,
                "/expected_policy_revision",
            )?;
            let next = p
                .checked_add(1)
                .ok_or_else(|| Error::new(Code::ArithmeticOverflow, "/policy/revision"))?;
            let mut expected = data["policy"].clone();
            expected["revision"] = Value::String(next.to_string());
            require(
                expected == r["privacy"] && jobs.iter().all(terminal),
                Code::Integrity,
                "/privacy",
            )?;
        }
        Some("cancel_job") => {
            if let Some(job) = jobs.iter().find(|j| j["job_id"] == data["job_id"]) {
                require(terminal(job), Code::Integrity, "/jobs")?;
            } else {
                require(jobs.len() == 1024, Code::Integrity, "/jobs")?;
            }
        }
        _ => return Err(Error::new(Code::UnknownClosure, "/command")),
    }
    Ok(())
}

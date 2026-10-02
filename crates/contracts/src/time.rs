use chrono::{LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use ts_rs::TS;

fn parse_date(value: &str) -> Option<NaiveDate> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return None;
    }
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()?;
    if ("1900-01-01"..="2199-12-31").contains(&value) {
        Some(date)
    } else {
        None
    }
}

fn parse_time(value: &str) -> Option<NaiveTime> {
    let bytes = value.as_bytes();
    if bytes.len() != 8
        || bytes[2] != b':'
        || bytes[5] != b':'
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 2 || i == 5 || b.is_ascii_digit())
    {
        return None;
    }
    let time = NaiveTime::parse_from_str(value, "%H:%M:%S").ok()?;
    if value[6..8] == *"60" {
        None
    } else {
        Some(time)
    }
}

fn parse_instant(value: &str) -> Option<chrono::DateTime<Utc>> {
    let bytes = value.as_bytes();
    if bytes.len() != 20 || bytes[10] != b'T' || bytes[19] != b'Z' {
        return None;
    }
    let date = parse_iso_date(&value[0..10])?;
    let time = parse_time(&value[11..19])?;
    if bytes[11..19].len() != 8 {
        return None;
    }
    Some(NaiveDateTime::new(date, time).and_utc())
}

fn parse_iso_date(value: &str) -> Option<NaiveDate> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return None;
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

macro_rules! string_primitive {
    ($name:ident, $validator:ident, $schema:expr) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, TS)]
        pub struct $name(#[ts(type = "string")] pub String);

        impl TryFrom<&str> for $name {
            type Error = crate::DomainError;
            fn try_from(value: &str) -> Result<Self, Self::Error> {
                if $validator(value).is_none() {
                    Err(crate::DomainError::new(crate::ErrorCode::InvalidValue, ""))
                } else {
                    Ok(Self(value.to_owned()))
                }
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = String::deserialize(deserializer)
                    .map_err(|_| D::Error::custom("CODE:invalid_value"))?;
                Self::try_from(value.as_str()).map_err(|_| D::Error::custom("CODE:invalid_value"))
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> std::borrow::Cow<'static, str> {
                stringify!($name).into()
            }
            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                $schema
            }
        }
    };
}

string_primitive!(
    LocalDate,
    parse_date,
    json_schema!({"type":"string","pattern":"^(19|20|21)[0-9]{2}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])$","description":"Gregorian date 1900-01-01 through 2199-12-31"})
);
string_primitive!(
    LocalTime,
    parse_time,
    json_schema!({"type":"string","pattern":"^([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]$"})
);
string_primitive!(
    Instant,
    parse_instant,
    json_schema!({"type":"string","pattern":"^[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])T([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]Z$","description":"Exact UTC timestamp with whole seconds"})
);

impl Instant {
    pub fn timestamp_seconds(&self) -> i64 {
        // Every constructed Instant has passed the strict parser.
        parse_instant(&self.0)
            .expect("validated Instant")
            .timestamp()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, TS)]
pub struct ZoneId(#[ts(type = "string")] pub String);

impl TryFrom<&str> for ZoneId {
    type Error = crate::DomainError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value
            .parse::<Tz>()
            .map(|_| Self(value.to_owned()))
            .map_err(|_| crate::DomainError::new(crate::ErrorCode::InvalidValue, ""))
    }
}

impl Serialize for ZoneId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ZoneId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)
            .map_err(|_| D::Error::custom("CODE:invalid_value"))?;
        Self::try_from(value.as_str()).map_err(|_| D::Error::custom("CODE:invalid_value"))
    }
}

impl JsonSchema for ZoneId {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ZoneId".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"string","minLength":1,"pattern":"^[A-Za-z0-9_+.-]+(?:/[A-Za-z0-9_+.-]+)*$","description":"Case-sensitive IANA timezone accepted by pinned chrono-tz"})
    }
}

crate::string_enum! {pub enum Resolution { Unambiguous, Earlier, Later }}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ZonedMoment {
    pub local_date: LocalDate,
    pub local_time: LocalTime,
    pub zone: ZoneId,
    pub resolution: Resolution,
    pub instant: Instant,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawZonedMoment {
    local_date: LocalDate,
    local_time: LocalTime,
    zone: ZoneId,
    resolution: Resolution,
    instant: Instant,
}

impl ZonedMoment {
    pub fn new(
        local_date: LocalDate,
        local_time: LocalTime,
        zone: ZoneId,
        resolution: Resolution,
        instant: Instant,
    ) -> Result<Self, crate::DomainError> {
        let date = parse_date(&local_date.0)
            .ok_or_else(|| crate::DomainError::new(crate::ErrorCode::InvalidValue, "local_date"))?;
        let time = parse_time(&local_time.0)
            .ok_or_else(|| crate::DomainError::new(crate::ErrorCode::InvalidValue, "local_time"))?;
        let tz: Tz = zone
            .0
            .parse()
            .map_err(|_| crate::DomainError::new(crate::ErrorCode::InvalidValue, "zone"))?;
        let actual = parse_instant(&instant.0)
            .ok_or_else(|| crate::DomainError::new(crate::ErrorCode::InvalidValue, "instant"))?;
        let local = NaiveDateTime::new(date, time);
        let expected = match (tz.from_local_datetime(&local), resolution) {
            (LocalResult::Single(value), Resolution::Unambiguous) => value,
            (LocalResult::Ambiguous(a, b), Resolution::Earlier) => {
                if a <= b {
                    a
                } else {
                    b
                }
            }
            (LocalResult::Ambiguous(a, b), Resolution::Later) => {
                if a >= b {
                    a
                } else {
                    b
                }
            }
            _ => {
                return Err(crate::DomainError::new(
                    crate::ErrorCode::InvalidValue,
                    "resolution",
                ));
            }
        };
        if expected.with_timezone(&Utc) != actual
            || actual.with_timezone(&tz).naive_local() != local
        {
            return Err(crate::DomainError::new(
                crate::ErrorCode::InvalidValue,
                "instant",
            ));
        }
        Ok(Self {
            local_date,
            local_time,
            zone,
            resolution,
            instant,
        })
    }
}

impl<'de> Deserialize<'de> for ZonedMoment {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawZonedMoment::deserialize(deserializer)?;
        Self::new(
            raw.local_date,
            raw.local_time,
            raw.zone,
            raw.resolution,
            raw.instant,
        )
        .map_err(|_| D::Error::custom("CODE:invalid_value"))
    }
}

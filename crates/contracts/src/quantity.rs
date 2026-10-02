use crate::{DomainError, ErrorCode};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use ts_rs::TS;

pub const MAX_NUMERATOR: u64 = 1_000_000_000_000;
pub const MAX_DENOMINATOR: u64 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, TS)]
pub struct DecimalU64(#[ts(type = "string")] pub u64);

impl Serialize for DecimalU64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for DecimalU64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)
            .map_err(|_| D::Error::custom("CODE:invalid_value"))?;
        if value.starts_with('-') {
            return Err(D::Error::custom("CODE:invalid_value"));
        }
        if value.is_empty()
            || (value.len() > 1 && value.starts_with('0'))
            || !value.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(D::Error::custom("CODE:noncanonical_value"));
        }
        value
            .parse::<u64>()
            .map(Self)
            .map_err(|_| D::Error::custom("CODE:limit_exceeded"))
    }
}

impl JsonSchema for DecimalU64 {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "DecimalU64".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"string","pattern":crate::revision::decimal_u64_pattern(true),"description":"Canonical decimal u64, maximum 18446744073709551615"})
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
pub struct Rational {
    pub numerator: DecimalU64,
    pub denominator: DecimalU64,
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl Rational {
    pub fn new(numerator: u64, denominator: u64) -> Result<Self, DomainError> {
        if denominator == 0 {
            return Err(DomainError::new(ErrorCode::InvalidValue, ""));
        }
        if (numerator == 0 && denominator != 1) || gcd(numerator.into(), denominator.into()) != 1 {
            return Err(DomainError::new(ErrorCode::NoncanonicalValue, ""));
        }
        if numerator > MAX_NUMERATOR || denominator > MAX_DENOMINATOR {
            return Err(DomainError::new(ErrorCode::LimitExceeded, ""));
        }
        Ok(Self {
            numerator: DecimalU64(numerator),
            denominator: DecimalU64(denominator),
        })
    }

    fn reduced(numerator: u128, denominator: u128) -> Result<Self, DomainError> {
        if denominator == 0 {
            return Err(DomainError::new(ErrorCode::InvalidValue, ""));
        }
        let divisor = gcd(numerator, denominator);
        let n = numerator / divisor;
        let d = denominator / divisor;
        if n > MAX_NUMERATOR as u128 || d > MAX_DENOMINATOR as u128 {
            return Err(DomainError::new(ErrorCode::LimitExceeded, ""));
        }
        Self::new(n as u64, d as u64)
    }

    pub fn checked_add(&self, rhs: &Self) -> Result<Self, DomainError> {
        Self::new(self.numerator.0, self.denominator.0)?;
        Self::new(rhs.numerator.0, rhs.denominator.0)?;
        let a = (self.numerator.0 as u128).checked_mul(rhs.denominator.0 as u128);
        let b = (rhs.numerator.0 as u128).checked_mul(self.denominator.0 as u128);
        let d = (self.denominator.0 as u128).checked_mul(rhs.denominator.0 as u128);
        let n = a.zip(b).and_then(|(a, b)| a.checked_add(b));
        n.zip(d)
            .map(|(n, d)| Self::reduced(n, d))
            .unwrap_or_else(|| Err(DomainError::new(ErrorCode::ArithmeticOverflow, "")))
    }

    pub fn checked_mul(&self, rhs: &Self) -> Result<Self, DomainError> {
        Self::new(self.numerator.0, self.denominator.0)?;
        Self::new(rhs.numerator.0, rhs.denominator.0)?;
        let n = (self.numerator.0 as u128).checked_mul(rhs.numerator.0 as u128);
        let d = (self.denominator.0 as u128).checked_mul(rhs.denominator.0 as u128);
        n.zip(d)
            .map(|(n, d)| Self::reduced(n, d))
            .unwrap_or_else(|| Err(DomainError::new(ErrorCode::ArithmeticOverflow, "")))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRational {
    numerator: DecimalU64,
    denominator: DecimalU64,
}

impl<'de> Deserialize<'de> for Rational {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawRational::deserialize(deserializer)?;
        Self::new(raw.numerator.0, raw.denominator.0).map_err(|e| {
            D::Error::custom(match e.code {
                ErrorCode::LimitExceeded => "CODE:limit_exceeded",
                ErrorCode::InvalidValue => "CODE:invalid_value",
                _ => "CODE:noncanonical_value",
            })
        })
    }
}

impl JsonSchema for Rational {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Rational".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type":"object", "additionalProperties":false,
            "required":["numerator","denominator"],
            "properties":{
                "numerator":{"type":"string","pattern":"^(0|[1-9][0-9]{0,11}|1000000000000)$"},
                "denominator":{"type":"string","pattern":"^([1-9][0-9]{0,5}|1000000)$"}
            },
            "description":"Reduced rational; gcd must equal 1 and zero must be 0/1."
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(transparent)]
pub struct PositiveRational(pub Rational);

impl PositiveRational {
    pub fn new(numerator: u64, denominator: u64) -> Result<Self, DomainError> {
        let value = Rational::new(numerator, denominator)?;
        if numerator == 0 {
            Err(DomainError::new(ErrorCode::InvalidValue, ""))
        } else {
            Ok(Self(value))
        }
    }
}

impl<'de> Deserialize<'de> for PositiveRational {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Rational::deserialize(deserializer)?;
        if value.numerator.0 == 0 {
            return Err(D::Error::custom("CODE:invalid_value"));
        }
        Ok(Self(value))
    }
}

impl JsonSchema for PositiveRational {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "PositiveRational".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type":"object", "additionalProperties":false,
            "required":["numerator","denominator"],
            "properties":{
                "numerator":{"type":"string","pattern":"^([1-9][0-9]{0,11}|1000000000000)$"},
                "denominator":{"type":"string","pattern":"^([1-9][0-9]{0,5}|1000000)$"}
            },
            "description":"Positive reduced rational; gcd must equal 1."
        })
    }
}

crate::string_enum! {pub enum Unit { G, Ml, Each }}
crate::string_enum! {pub enum IngredientForm { Raw, Dry, Cooked, Drained, Whole, Prepared }}
crate::record! {pub struct Quantity {pub amount: PositiveRational, pub unit: Unit, pub form: IngredientForm}}
crate::record! {pub struct NonnegativeQuantity {pub amount: Rational, pub unit: Unit, pub form: IngredientForm}}

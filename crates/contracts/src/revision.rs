use crate::{DomainError, ErrorCode};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use ts_rs::TS;

fn parse_positive(value: &str) -> Result<u64, ErrorCode> {
    if value.is_empty() || value.starts_with('0') || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ErrorCode::NoncanonicalValue);
    }
    value.parse().map_err(|_| ErrorCode::LimitExceeded)
}

/// Exact decimal-u64 grammar for generated schemas. The long alternatives
/// describe each first digit smaller than u64::MAX at the same width.
pub(crate) fn decimal_u64_pattern(allow_zero: bool) -> String {
    let max = u64::MAX.to_string();
    let digits = max.as_bytes();
    let mut branches = Vec::new();
    if allow_zero {
        branches.push("0".to_owned());
    }
    branches.push("[1-9][0-9]{0,18}".to_owned());
    for (i, &byte) in digits.iter().enumerate() {
        let digit = byte - b'0';
        let minimum = if i == 0 { 1 } else { 0 };
        if digit == minimum {
            continue;
        }
        let prefix = &max[..i];
        let lower_digit = if digit == minimum + 1 {
            minimum.to_string()
        } else {
            format!("[{minimum}-{}]", digit - 1)
        };
        let rest = digits.len() - i - 1;
        let suffix = if rest == 0 {
            String::new()
        } else {
            format!("[0-9]{{{rest}}}")
        };
        branches.push(format!("{prefix}{lower_digit}{suffix}"));
    }
    branches.push(max);
    format!("^(?:{})$", branches.join("|"))
}

macro_rules! positive_counter {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, TS)]
        pub struct $name(#[ts(type = "string")] pub u64);

        impl $name {
            pub fn new(value: u64) -> Result<Self, DomainError> {
                if value == 0 { Err(DomainError::new(ErrorCode::InvalidValue, "")) }
                else { Ok(Self(value)) }
            }

            pub fn checked_increment(self) -> Result<Self, DomainError> {
                Self::new(self.0)?;
                self.0.checked_add(1).map(Self)
                    .ok_or_else(|| DomainError::new(ErrorCode::ArithmeticOverflow, ""))
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = String::deserialize(deserializer)
                    .map_err(|_| D::Error::custom("CODE:invalid_value"))?;
                parse_positive(&value).map(Self)
                    .map_err(|code| D::Error::custom(match code {
                        ErrorCode::LimitExceeded => "CODE:limit_exceeded",
                        _ => "CODE:noncanonical_value",
                    }))
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> std::borrow::Cow<'static, str> { stringify!($name).into() }
            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                json_schema!({"type":"string","pattern":decimal_u64_pattern(false),"description":"Canonical positive decimal u64, maximum 18446744073709551615"})
            }
        }
    };
}

positive_counter!(Revision);
positive_counter!(Generation);

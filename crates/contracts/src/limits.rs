use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize};
use std::{borrow::Cow, marker::PhantomData};
use ts_rs::{Config, TS, TypeVisitor};
pub const MAX_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_DEPTH: usize = 32;
pub const MAX_ARTEFACT_BYTES: usize = 32 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(type = "1")]
pub struct SchemaVersion;
impl Serialize for SchemaVersion {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(1)
    }
}
impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let n = u16::deserialize(d)?;
        if n != 1 {
            return Err(serde::de::Error::custom("CODE:unsupported_version"));
        }
        Ok(Self)
    }
}
impl JsonSchema for SchemaVersion {
    fn schema_name() -> Cow<'static, str> {
        "SchemaVersion".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({"type":"integer","const":1})
    }
}

/// Required on the wire, including when its value is null.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Nullable<T>(pub Option<T>);
impl<'de, T: serde::de::DeserializeOwned> Deserialize<'de> for Nullable<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // deserialize_any (via Value) deliberately rejects Serde's missing-field sentinel.
        let v = serde_json::Value::deserialize(d)?;
        if v.is_null() {
            Ok(Self(None))
        } else {
            serde_json::from_value(v)
                .map(|v| Self(Some(v)))
                .map_err(serde::de::Error::custom)
        }
    }
}
impl<T: JsonSchema> JsonSchema for Nullable<T> {
    fn schema_name() -> Cow<'static, str> {
        format!("Nullable_{}", T::schema_name()).into()
    }
    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        <Option<T>>::json_schema(g)
    }
}
impl<T: TS> TS for Nullable<T> {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(c: &Config) -> String {
        <Option<T>>::name(c)
    }
    fn inline(c: &Config) -> String {
        <Option<T>>::inline(c)
    }
    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        v.visit::<T>();
        T::visit_dependencies(v);
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct BoundedVec<T, const MIN: usize, const MAX: usize>(pub Vec<T>);
impl<T, const A: usize, const B: usize> std::ops::Deref for BoundedVec<T, A, B> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.0
    }
}
impl<T, const A: usize, const B: usize> std::ops::DerefMut for BoundedVec<T, A, B> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.0
    }
}
impl<'de, T: Deserialize<'de>, const MIN: usize, const MAX: usize> Deserialize<'de>
    for BoundedVec<T, MIN, MAX>
{
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V<T, const A: usize, const B: usize>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>, const A: usize, const B: usize> serde::de::Visitor<'de>
            for V<T, A, B>
        {
            type Value = BoundedVec<T, A, B>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "bounded array")
            }
            fn visit_seq<S: serde::de::SeqAccess<'de>>(
                self,
                mut s: S,
            ) -> Result<Self::Value, S::Error> {
                let mut a = Vec::new();
                loop {
                    if a.len() == B {
                        s.next_element_seed(RejectOverLimit)?;
                        break;
                    }
                    let Some(x) = s.next_element::<T>()? else {
                        break;
                    };
                    a.push(x);
                }
                if a.len() < A {
                    return Err(serde::de::Error::custom("CODE:invalid_value"));
                }
                Ok(BoundedVec(a))
            }
        }
        d.deserialize_seq(V::<T, MIN, MAX>(PhantomData))
    }
}
struct RejectOverLimit;
impl<'de> serde::de::DeserializeSeed<'de> for RejectOverLimit {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, _: D) -> Result<Self::Value, D::Error> {
        Err(serde::de::Error::custom("CODE:limit_exceeded"))
    }
}
impl<T: JsonSchema, const A: usize, const B: usize> JsonSchema for BoundedVec<T, A, B> {
    fn schema_name() -> Cow<'static, str> {
        format!("Array_{}_{}_{}", T::schema_name(), A, B).into()
    }
    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        let mut s = <Vec<T>>::json_schema(g);
        s.insert("minItems".into(), A.into());
        s.insert("maxItems".into(), B.into());
        s
    }
}
impl<T: TS, const A: usize, const B: usize> TS for BoundedVec<T, A, B> {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;
    fn name(c: &Config) -> String {
        <Vec<T>>::name(c)
    }
    fn inline(c: &Config) -> String {
        <Vec<T>>::inline(c)
    }
    fn visit_dependencies(v: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
        v.visit::<T>();
        T::visit_dependencies(v);
    }
}

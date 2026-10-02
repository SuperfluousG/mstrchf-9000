use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize};
use std::borrow::Cow;
use ts_rs::{Config, TS};
macro_rules! wire_string {
    ($name:ident,$pattern:expr,$valid:expr) => {
        wire_string!($name, $pattern, $valid, None::<usize>);
    };
    ($name:ident,$pattern:expr,$valid:expr,$max_length:expr) => {
        #[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord,Serialize)]
        #[serde(transparent)] pub struct $name(pub String);
        impl<'de> Deserialize<'de> for $name {fn deserialize<D:Deserializer<'de>>(d:D)->Result<Self,D::Error>{let s=String::deserialize(d)?;if !($valid)(&s){return Err(serde::de::Error::custom("CODE:invalid_value"));}Ok(Self(s))}}
        impl JsonSchema for $name{fn schema_name()->Cow<'static,str>{stringify!($name).into()}fn json_schema(_: &mut SchemaGenerator)->Schema{let mut schema=schemars::json_schema!({"type":"string","pattern":$pattern});if let Some(max_length)=$max_length{schema.insert("maxLength".into(),max_length.into());}schema}}
        impl TS for $name{type WithoutGenerics=Self;type OptionInnerType=Self;fn output_path()->Option<std::path::PathBuf>{Some(std::path::PathBuf::from(concat!(stringify!($name),".ts")))}fn name(_: &Config)->String{stringify!($name).into()}fn inline(_: &Config)->String{format!("string & {{ readonly __brand: \"{}\" }}",stringify!($name))}fn decl(c:&Config)->String{format!("type {} = {};",Self::name(c),Self::inline(c))}fn decl_concrete(c:&Config)->String{Self::decl(c)}fn visit_dependencies(v:&mut impl ts_rs::TypeVisitor) where Self:'static{v.visit::<Self>();}}
    };
}
fn valid_id(s: &str, p: &str) -> bool {
    s.strip_prefix(p).is_some_and(|h| {
        h.len() == 32
            && h.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && h.bytes().any(|b| b != b'0')
    })
}
macro_rules! id {
    ($n:ident,$p:literal) => {
        wire_string!(
            $n,
            concat!("^", $p, "(?!0{32}$)[0-9a-f]{32}$"),
            |s: &str| valid_id(s, $p)
        );
    };
}
id!(HouseholdId, "hh_");
id!(DinerId, "dn_");
id!(GuestId, "gu_");
id!(OccasionId, "oc_");
id!(WeekId, "wk_");
id!(RecipeId, "rc_");
id!(IngredientId, "ig_");
id!(ExtraId, "ex_");
id!(BatchId, "ba_");
id!(ComponentId, "co_");
id!(AllocationId, "al_");
id!(PrepSessionId, "ps_");
id!(DemandLineId, "dl_");
id!(EvidenceId, "ev_");
id!(AssetId, "as_");
id!(JobId, "jb_");
id!(ResultId, "rs_");
id!(OperationId, "op_");
id!(ArtefactId, "ar_");
id!(BasketId, "bk_");
id!(OrderId, "or_");
id!(CalendarEventId, "ce_");
id!(CalendarDestinationId, "cd_");
id!(ApprovalId, "au_");
id!(CommandId, "cm_");
id!(ReadId, "rd_");
id!(ReadObservationId, "ob_");
fn valid_digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
macro_rules! digest {
    ($n:ident) => {
        wire_string!($n, "^[0-9a-f]{64}$", valid_digest);
    };
}
digest!(Digest);
digest!(InputDigest);
digest!(RecipeDigest);
digest!(ResultDigest);
digest!(PayloadDigest);
digest!(AssetDigest);
digest!(PlanInputDigest);
digest!(DemandDigest);
wire_string!(GrantToken, "^(?!0{32}$)[0-9a-f]{32}$", |s: &str| valid_id(
    &format!("x_{s}"),
    "x_"
));
fn valid_text(s: &str, max: usize, multiline: bool) -> bool {
    !s.is_empty()
        && s.trim() == s
        && s.chars().count() <= max
        && !s
            .chars()
            .any(|c| c.is_control() && !(multiline && (c == '\n' || c == '\t')))
}
macro_rules! text_type{($n:ident,$m:literal,$multi:literal)=>{
    #[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord,Serialize,TS)] #[serde(transparent)] #[ts(type="string")]pub struct $n(pub String);
    impl<'de> Deserialize<'de> for $n{fn deserialize<D:Deserializer<'de>>(d:D)->Result<Self,D::Error>{let s=String::deserialize(d)?;if !valid_text(&s,$m,$multi){return Err(serde::de::Error::custom("CODE:invalid_value"));}Ok(Self(s))}}
    impl JsonSchema for $n{fn schema_name()->Cow<'static,str>{stringify!($n).into()}fn json_schema(_: &mut SchemaGenerator)->Schema{schemars::json_schema!({"type":"string","minLength":1,"maxLength":$m})}}
};}
text_type!(Text, 4000, true);
text_type!(Title, 200, false);
text_type!(NameText, 80, false);
text_type!(LongText, 65536, true);
wire_string!(
    HttpUrl,
    // Explicit Rust control/White_Space set, rather than ECMAScript's different
    // whitespace shorthand. The final assertion also forbids trailing newlines.
    "^https?://[^\\u0000-\\u0020\\u007f-\\u00a0\\u1680\\u2000-\\u200a\\u2028\\u2029\\u202f\\u205f\\u3000]+(?![\\s\\S])",
    |s: &str| s.chars().count() <= 2048
        && (s.starts_with("https://") || s.starts_with("http://"))
        && s.split_once("://").is_some_and(|(_, a)| !a.is_empty())
        && !s.chars().any(|c| c.is_control() || c.is_whitespace()),
    Some(2048usize)
);

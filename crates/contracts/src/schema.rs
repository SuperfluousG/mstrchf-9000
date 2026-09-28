//! Deterministic public contract generation from the Rust wire types.
use crate::*;
use schemars::{JsonSchema, generate::SchemaSettings};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{
    any::TypeId,
    collections::{BTreeMap, HashSet},
    fs,
    path::Path,
};
use ts_rs::{Config, TS, TypeVisitor};

pub const ROOT_NAMES: [&str; 19] = [
    "household-draft",
    "household-profile",
    "planning-context",
    "meal-requirements",
    "recipe-candidate",
    "recipe-revision",
    "week",
    "week-plan",
    "derived-plan",
    "job-request",
    "job-result",
    "operation-request",
    "operation-grant",
    "operation-observation",
    "worker-stop-evidence",
    "app-command",
    "snapshot",
    "event-batch",
    "read-observation",
];

const HEADER: &str = "Generated from mstrchf-contracts; run cargo run --locked -p mstrchf-contracts --bin generate-contracts";

pub fn schema_for<T: JsonSchema>() -> Value {
    let mut generator = SchemaSettings::draft2020_12().into_generator();
    serde_json::to_value(generator.root_schema_for::<T>()).expect("schema is JSON")
}

fn root_schema<T: JsonSchema>(name: &str) -> Result<Vec<u8>, DomainError> {
    let mut value = schema_for::<T>();
    let object = value
        .as_object_mut()
        .ok_or_else(|| DomainError::new(ErrorCode::InvalidValue, name))?;
    object.insert(
        "$id".into(),
        json!(format!("urn:mstrchf:contracts:v1:{name}")),
    );
    object.insert("$comment".into(), json!(HEADER));
    verify_local_refs(&value)?;
    let mut bytes = serde_json::to_vec_pretty(&value)
        .map_err(|_| DomainError::new(ErrorCode::InvalidValue, name))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn verify_local_refs(value: &Value) -> Result<(), DomainError> {
    match value {
        Value::Object(fields) => {
            if let Some(reference) = fields.get("$ref") {
                let Some(reference) = reference.as_str() else {
                    return Err(DomainError::new(ErrorCode::InvalidValue, "$ref"));
                };
                if !reference.starts_with("#/$defs/") {
                    return Err(DomainError::new(ErrorCode::InvalidValue, "$ref"));
                }
            }
            for child in fields.values() {
                verify_local_refs(child)?;
            }
        }
        Value::Array(items) => {
            for child in items {
                verify_local_refs(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn parse_root(root: &str, bytes: &[u8]) -> Result<(), DomainError> {
    macro_rules! parse {
        ($type:ty) => {
            crate::parse_document::<$type>(bytes).map(|_| ())
        };
    }
    match root {
        "household-draft" => parse!(HouseholdDraft),
        "household-profile" => parse!(HouseholdProfile),
        "planning-context" => parse!(PlanningContext),
        "meal-requirements" => parse!(MealRequirementsDocumentV1),
        "recipe-candidate" => parse!(RecipeCandidate),
        "recipe-revision" => parse!(RecipeRevision),
        "week" => parse!(Week),
        "week-plan" => parse!(WeekPlanDocumentV1),
        "derived-plan" => parse!(DerivedPlan),
        "job-request" => parse!(JobRequestV1),
        "job-result" => parse!(JobResultV1),
        "operation-request" => parse!(OperationRequest),
        "operation-grant" => parse!(OperationGrant),
        "operation-observation" => parse!(OperationObservation),
        "worker-stop-evidence" => parse!(WorkerStopEvidence),
        "app-command" => parse!(AppCommandV1),
        "snapshot" => parse!(Snapshot),
        "event-batch" => parse!(EventBatchDocumentV1),
        "read-observation" => parse!(ReadObservationV1),
        _ => Err(DomainError::new(ErrorCode::InvalidValue, "root")),
    }
}

struct TsCollector {
    config: Config,
    seen: HashSet<TypeId>,
    declarations: BTreeMap<String, String>,
}

impl TsCollector {
    fn new() -> Self {
        Self {
            config: Config::default(),
            seen: HashSet::new(),
            declarations: BTreeMap::new(),
        }
    }
}

impl TypeVisitor for TsCollector {
    fn visit<T: TS + 'static + ?Sized>(&mut self) {
        if !self.seen.insert(TypeId::of::<T>()) {
            return;
        }
        if T::output_path().is_some() {
            let declaration = T::decl(&self.config);
            let name = T::ident(&self.config);
            self.declarations.entry(name).or_insert(declaration);
        }
        T::visit_dependencies(self);
    }
}

fn typescript() -> Result<Vec<u8>, DomainError> {
    let mut collector = TsCollector::new();
    macro_rules! collect {
        ($type:ty) => {
            collector.visit::<$type>()
        };
    }
    collect!(HouseholdDraft);
    collect!(HouseholdProfile);
    collect!(PlanningContext);
    collect!(MealRequirementsDocumentV1);
    collect!(RecipeCandidate);
    collect!(RecipeRevision);
    collect!(Week);
    collect!(WeekPlanDocumentV1);
    collect!(DerivedPlan);
    collect!(JobRequestV1);
    collect!(JobResultV1);
    collect!(OperationRequest);
    collect!(OperationGrant);
    collect!(OperationObservation);
    collect!(WorkerStopEvidence);
    collect!(AppCommandV1);
    collect!(Snapshot);
    collect!(EventBatchDocumentV1);
    collect!(ReadObservationV1);
    if collector.declarations.len() < ROOT_NAMES.len() {
        return Err(DomainError::new(ErrorCode::InvalidValue, "typescript"));
    }
    let mut output = format!("// {HEADER}\n// Do not edit.\n\n");
    for declaration in collector.declarations.values() {
        output.push_str("export ");
        output.push_str(declaration.trim());
        output.push_str("\n\n");
    }
    Ok(format!("{}\n", output.trim_end()).into_bytes())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn render_contracts() -> Result<BTreeMap<String, Vec<u8>>, DomainError> {
    let mut files = BTreeMap::new();
    macro_rules! emit {
        ($name:literal, $type:ty) => {
            files.insert(
                format!("contracts/v1/{}.schema.json", $name),
                root_schema::<$type>($name)?,
            );
        };
    }
    emit!("household-draft", HouseholdDraft);
    emit!("household-profile", HouseholdProfile);
    emit!("planning-context", PlanningContext);
    emit!("meal-requirements", MealRequirementsDocumentV1);
    emit!("recipe-candidate", RecipeCandidate);
    emit!("recipe-revision", RecipeRevision);
    emit!("week", Week);
    emit!("week-plan", WeekPlanDocumentV1);
    emit!("derived-plan", DerivedPlan);
    emit!("job-request", JobRequestV1);
    emit!("job-result", JobResultV1);
    emit!("operation-request", OperationRequest);
    emit!("operation-grant", OperationGrant);
    emit!("operation-observation", OperationObservation);
    emit!("worker-stop-evidence", WorkerStopEvidence);
    emit!("app-command", AppCommandV1);
    emit!("snapshot", Snapshot);
    emit!("event-batch", EventBatchDocumentV1);
    emit!("read-observation", ReadObservationV1);
    files.insert("app/generated/contracts.ts".into(), typescript()?);
    let hashes: BTreeMap<String, String> = files
        .iter()
        .map(|(path, bytes)| (path.clone(), sha256_hex(bytes)))
        .collect();
    let manifest = json!({
        "$comment": HEADER,
        "schema_version": 1,
        "roots": ROOT_NAMES,
        "hashes": hashes,
    });
    let mut bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|_| DomainError::new(ErrorCode::InvalidValue, "manifest"))?;
    bytes.push(b'\n');
    files.insert("contracts/v1/manifest.json".into(), bytes);
    Ok(files)
}

pub fn write_generated(root: &Path) -> Result<(), DomainError> {
    for (relative, bytes) in render_contracts()? {
        let destination = root.join(relative);
        let parent = destination
            .parent()
            .ok_or_else(|| DomainError::new(ErrorCode::StorageFailure, "output"))?;
        fs::create_dir_all(parent)
            .map_err(|_| DomainError::new(ErrorCode::StorageFailure, "output"))?;
        fs::write(destination, bytes)
            .map_err(|_| DomainError::new(ErrorCode::StorageFailure, "output"))?;
    }
    Ok(())
}

pub fn check_generated(root: &Path) -> Result<(), DomainError> {
    let mut actual = BTreeMap::new();
    let schema_dir = root.join("contracts/v1");
    let entries = fs::read_dir(schema_dir)
        .map_err(|_| DomainError::new(ErrorCode::StorageFailure, "contracts/v1"))?;
    for entry in entries {
        let entry =
            entry.map_err(|_| DomainError::new(ErrorCode::StorageFailure, "contracts/v1"))?;
        if !entry
            .file_type()
            .map_err(|_| DomainError::new(ErrorCode::StorageFailure, "contracts/v1"))?
            .is_file()
        {
            return Err(DomainError::new(ErrorCode::InvalidValue, "contracts/v1"));
        }
        let relative = format!("contracts/v1/{}", entry.file_name().to_string_lossy());
        let bytes = fs::read(entry.path())
            .map_err(|_| DomainError::new(ErrorCode::StorageFailure, &relative))?;
        actual.insert(relative, bytes);
    }
    if let Ok(bytes) = fs::read(root.join("app/generated/contracts.ts")) {
        actual.insert("app/generated/contracts.ts".into(), bytes);
    }
    compare_generated_bytes(&actual)
}

/// Compare an observed output set with the exact generated set, without altering it.
pub fn compare_generated_bytes(actual: &BTreeMap<String, Vec<u8>>) -> Result<(), DomainError> {
    let expected = render_contracts()?;
    for (relative, bytes) in &expected {
        match actual.get(relative) {
            None => return Err(DomainError::new(ErrorCode::MissingReference, relative)),
            Some(actual_bytes) if actual_bytes != bytes => {
                return Err(DomainError::new(ErrorCode::DigestMismatch, relative));
            }
            _ => {}
        }
    }
    for relative in actual.keys() {
        if !expected.contains_key(relative) {
            return Err(DomainError::new(ErrorCode::InvalidValue, relative));
        }
    }
    Ok(())
}

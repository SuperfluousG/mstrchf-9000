//! Time custody through actual commands and an explicitly re-signed negative backup.
use mstrchf_browser::{maker_packets::*, run_json};
use mstrchf_contracts::*;
use serde_json::{Value, json};

const BEFORE: &str = "2026-10-12T17:16:55Z";
const POLICY: &str = "2026-10-12T17:16:56Z";
const AFTER: &str = "2026-10-12T20:00:00Z";
fn call(request: Value) -> Value {
    serde_json::from_slice(&run_json(&serde_json::to_vec(&request).unwrap())).unwrap()
}
fn good(request: Value) -> Value {
    let out = call(request);
    assert_eq!(out["ok"], true, "{out}"); out["value"].clone()
}
fn legacy() -> Value {
    serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone()
}
fn envelope(state: &Value) -> Value {
    json!({"schema_version":1,"command_id":"cm_000000000000000000000000000001f4","expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":{"kind":"admit_reviewed_recipe","data":{"packet":porridge_packet_ref().unwrap(),"expected_catalogue_revision":"1"}}})
}
fn assert_refusal(out: Value, path: &str) {
    assert_eq!(out["ok"], false, "{out}");
    assert_eq!(out["error"]["code"], "authority_denied");
    assert_eq!(out["error"]["issues"][0]["path"], path);
    assert_eq!(out["error"]["retry"], "never");
}

#[test]
fn compiled_packet_not_before_is_the_latest_exact_typed_evidence_or_review_time() {
    let packet = known_packet(&porridge_packet_ref().unwrap()).unwrap();
    assert_eq!(packet.not_before(), &Instant(POLICY.into()));
    let mut observations = packet.record().evidence_documents.as_array().unwrap().iter()
        .map(|document|parse_document::<Instant>(&canonical_bytes(&document["at"]).unwrap()).unwrap())
        .collect::<Vec<_>>();
    observations.extend(packet.recipe().reviews.iter().map(|review|parse_document::<Instant>(&canonical_bytes(&review.at).unwrap()).unwrap()));
    assert_eq!(observations.len(),14);
    assert_eq!(observations.iter().max().unwrap(),packet.not_before());
}

#[test]
fn ordinary_admission_cannot_precede_policy_and_accepts_the_exact_boundary() {
    let state = legacy(); let command = envelope(&state);
    assert_refusal(call(json!({"op":"dispatch","state":state,"command":command,"now":BEFORE})),"/reviewed_packets/admission_before_review");
    let unchanged = good(json!({"op":"validate_state","state":state,"now":AFTER}));
    assert_eq!(unchanged,state);
    let admitted = good(json!({"op":"dispatch","state":state,"command":command,"now":POLICY}));
    assert_eq!(admitted["state"]["receipts"].as_array().unwrap().last().unwrap()["committed_at"], POLICY);
    assert_eq!(admitted["state"]["reviewed_packets"].as_array().unwrap().len(),1);
    assert_eq!(good(json!({"op":"validate_state","state":admitted["state"],"now":AFTER})),admitted["state"]);
}

#[test]
fn re_signed_imported_admission_cannot_claim_to_precede_its_compiled_review() {
    let initial = legacy(); let command = envelope(&initial);
    let mut state = good(json!({"op":"dispatch","state":initial,"command":command,"now":AFTER}))["state"].clone();
    good(json!({"op":"validate_state","state":state,"now":AFTER}));
    let record = state["receipts"].as_array_mut().unwrap().last_mut().unwrap();
    record["committed_at"] = json!(BEFORE);
    // Public digests are recomputed only in this hostile fixture. The source
    // packet, catalogue pin and original command/response remain exact.
    let command: AppCommandV1 = parse_document(&canonical_bytes(&record["command"]).unwrap()).unwrap();
    let response: Snapshot = parse_document(&canonical_bytes(&record["response"]).unwrap()).unwrap();
    record["command_digest"] = json!(canonical_digest(&command).unwrap());
    record["response_digest"] = json!(canonical_digest(&response).unwrap());
    assert_refusal(call(json!({"op":"validate_state","state":state,"now":AFTER})),"/receipts/admission_before_review");
    let cold = std::thread::spawn(move||call(json!({"op":"validate_state","state":state,"now":AFTER}))).join().unwrap();
    assert_refusal(cold,"/receipts/admission_before_review");
}

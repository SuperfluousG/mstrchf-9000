//! Independent admission order and chronological custody through run_json.
//! Re-signed journals below are hostile fixtures, never an authority mechanism.
use mstrchf_browser::run_json;
use mstrchf_contracts::*;
use serde_json::{Value, json};

const BEFORE: &str = "2026-10-13T05:20:47Z";
const BOUNDARY: &str = "2026-10-13T05:20:48Z";
const AFTER: &str = "2026-10-13T06:00:00Z";
fn extra() -> Value { json!({"recipe":{"recipe_id":"rc_80000000000000000000000000000009","revision":"1","digest":"ef0cbaba1c1698ccaaa8de4b0f6ee4526d33150645532a3ca822f65ba9cf97c9"},"packet_digest":"e9675ef1aec183cf4d540b5c2c23856a1666f02bcf6b57080e8b845f1551209b"}) }
fn base() -> Value { json!({"recipe":{"recipe_id":"rc_80000000000000000000000000000008","revision":"1","digest":"fa50f509872b19c8b986dff36ec91397e5268a60afbb93a7728badfde8661a71"},"packet_digest":"a6a189f045a8fe21617efb5fed082a43773071c6250c2b06bdda943e32395407"}) }
fn initial() -> Value { serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone() }
fn call(request: Value) -> Value { serde_json::from_slice(&run_json(&serde_json::to_vec(&request).unwrap())).unwrap() }
fn good(request: Value) -> Value { let out=call(request); assert_eq!(out["ok"],true,"{out}"); out["value"].clone() }
fn envelope(state:&Value, packet:Value, revision:&str, id:u32)->Value {
    json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":{"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":revision}}})
}
fn dispatch(state:&Value, command:&Value, now:&str)->Value { call(json!({"op":"dispatch","state":state,"command":command,"command_json":serde_json::to_string(command).unwrap(),"now":now})) }
fn admitted(state:&Value, command:&Value, now:&str)->Value { let out=dispatch(state,command,now); assert_eq!(out["ok"],true,"{out}"); out["value"]["state"].clone() }
fn assert_error(out:Value,code:&str,path:&str) { assert_eq!(out["ok"],false,"{out}"); assert_eq!(out["error"]["code"],code,"{out}"); assert_eq!(out["error"]["issues"][0]["path"],path,"{out}"); }
fn cold(state:Value)->Value { std::thread::spawn(move||good(json!({"op":"validate_state","state":state,"now":AFTER}))).join().unwrap() }
fn pair()->(Value,Value,Value,usize) {
    let before=initial(); let index=before["receipts"].as_array().unwrap().len();
    let extra_command=envelope(&before,extra(),"1",8100); let with_extra=admitted(&before,&extra_command,BOUNDARY);
    let base_command=envelope(&with_extra,base(),"2",8101); let state=admitted(&with_extra,&base_command,AFTER);
    (state,extra_command,base_command,index)
}
fn rehash(receipt:&mut Value) {
    let command:AppCommandV1=parse_document(&canonical_bytes(&receipt["command"]).unwrap()).unwrap();
    let response:Snapshot=parse_document(&canonical_bytes(&receipt["response"]).unwrap()).unwrap();
    receipt["command_digest"]=json!(canonical_digest(&command).unwrap().0);
    receipt["response_digest"]=json!(canonical_digest(&response).unwrap().0);
}

#[test]
fn base_admission_requires_prior_independent_extra_even_before_any_meal_selection() {
    let state=initial(); let command=envelope(&state,base(),"1",8000);
    assert_error(dispatch(&state,&command,AFTER),"missing_reference","/catalogue/extra_dependency");
    assert_eq!(good(json!({"op":"validate_state","state":state,"now":AFTER})),state);
    assert_eq!(cold(state.clone()),state);
}

#[test]
fn extra_review_clock_is_inclusive_and_independent_pair_admissions_preserve_original_food_and_replay() {
    let before=initial(); let too_early=envelope(&before,extra(),"1",8099);
    assert_error(dispatch(&before,&too_early,BEFORE),"authority_denied","/reviewed_packets/admission_before_review");
    let (state,extra_command,base_command,index)=pair();
    assert_eq!(state["reviewed_packets"].as_array().unwrap().len(),2);
    assert_eq!(state["reviewed_packets"][0]["packet"]["recipe_reference"],extra()["recipe"]);
    assert_eq!(state["reviewed_packets"][1]["packet"]["recipe_reference"],base()["recipe"]);
    assert_eq!(state["snapshot"]["weeks"],before["snapshot"]["weeks"]);
    assert_eq!(state["snapshot"]["profile"],before["snapshot"]["profile"]);
    assert_eq!(state["snapshot"]["privacy"],before["snapshot"]["privacy"]);
    assert_eq!(state["history"],before["history"]);
    for n in 0..index { assert_eq!(state["receipts"][n],before["receipts"][n]); }
    assert_eq!(state["receipts"][index]["committed_at"],BOUNDARY);
    assert_eq!(state["receipts"][index]["food_catalogue_pin"]["revision"],"2");
    assert_eq!(state["receipts"][index+1]["food_catalogue_pin"]["revision"],"3");
    assert_eq!(cold(state.clone()),state);
    for command in [extra_command,base_command] {
        let replay=dispatch(&state,&command,AFTER); assert_eq!(replay["ok"],true,"{replay}");
        assert_eq!(replay["value"]["state"],state); assert_eq!(replay["value"]["receipt"]["disposition"],"replayed");
    }
}

#[test]
fn re_signed_journal_cannot_move_base_before_dependency_or_extra_before_review() {
    let (state,_,_,index)=pair();
    let mut reversed=state.clone();
    // Keep revisions/IDs/responses plausible; change only the two exact packet
    // intents and recompute public digests. Dependency admission must reject
    // before later catalogue-pin or final-record ordering checks could do so.
    reversed["receipts"][index]["command"]["command"]["data"]["packet"]=base();
    reversed["receipts"][index+1]["command"]["command"]["data"]["packet"]=extra();
    rehash(&mut reversed["receipts"][index]); rehash(&mut reversed["receipts"][index+1]);
    assert_error(call(json!({"op":"validate_state","state":reversed,"now":AFTER})),"missing_reference","/catalogue/extra_dependency");
    let cold_reversed=std::thread::spawn(move||call(json!({"op":"validate_state","state":reversed,"now":AFTER}))).join().unwrap();
    assert_error(cold_reversed,"missing_reference","/catalogue/extra_dependency");
    let mut pre_review=state;
    pre_review["receipts"][index]["committed_at"]=json!(BEFORE); rehash(&mut pre_review["receipts"][index]);
    assert_error(call(json!({"op":"validate_state","state":pre_review,"now":AFTER})),"authority_denied","/receipts/admission_before_review");
}

use std::{env, fs, path::PathBuf};

fn main() {
    let source = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../store/src");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    // The immutable engineering-v1 asset keeps its literal hash. Adapt only
    // its Rust constructors to the optional field; legacy projections omit it.
    let legacy_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("assets/catalogue/engineering-v1/food-premises-v1.rs");
    let legacy = fs::read_to_string(&legacy_path).unwrap();
    assert_eq!(legacy.matches("MethodPhases{").count(), 2);
    fs::write(out.join("legacy_food_compat.rs"),
        legacy.replace("MethodPhases{", "MethodPhases{portable_cold:None,")).unwrap();
    println!("cargo:rerun-if-changed={}", legacy_path.display());
    // Compile the existing pure implementation; do not fork its arithmetic or rules.
    let development = fs::read_to_string(source.join("development.rs")).unwrap();
    let start = development.find("use mstrchf_contracts::*;").unwrap();
    let end = development.find("fn food_error(").unwrap();
    let mut pure = development[start..end].to_owned();
    // Browser food premises are owned by its immutable compiled view. Remove
    // the mutable native recipe/evidence builders, preserving pure context.
    let native_catalogue_start = pure.find("pub fn catalogue()").unwrap();
    let native_catalogue_end = pure[native_catalogue_start..].find("pub(crate) fn context(").unwrap() + native_catalogue_start;
    pure.replace_range(native_catalogue_start..native_catalogue_end, "");
    fs::write(out.join("development_shared.rs"), pure).unwrap();
    // Parameterise that same implementation with an explicit immutable runtime
    // view. Arithmetic, scheduling, identities and ranking still use the core.
    let mut view = fs::read_to_string(source.join("planner.rs")).unwrap();
    let start = view.find("pub fn catalogue()->").unwrap();
    let end = view[start..].find("pub(crate) fn retained_conflicts(").unwrap() + start;
    view.replace_range(start..end, "");
    // Browser-only owner input: the legacy native adapter has no declaration UI.
    view = view.replace("pub struct ChoiceOptions {", "pub struct ChoiceOptions {pub portable_cold:Option<crate::PortableColdOptions>,");
    view = view.replace("impl Default for ChoiceOptions {fn default()->Self{Self{", "impl Default for ChoiceOptions {fn default()->Self{Self{portable_cold:None,");
    let saved_choice = "assess_choice(&c,&selected.slot,&choice,selected.origin,selected.repeat_authorised,&reviewed)";
    assert!(view.contains(saved_choice));
    view = view.replace(saved_choice, "crate::portable::assess_saved_choice(view,&c,&selected.slot,&choice,selected.origin,selected.repeat_authorised,&reviewed)");
    let manual_choice = "assess_choice(&c,slot,&spec,SelectionOrigin::Fixed,false,&reviewed)";
    assert!(view.contains(manual_choice));
    view = view.replace(manual_choice, "crate::portable::assess_manual_choice(view,&c,slot,&spec,&options,&reviewed)");
    let phase_anchor = "let end=selected.meal_overrides.eat_by.0.as_ref().or(meal.eat_by.0.as_ref()).ok_or_else(||error(ErrorCode::MissingReference,\"/eat_by\"))?;";
    assert!(view.contains(phase_anchor));
    view = view.replace(phase_anchor, "let end=crate::portable::phase_anchor(view,selected,&phase,meal)?;");
    let suggested_patch = "mstrchf_core::selection::prepare_selection_patch(&c,&intent,&proposal,&reviewed,&[])";
    assert!(view.contains(suggested_patch));
    view = view.replace(suggested_patch, "mstrchf_core::selection::prepare_selection_patch_with_food(&c,&intent,&proposal,&reviewed,&[],&view.food_evidence(&c.stamp.recipe_refs.0)?)");
    let suggestion_target = "    for slot in targets {";
    assert!(view.contains(suggestion_target));
    view = view.replace(suggestion_target, "    for slot in targets {\n        if crate::portable::owner_bound_slot(&candidate.plan,&slot) {continue;}");
    view = view.replace("pub(crate) fn retained_conflicts(p:", "pub fn retained_conflicts(view:&crate::catalogue::RuntimeCatalogue,p:");
    view = view.replace("pub(crate) fn derive(s:", "pub fn derive(view:&crate::catalogue::RuntimeCatalogue,s:");
    view = view.replace("pub(crate) fn choices(s:", "pub fn choices(view:&crate::catalogue::RuntimeCatalogue,s:");
    view = view.replace("pub(crate) fn custom_choices(s:", "pub fn custom_choices(view:&crate::catalogue::RuntimeCatalogue,s:");
    view = view.replace("pub(crate) fn suggest(s:", "pub fn suggest(view:&crate::catalogue::RuntimeCatalogue,s:");
    let start = view.find("pub(crate) fn context(").unwrap();
    let end = view[start..].find("\npub fn derive(").unwrap() + start;
    view.replace_range(start..end,
        "pub fn context(view:&crate::catalogue::RuntimeCatalogue,s:&Snapshot,p:&HouseholdProfile,w:&Week)->Result<PlanningContext,DomainError>{let mut c=crate::development::context(s,p,w,view.recipes().to_vec())?;c.stamp.catalogue_revision=view.pin().revision;Ok(c)}\n");
    view = view.replace("let reviewed=reviewed()?;", "let reviewed=view.reviewed()?;");
    view = view.replace("let recipes=reviewed()?;", "let recipes=view.reviewed()?;");
    view = view.replace("    let inputs=&material()?.1;\n", "");
    view = view.replace("context(s,p,w,catalogue()?)", "context(view,s,p,w)");
    view = view.replace("context(s,p,&candidate,catalogue()?)", "context(view,s,p,&candidate)");
    view = view.replacen("    let c=context(view,s,p,w)?;", "    let c=context(view,s,p,w)?;\n    check_context_scope(view,&c,w)?;", 1);
    view = view.replace("{custom_choices(s,p,w,slot,", "{custom_choices(view,s,p,w,slot,");
    view = view.replace("choices(s,p,&candidate,", "choices(view,s,p,&candidate,");
    view = view.replace("derive(s,p,", "derive(view,s,p,");
    let start = view.find("    let mut evidence=FoodEvidence{").unwrap();
    let end = view[start..].find("    let phases=").unwrap() + start;
    view.replace_range(start..end, "    let evidence=view.food_evidence(&refs)?;\n");
    // Existing cold-home phases retain their advance session. The new original
    // source keeps all cooking together and routes only named cooling phases to
    // its separate advance session, including a cooking start earlier than T-60.
    let phase_role = "role:if prep.latest_offset_seconds < -3600 {SessionRole::AdvancePrep}else{SessionRole::Cooking}";
    assert!(view.contains(phase_role));
    view = view.replace(phase_role, "role:phase_session_role(r,&phase.occurrence.recipe,prep)?");
    view = view.replace("    if let Some(time)=options.eat_by_time", "    let scope_options=actual_board_options(&options);\n    if let Some(time)=options.eat_by_time.as_ref()");
    // Choice assessment may inspect only the requested retained two-unit target
    // without its departed guest. The final complete candidate remains strict.
    let choice_context="    options.overrides.validate_shape().map_err(|_|error(ErrorCode::InvalidValue,\"/choice/overrides\"))?;\n    let c=context(view,s,p,w)?;";
    assert!(view.contains(choice_context));
    view=view.replace(choice_context,"    options.overrides.validate_shape().map_err(|_|error(ErrorCode::InvalidValue,\"/choice/overrides\"))?;\n    let c=choice_context(view,s,p,w,slot)?;");
    view = view.replace("        let rf=crate::development::recipe_reference(r)?;", "        let rf=crate::development::recipe_reference(r)?;\n        if !view.offers_new_choice(&rf)? {continue;}");
    // Only the exact original-component service families get this path. Old
    // source matching, link material and choices retain their existing bytes.
    let source_choice = "        let source=options.leftover_source.as_ref().map(|source|w.plan.selections.iter().find(|v|&v.slot==source&&v.recipe==rf&&matches!(v.batch_use,BatchUse::NewBatch{..}))).flatten();";
    assert!(view.contains(source_choice));
    view = view.replace(source_choice, "        if is_original_component_service(&rf) && options.leftover_source.is_none(){continue;}\n        let source=choice_source(r,&rf,w,slot,&options)?;");
    let source_components = "component_ids:BoundedVec(r.components.iter().map(|v|v.component_id.clone()).collect())";
    assert!(view.contains(source_components));
    view = view.replace(source_components, "component_ids:choice_component_ids(r)?");
    let link_components = "components:BoundedVec(scaled.components.iter().map(|v|ComponentAllocation{component_id:v.component_id.clone(),quantity:v.quantity.clone()}).collect())";
    assert!(view.contains(link_components));
    view = view.replace(link_components, "components:choice_link_components(r,&scaled)?");
    view = view.replace("v.recipe().recipe_id==r.recipe_id", "crate::development::recipe_reference(v.recipe()).as_ref()==Ok(&rf)");
    view = view.replace("if derive(view,s,p,&candidate).is_ok()", "if context(view,s,p,&candidate).is_ok_and(|actual|choice_scope(view,&actual,&candidate,&selection,&scope_options).is_ok()) && derive(view,s,p,&candidate).is_ok()");
    view = view.replace("catalogue_revision:Revision(1),categories", "catalogue_revision:view.pin().revision,categories");
    view = view.replace("    for selected in &w.plan.selections.0 {", "    for selected in &w.plan.selections.0 {\n        if view.is_personal_recipe(&selected.recipe) || view.is_public_recipe(&selected.recipe) { if let Err(e)=retained_personal_scope(view,p,w,selected) {conflicts.push(Conflict{code:e.code,week_ids:BoundedVec(vec![w.range.week_id.clone()]),slot_ids:BoundedVec(vec![selected.slot.clone()]),recipe_refs:BoundedVec(vec![selected.recipe.clone()]),evidence_ids:BoundedVec(vec![]),blocking:true});} }\n");
    assert!(!view.contains("catalogue()?"));
    assert!(!view.contains("reviewed()?;" ) || view.contains("view.reviewed()?;"));
    // Retain the same pure food inputs for later approval/calendar custody.
    // The ordinary board still consumes only the unchanged FoodDerivation.
    let derive_signature = "pub fn derive(view:&crate::catalogue::RuntimeCatalogue,s:&Snapshot,p:&HouseholdProfile,w:&Week)->Result<Option<FoodDerivation>,DomainError>{";
    assert!(view.contains(derive_signature));
    view = view.replace(derive_signature, "pub fn derive_material(view:&crate::catalogue::RuntimeCatalogue,s:&Snapshot,p:&HouseholdProfile,w:&Week)->Result<Option<(SavedFoodBasis,FoodDerivation)>,DomainError>{");
    let derive_tail = "    derive_plan(&w.plan,&c,&FoodInputs{evidence:&evidence,identities:&identities,schedule:&schedule,feasibility:&feasibility}).map(Some).map_err(food_error)";
    assert!(view.contains(derive_tail));
    view = view.replace(derive_tail, "    let derived=derive_plan(&w.plan,&c,&FoodInputs{evidence:&evidence,identities:&identities,schedule:&schedule,feasibility:&feasibility}).map_err(food_error)?;\n    Ok(Some((SavedFoodBasis{plan:w.plan.clone(),context:c,evidence,identities,schedule},derived)))");
    view.push_str("\npub fn derive(view:&crate::catalogue::RuntimeCatalogue,s:&Snapshot,p:&HouseholdProfile,w:&Week)->Result<Option<FoodDerivation>,DomainError>{derive_material(view,s,p,w).map(|value|value.map(|(_,food)|food))}\n");
    fs::write(out.join("planner_view_shared.rs"), view.replacen("//!", "//", 1)).unwrap();
    println!(
        "cargo:rerun-if-changed={}",
        source.join("development.rs").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        source.join("planner.rs").display()
    );
}

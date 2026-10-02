// Independent assertions; setup helpers copied from reviewed tests and pinned by review SHA.
fn reviewed_classes(r: &RecipeRevision) -> (EvidenceBasis, ComponentClassView) {
    let (basis, mut classes) = inputs(r);
    if r.components.len() == 2 {
        let mut fact = classes.components[0].clone();
        fact.component_id = r.components[1].component_id.clone();
        classes.components.push(fact);
    }
    (basis, classes)
}
fn cooked() -> Quantity {
    Quantity { amount: p(300,1), unit: Unit::G, form: IngredientForm::Cooked }
}
fn dry_result(rr: &ReviewedRecipe<'_>, id: &EvidenceId) -> Result<Quantity, DomainError> {
    convert_quantity(rr, &rr.recipe().ingredients[0].ingredient_id, &cooked(), Unit::G, IngredientForm::Dry, id)
}
#[test]
fn r1_identical_rule_repeated_across_components_remains_unambiguous() {
    let mut r = recipe(); conversion(&mut r); second_component(&mut r);
    assert_eq!(r.ingredients[0].conversion_evidence, r.ingredients[1].conversion_evidence);
    let (b,c) = reviewed_classes(&r);
    let rr = assess_revision(&r,&b,&c).unwrap();
    let result = dry_result(&rr,&eid(9));
    println!("identical repeated evidence: {result:?}");
    assert_eq!(result.unwrap().amount, p(100,1));
}

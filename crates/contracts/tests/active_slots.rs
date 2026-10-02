use mstrchf_contracts::*;
fn week() -> Week {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/contracts/week-disabled-retained.json");
    let mut w: Week = parse_document(&std::fs::read(path).unwrap()).unwrap();
    w.day_enabled = BoundedVec(vec![true; 7]);
    w.meal_type_enabled = MealTypeEnabled {
        breakfast: true,
        lunch: true,
        dinner: true,
    };
    w
}
#[test]
fn empty_plan_slots_have_explicit_chronological_meal_order() {
    let mut w = week();
    w.plan.selections.0.clear();
    let before = w.clone();
    let slots = active_slots(&w).unwrap();
    assert_eq!(slots.len(), 21);
    for (day, group) in slots.chunks_exact(3).enumerate() {
        assert_eq!(
            group.iter().map(|s| s.meal_type).collect::<Vec<_>>(),
            vec![MealType::Breakfast, MealType::Lunch, MealType::Dinner]
        );
        assert!(group.iter().all(|s| s.date == group[0].date));
        assert_eq!(
            group[0].date.0,
            [
                "2026-12-28",
                "2026-12-29",
                "2026-12-30",
                "2026-12-31",
                "2027-01-01",
                "2027-01-02",
                "2027-01-03"
            ][day]
        );
    }
    w.cells.0.reverse();
    assert_eq!(active_slots(&w).unwrap(), slots);
    w.cells.0.rotate_left(5);
    assert_eq!(active_slots(&w).unwrap(), slots);
    assert_eq!(before.plan, w.plan);
}
#[test]
fn every_exclusion_combination_and_order_preserves_occupied_projection() {
    for meal in [MealType::Breakfast, MealType::Lunch, MealType::Dinner] {
        for flags in 0..8 {
            for reverse in [false, true] {
                let mut w = week();
                let original_plan = w.plan.clone();
                let target = w
                    .cells
                    .iter()
                    .find(|c| c.slot.date == w.range.monday && c.slot.meal_type == meal)
                    .unwrap()
                    .slot
                    .clone();
                let day = flags & 1 == 0;
                let row = flags & 2 == 0;
                let skip = flags & 4 != 0;
                for operation in if reverse { [2, 1, 0] } else { [0, 1, 2] } {
                    match operation {
                        0 => w.day_enabled.0[0] = day,
                        1 => match meal {
                            MealType::Breakfast => w.meal_type_enabled.breakfast = row,
                            MealType::Lunch => w.meal_type_enabled.lunch = row,
                            MealType::Dinner => w.meal_type_enabled.dinner = row,
                        },
                        _ => {
                            w.cells
                                .0
                                .iter_mut()
                                .find(|c| c.slot == target)
                                .unwrap()
                                .choice = if skip {
                                MealLocation::Skip
                            } else {
                                MealLocation::AtHome
                            }
                        }
                    }
                }
                let slots = active_slots(&w).unwrap();
                assert_eq!(slots.contains(&target), day && row && !skip);
                let occupied: Vec<_> = w
                    .plan
                    .selections
                    .iter()
                    .filter(|s| slots.contains(&s.slot))
                    .collect();
                assert_eq!(active_selections(&w), occupied);
                assert_eq!(w.plan, original_plan);
            }
        }
    }
}
#[test]
fn all_excluded_and_reenabled_preserve_retained_plan() {
    let mut w = week();
    let original = w.clone();
    for mode in 0..3 {
        w = original.clone();
        match mode {
            0 => w.day_enabled = BoundedVec(vec![false; 7]),
            1 => {
                w.meal_type_enabled = MealTypeEnabled {
                    breakfast: false,
                    lunch: false,
                    dinner: false,
                }
            }
            _ => w
                .cells
                .0
                .iter_mut()
                .for_each(|c| c.choice = MealLocation::Skip),
        }
        assert!(active_slots(&w).unwrap().is_empty());
        assert!(active_selections(&w).is_empty());
        assert_eq!(w.plan, original.plan);
    }
    assert_eq!(active_slots(&original).unwrap().len(), 21);
}
#[test]
fn malformed_public_weeks_refuse_before_activity_indexing() {
    for mode in 0..7 {
        let mut w = week();
        match mode {
            0 => w.day_enabled.0.clear(),
            1 => w.range.monday.0 = "bad".into(),
            2 => w.cells.0[0].slot.date.0 = "2026-02-30".into(),
            3 => w.cells.0.pop().map(|_| ()).unwrap(),
            4 => w.cells.0[1] = w.cells[0].clone(),
            5 => w.plan.revision = Revision(0),
            _ => {
                w.plan.selections.0[0].servings.0[0]
                    .servings
                    .0
                    .denominator
                    .0 = 0
            }
        }
        assert!(active_slots(&w).is_err(), "mode={mode}");
    }
}

#[test]
fn plan_permutation_cannot_change_slot_order_or_hide_unoccupied_cells() {
    let mut w = week();
    let dinner = w.plan.selections[0].clone();
    for meal in [MealType::Lunch, MealType::Breakfast] {
        let mut selection = dinner.clone();
        selection.slot.meal_type = meal;
        for serving in &mut selection.servings.0 {
            serving.preset_ref.meal_type = meal;
        }
        w.plan.selections.0.push(selection);
    }
    let slots = active_slots(&w).unwrap();
    assert_eq!(slots.len(), 21);
    assert_eq!(active_selections(&w).len(), 3);
    w.plan.selections.0.reverse();
    w.cells.0.reverse();
    assert_eq!(active_slots(&w).unwrap(), slots);
    assert_eq!(
        active_selections(&w),
        w.plan.selections.iter().collect::<Vec<_>>()
    );
    w.meal_type_enabled.lunch = false;
    let slots = active_slots(&w).unwrap();
    assert_eq!(slots.len(), 14);
    assert_eq!(
        active_selections(&w),
        w.plan
            .selections
            .iter()
            .filter(|s| slots.contains(&s.slot))
            .collect::<Vec<_>>()
    );
}

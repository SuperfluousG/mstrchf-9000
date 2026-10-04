import type { HouseholdDraft, Snapshot } from '../../generated/contracts';
export type SyntheticExample = 'empty' | 'quick-meals';
// Development entry only. Existing native-owned household identity/revision are preserved.
export function syntheticDraft(example:SyntheticExample,saved:Snapshot):HouseholdDraft {
  const draft = structuredClone(saved.household_draft);
  const stages: HouseholdDraft['completed_stages'] = draft.completed_stages.filter(stage => stage !== 'food_preferences');
  if (example === 'quick-meals') stages.push('food_preferences');
  return {
    ...draft, completed_stages:stages.sort(),
    preferences:example==='empty'?null:{
      spending_style:'balanced',higher_protein:false,more_variety:false,quick_meals:true,
      whole_food_preference:false,dislikes:[],equipment:['hob'],preferred_hands_on_minutes:20,
      meal_times:[{meal_type:'breakfast',time:'08:00:00'}, {meal_type:'dinner',time:'18:30:00'},
        {meal_type:'lunch',time:'12:30:00'}],mobility:{reheating:false,cold_storage:false,temperature:'either'},
    },
  };
}

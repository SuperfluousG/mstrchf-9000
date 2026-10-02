import type { HouseholdDraft, Snapshot } from '../../generated/contracts';
export type SyntheticExample = 'empty' | 'quick-meals';
// Development entry only. Existing native-owned household identity/revision are preserved.
export function syntheticDraft(example:SyntheticExample,saved:Snapshot):HouseholdDraft {
  return {
    ...saved.household_draft, completed_stages:example==='empty'?[]:['food_preferences'], diners:[], usual_week:null, connections:null,
    preferences:example==='empty'?null:{
      spending_style:'balanced',higher_protein:false,more_variety:false,quick_meals:true,
      whole_food_preference:false,dislikes:[],equipment:['hob'],preferred_hands_on_minutes:20,
      meal_times:[{meal_type:'breakfast',time:'08:00:00'}, {meal_type:'dinner',time:'18:30:00'},
        {meal_type:'lunch',time:'12:30:00'}],mobility:{reheating:false,cold_storage:false,temperature:'either'},
    },
  };
}

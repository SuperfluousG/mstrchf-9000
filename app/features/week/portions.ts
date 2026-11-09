import type {MealType,PersonPortionOverride,PersonRef,PortionPreference,PortionPresetName,PositiveRational} from '../../generated/contracts';

export const MAX_PORTION_NUMERATOR=1_000_000_000_000n;
export const MAX_PORTION_DENOMINATOR=1_000_000n;
export const PORTION_INPUT_HELP='Use a positive decimal or fraction, for example 1.25 or 5/4. This multiplies the reviewed preset; support is checked by the recipe.';
const gcd=(a:bigint,b:bigint):bigint=>{while(b){[a,b]=[b,a%b];}return a;};
export function parsePortionAdjustment(input:string):PositiveRational {
 const text=input.trim();
 if(!text||text.length>80)throw new Error('Enter a positive decimal or fraction (up to 80 characters).');
 let n:bigint,d:bigint;
 const fraction=/^(\d+)\s*\/\s*(\d+)$/.exec(text);
 if(fraction){n=BigInt(fraction[1]!);d=BigInt(fraction[2]!);}
 else if(/^(?:\d+(?:\.\d*)?|\.\d+)$/.test(text)){
  const [whole='',part='']=text.split('.');n=BigInt((whole||'0')+part);d=10n**BigInt(part.length);
 }else throw new Error('Enter a positive decimal or fraction; signs, exponents and commas are not supported.');
 if(n<=0n||d<=0n)throw new Error('The multiplier and fraction denominator must be greater than zero.');
 const divisor=gcd(n,d);n/=divisor;d/=divisor;
 if(n>MAX_PORTION_NUMERATOR||d>MAX_PORTION_DENOMINATOR)throw new Error('The reduced multiplier must have numerator at most 1000000000000 and denominator at most 1000000.');
 return {numerator:n.toString(),denominator:d.toString()};
}
export function portionInputError(input:string):string|null {try{parsePortionAdjustment(input);return null;}catch(error){return (error as Error).message;}}
export const adjustmentText=(value:PositiveRational)=>value.denominator==='1'?value.numerator:`${value.numerator}/${value.denominator}`;
export function effectivePortion(preference:PortionPreference,meal:MealType):{preset:PortionPresetName;adjustment:PositiveRational}{
 const override=preference.meal_overrides.find(value=>value.meal_type===meal);
 return override??preference;
}
/** Core precedence replaces the effective preset/adjustment, rather than
 * multiplying a saved adjustment twice. Blank preview fields add no override. */
export function previewPortionOverride(person:PersonRef,preference:PortionPreference,meal:MealType,preset:PortionPresetName|'',input:string):PersonPortionOverride|null {
 if(!preset&&!input.trim())return null;
 return {person,preset:preset||effectivePortion(preference,meal).preset,adjustment:parsePortionAdjustment(input.trim()||'1')};
}
export const portionContextStamp=(value:unknown)=>JSON.stringify(value,(_key,item)=>item&&typeof item==='object'&&!Array.isArray(item)?Object.fromEntries(Object.keys(item).sort().map(key=>[key,item[key]])):item);

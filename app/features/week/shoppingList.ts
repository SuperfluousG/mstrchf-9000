import type {CataloguePin} from './shoppingListTypes';
import type {DemandLine,EssentialsConfirmation,RecipeRef,SlotId,WeekRange} from '../../generated/contracts';
import {rational,human} from './plannerTypes';

export type ShoppingListOrigin={recipe:RecipeRef;title:string;admission:'personal_reviewed'|'public_reviewed'|'engineering';batch_id:string|null;extra_id:string|null;source_slot:SlotId;target_slots:SlotId[]};
export type ShoppingListLine={demand:DemandLine;name:string;action:'have_enough'|'add_to_shop';confirmation_status:'current'|'stale'|'none'|'not_offered';confirmation:EssentialsConfirmation|null;origins:ShoppingListOrigin[]};
export type ShoppingListProjection={format:'mstrchf-manual-shopping-list';version:1;snapshot_revision:string;week_id:string;week_range:WeekRange;week_revision:string;catalogue_pin:CataloguePin;derived_digest:string;demand_digest:string;evaluated_at:string;open_slots:SlotId[];contains_engineering:boolean;retailer_status:'unqualified';price_status:'unavailable';lines:ShoppingListLine[]};

export const quantityText=(line:ShoppingListLine)=>`${rational(line.demand.quantity.amount)} ${line.demand.quantity.unit} · ${human(line.demand.quantity.form)}`;
export const originText=(origin:ShoppingListOrigin)=>`${origin.title} · ${origin.source_slot.date} ${origin.source_slot.meal_type}${origin.extra_id?' · separate extra':''}${origin.admission==='engineering'?' · SYNTHETIC ENGINEERING FIXTURE':''} · used for ${origin.target_slots.map(slot=>`${slot.date} ${slot.meal_type}`).join(', ')}`;

export function shoppingListText(list:ShoppingListProjection):string {
 const lines=[`MasterChef 9000 — manual shopping list`,`Week beginning ${list.week_range.monday} · ${list.week_range.zone}`,
  `Saved snapshot ${list.snapshot_revision} · week revision ${list.week_revision} · generated ${list.evaluated_at}`,
  list.open_slots.length?`${list.open_slots.length} active meals remain open. Quantities cover saved active meals only.`:'All active meals have saved selections.',
  'Retailer stock, product suitability, packs, prices and delivery are unqualified. This is not basket or order confirmation.',
  'Ingredient forms describe recipe inputs, not retailer product specifications. Raw milk input form does not mean unpasteurised milk.'];
 if(list.contains_engineering)lines.push('Includes synthetic engineering ingredients. This list is not a qualified production shop.');
 const render=(line:ShoppingListLine)=>{
  lines.push(`${line.name} · ${quantityText(line)}`);
  if(line.confirmation_status==='stale')lines.push('  Previous pantry confirmation is stale; included in shopping again.');
  if(line.confirmation_status==='not_offered')lines.push('  Pantry deduction is not supported for this ingredient; the full amount is included.');
  for(const origin of line.origins)lines.push(`  ${originText(origin)}`);
 };
 lines.push('','Add to shop');for(const line of list.lines.filter(line=>line.action==='add_to_shop'))render(line);
 if(!list.lines.some(line=>line.action==='add_to_shop'))lines.push('No quantities to add for the saved active selections.');
 const covered=list.lines.filter(line=>line.action==='have_enough');
 if(covered.length){lines.push('','Have enough — explicitly confirmed for these exact amounts');for(const line of covered)render(line);}
 lines.push('',`Demand digest: ${list.demand_digest}`,`Derived digest: ${list.derived_digest}`,`Catalogue manifest: ${list.catalogue_pin.manifest_digest}`);
 return lines.join('\n')+'\n';
}

export function exportShoppingList(list:ShoppingListProjection,format:'text'|'json') {
 const body=format==='text'?shoppingListText(list):JSON.stringify(list,null,2)+'\n';
 const url=URL.createObjectURL(new Blob([body],{type:format==='text'?'text/plain;charset=utf-8':'application/json'}));
 const link=document.createElement('a');link.href=url;link.download=`MasterChef-shopping-list-${list.week_range.monday}.${format==='text'?'txt':'json'}`;link.click();
 setTimeout(()=>URL.revokeObjectURL(url),1000);
}

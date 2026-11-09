// Pure review presentation. Owner-supplied native demand is read, never changed.
// Declared source facts/mappings and public hashes create no matching, retailer,
// account, human approval, MatchBasis, Intent, grant or execution authority.
import {digest} from '../basket/protocol.ts';
import {keys,count,decimal,instant,quantity,priceTerms,money,minimumPacks,surplus,packQuote,addMoney,fail,
  type Quantity,type PriceTerms,type Money,type Rational} from '../basket/exact.ts';

export type Json=null|boolean|number|string|Json[]|{[key:string]:Json};
export type DraftFact<T>={kind:'known';value:T}|{kind:'unknown';reason:string}|{kind:'ambiguous';values:T[]};
// context_key is the opaque actual owner namespace/viewKey, never an email or
// retailer address. Counters alone may collide after identical backup imports.
export type OwnerBinding={context_key:string;generation:string|null;lifecycle_epoch:number;privacy_revision:string};
export type NativeDemandLine={line_id:string;ingredient_id:string;quantity:Quantity;origin_batches:string[];
  origin_extras:string[];essential:boolean;conversion_evidence_ids:string[]};
export type NativeShoppingList={format:'mstrchf-manual-shopping-list';version:1;snapshot_revision:string;
  week_id:string;week_range:Json;week_revision:string;catalogue_pin:Json;derived_digest:string;demand_digest:string;
  evaluated_at:string;open_slots:Json[];contains_engineering:boolean;retailer_status:string;price_status:string;
  lines:{demand:NativeDemandLine;name:string;action:'add_to_shop'|'have_enough';confirmation_status:string;
    confirmation:Json;origins:Json[]}[]};
export type RetainedProductObservation={observation_id:string;
  product:{retailer:string;product_id:string;variant_id:string|null};
  source:{url:string;title:string;ingredients:DraftFact<string>;allergens:DraftFact<string>;storage:DraftFact<string>;
    observed_at:string;expires_at:string|null;provenance_text:string};
  // Explicit as-observed standard base price. A member-only/offer price cannot
  // populate this fact; unknown remains unknown even when an offer is known.
  pack:DraftFact<Quantity>;standard_unit_price:DraftFact<Money>;price_terms:DraftFact<PriceTerms>};
// A serialisable declaration for a conditional preview, not review/admission
// authority. Root calls capturePreviewMapping only after its explicit review
// interaction. Restoring/importing a copy never creates a live approval.
export type PreviewMapping={purpose:'conditional_pack_preview';demand_binding:string;observation_binding:string;
  observation_id:string;demand_line_id:string;ingredient_id:string;reviewed_at:string};
export type ConditionalQuote={status:'conditional_arithmetic';required:Quantity;supply:Quantity;
  whole_packs:number;existing_credit:0;surplus:Rational;pack_spend:Money;reusable_for_intent:false};
export type HistoricalComparison={status:'as_observed_standard_base_only';observed_at:string;currentness:'unknown';
  required:Quantity;supply:Quantity;whole_packs:number;existing_credit:0;surplus:Rational;pack_spend:Money;
  offers_applied:false;reusable_for_intent:false};
export type ReviewIssue='mapping_missing'|'mapping_stale'|'mapping_in_future'|'mapping_predates_source'|'ingredients_unknown'|'ingredients_ambiguous'
  |'pack_unknown'|'pack_ambiguous'|'pack_measure_mismatch'|'terms_unknown'|'terms_ambiguous'|'terms_not_current'
  |'terms_exceed_source_window'|'standard_price_unknown'|'standard_price_ambiguous'|'standard_price_terms_mismatch'
  |'source_expiry_unknown'|'source_not_current'|'engineering_demand'|'line_already_covered'|'quote_unresolved'
  |'currency_mismatch';
export type ProductReviewDraft={format:'mstrchf-product-review-draft';version:1;status:'review_only';
  owner_binding:OwnerBinding;demand_binding:string;native_list:NativeShoppingList;evaluated_at:string;
  products:{observation:RetainedProductObservation;mapping:PreviewMapping|null;source_status:'unqualified';
    meaning_status:'unqualified';native_matching:null;historical_comparison:HistoricalComparison|null;quote:ConditionalQuote|null;issues:ReviewIssue[]}[];
  quoted_pack_subtotal:Money|null;subtotal_scope:'quoted_rows_only';unquoted_line_ids:string[];
  historical_pack_subtotal:Money|null;historical_subtotal_scope:'compared_rows_only';uncompared_line_ids:string[];
  issues:ReviewIssue[];baseline:{status:'unknown';accepted_read_ref:null};account_status:'unknown';
  availability_status:'unknown';product_dates_status:'unknown';delivery_status:'unknown';fees_status:'unknown';
  effect_enabled:false;reusable_for_intent:false;notice:string};

const copy=<T>(value:T):T=>structuredClone(value);
function freeze<T>(value:T):T{if(value&&typeof value==='object'){for(const child of Object.values(value))freeze(child);Object.freeze(value);}return value;}
function text(value:unknown,path:string,max=32_000):string{
  if(typeof value!=='string'||!value.length||value.length>max||/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/u.test(value))fail('invalid_value',path);
  return value as string; // Inert retained text; root must render as text, never HTML.
}
function id(value:unknown,path:string):string{return text(value,path,256);}
function revision(value:unknown,path:string):string{if(decimal(value,path)===0n)fail('invalid_value',path);return value as string;}
function hash(value:unknown,path:string):string{if(typeof value!=='string'||! /^[a-f0-9]{64}$/.test(value))fail('invalid_value',path);return value as string;}
function array(value:unknown,path:string,max:number):unknown[]{if(!Array.isArray(value)||value.length>max)fail('limit_exceeded',path);return value as unknown[];}
function nativeQuantity(raw:unknown,path:string):Quantity{const q=quantity(raw,path);
  if(!['g','ml','each'].includes(q.unit)||BigInt(q.amount.numerator)>1_000_000_000_000n||BigInt(q.amount.denominator)>1_000_000n)fail('unsupported_quantity',path);
  return q;}
function fact<T>(raw:unknown,path:string,check:(value:unknown,path:string)=>T):DraftFact<T>{
  if(!raw||typeof raw!=='object'||Array.isArray(raw))fail('invalid_shape',path);
  const kind=(raw as {kind?:unknown}).kind;
  if(kind==='known'){const v=keys(raw,['kind','value'],path);return {kind:'known',value:check(v.value,path+'/value')};}
  if(kind==='unknown'){const v=keys(raw,['kind','reason'],path);return {kind:'unknown',reason:text(v.reason,path+'/reason',512)};}
  if(kind==='ambiguous'){const v=keys(raw,['kind','values'],path),values=array(v.values,path+'/values',8);if(values.length<2)fail('invalid_value',path);return {kind:'ambiguous',values:values.map((v,i)=>check(v,path+'/values/'+i))};}
  return fail('invalid_value',path+'/kind');
}
function owner(raw:OwnerBinding):OwnerBinding{const v=keys(raw,['context_key','generation','lifecycle_epoch','privacy_revision'],'/owner');
  return {context_key:id(v.context_key,'/owner/context_key'),generation:v.generation===null?null:decimal(v.generation,'/owner/generation').toString(),
    lifecycle_epoch:count(v.lifecycle_epoch,'/owner/lifecycle_epoch',Number.MAX_SAFE_INTEGER),privacy_revision:revision(v.privacy_revision,'/owner/privacy_revision')};}
function native(raw:NativeShoppingList):NativeShoppingList{
  keys(raw,['format','version','snapshot_revision','week_id','week_range','week_revision','catalogue_pin','derived_digest','demand_digest','evaluated_at','open_slots','contains_engineering','retailer_status','price_status','lines'],'/native_list');
  if(raw.format!=='mstrchf-manual-shopping-list'||raw.version!==1||typeof raw.contains_engineering!=='boolean')fail('unsupported_version','/native_list');
  revision(raw.snapshot_revision,'/native_list/snapshot_revision');revision(raw.week_revision,'/native_list/week_revision');
  id(raw.week_id,'/native_list/week_id');hash(raw.derived_digest,'/native_list/derived_digest');hash(raw.demand_digest,'/native_list/demand_digest');instant(raw.evaluated_at,'/native_list/evaluated_at');
  array(raw.open_slots,'/native_list/open_slots',21);text(raw.retailer_status,'/native_list/retailer_status',128);text(raw.price_status,'/native_list/price_status',128);
  const seen=new Set<string>();for(const [i,row] of array(raw.lines,'/native_list/lines',4096).entries()){
    const line=keys(row,['demand','name','action','confirmation_status','confirmation','origins'],'/native_list/lines/'+i);
    const d=keys(line.demand,['line_id','ingredient_id','quantity','origin_batches','origin_extras','essential','conversion_evidence_ids'],'/native_list/demand');
    const lineId=id(d.line_id,'/native_list/line_id');if(seen.has(lineId))fail('duplicate_key','/native_list/line_id');seen.add(lineId);
    id(d.ingredient_id,'/native_list/ingredient_id');nativeQuantity(d.quantity,'/native_list/quantity');
    for(const field of ['origin_batches','origin_extras','conversion_evidence_ids'])for(const value of array(d[field],'/native_list/'+field,512))id(value,'/native_list/'+field);
    if(typeof d.essential!=='boolean'||!['add_to_shop','have_enough'].includes(line.action as string))fail('invalid_value','/native_list/line');
    text(line.name,'/native_list/name',2048);text(line.confirmation_status,'/native_list/confirmation_status',128);array(line.origins,'/native_list/origins',512);
  }
  return copy(raw);
}
function observation(raw:RetainedProductObservation):RetainedProductObservation{
  const v=keys(raw,['observation_id','product','source','pack','standard_unit_price','price_terms'],'/observation');
  const p=keys(v.product,['retailer','product_id','variant_id'],'/observation/product');
  const s=keys(v.source,['url','title','ingredients','allergens','storage','observed_at','expires_at','provenance_text'],'/observation/source');
  const url=text(s.url,'/observation/url',4096);let parsed:URL;try{parsed=new URL(url);}catch{return fail('invalid_value','/observation/url');}
  if(!['http:','https:'].includes(parsed.protocol)||parsed.username||parsed.password)fail('invalid_value','/observation/url');
  const observed_at=instant(s.observed_at,'/observation/observed_at'),expires_at=s.expires_at===null?null:instant(s.expires_at,'/observation/expires_at');
  if(expires_at!==null&&expires_at<=observed_at)fail('invalid_value','/observation/expiry');
  return {observation_id:id(v.observation_id,'/observation/id'),product:{retailer:id(p.retailer,'/observation/retailer'),product_id:id(p.product_id,'/observation/product_id'),variant_id:p.variant_id===null?null:id(p.variant_id,'/observation/variant_id')},
    source:{url,title:text(s.title,'/observation/title',2048),ingredients:fact(s.ingredients,'/observation/ingredients',text),allergens:fact(s.allergens,'/observation/allergens',text),storage:fact(s.storage,'/observation/storage',text),observed_at,expires_at,provenance_text:text(s.provenance_text,'/observation/provenance')},
    pack:fact(v.pack,'/observation/pack',nativeQuantity),standard_unit_price:fact(v.standard_unit_price,'/observation/standard_unit_price',money),price_terms:fact(v.price_terms,'/observation/price_terms',priceTerms)};
}
async function demandBinding(binding:OwnerBinding,list:NativeShoppingList):Promise<string>{
  // Read evaluation time may advance without changing demand. Every substantive
  // native field, owner generation/epoch and privacy revision remains bound.
  const {evaluated_at:_,...material}=list;return digest({owner:binding,native_list:material});
}
function mapping(raw:PreviewMapping):PreviewMapping{keys(raw,['purpose','demand_binding','observation_binding','observation_id','demand_line_id','ingredient_id','reviewed_at'],'/mapping');
  if(raw.purpose!=='conditional_pack_preview')fail('invalid_value','/mapping/purpose');
  hash(raw.demand_binding,'/mapping/demand_binding');hash(raw.observation_binding,'/mapping/observation_binding');
  id(raw.observation_id,'/mapping/observation_id');id(raw.demand_line_id,'/mapping/demand_line_id');id(raw.ingredient_id,'/mapping/ingredient_id');instant(raw.reviewed_at,'/mapping/reviewed_at');return copy(raw);}

// Shape/custody checks for retained notes only. These wrappers attest no source
// truth, semantic meaning, native match, account, approval or external effect.
export function validateProductObservation(raw:unknown):Readonly<RetainedProductObservation>{return freeze(observation(raw as RetainedProductObservation));}
export function validatePreviewMapping(raw:unknown):Readonly<PreviewMapping>{return freeze(mapping(raw as PreviewMapping));}

export async function capturePreviewMapping(binding:OwnerBinding,list:NativeShoppingList,raw:RetainedProductObservation,
  decision:{demand_line_id:string;ingredient_id:string},reviewedAt:string):Promise<Readonly<PreviewMapping>>{
  const current=owner(binding),projection=native(list),product=observation(raw);keys(decision,['demand_line_id','ingredient_id'],'/decision');
  const line=projection.lines.find(line=>line.demand.line_id===decision.demand_line_id);
  if(!line||line.demand.ingredient_id!==decision.ingredient_id)return fail('identity_mismatch','/decision/demand');
  instant(reviewedAt,'/reviewed_at');if(reviewedAt<product.source.observed_at||reviewedAt<projection.evaluated_at)fail('context_stale','/reviewed_at');
  // New declarations retain the existing fresh-read capture restriction. A
  // previously captured declaration may later render as-observed history,
  // but an expired read cannot be freshly reviewed through this API.
  if(product.source.expires_at!==null&&reviewedAt>=product.source.expires_at)fail('source_stale','/reviewed_at');
  return freeze({purpose:'conditional_pack_preview',demand_binding:await demandBinding(current,projection),observation_binding:await digest(product),
    observation_id:product.observation_id,demand_line_id:line.demand.line_id,ingredient_id:line.demand.ingredient_id,reviewed_at:reviewedAt});
}

export async function buildProductReviewDraft(binding:OwnerBinding,list:NativeShoppingList,raw:readonly RetainedProductObservation[],
  reviews:readonly PreviewMapping[],evaluatedAt:string):Promise<Readonly<ProductReviewDraft>>{
  // Root supplies an audited owner projection and must recheck actual owner
  // context/generation/epoch after awaited hashes, before displaying this result.
  // This pure function cannot observe account switching or a concurrent commit.
  const current=owner(binding),projection=native(list),observations=array(raw,'/observations',128).map(value=>observation(value as RetainedProductObservation)),
    mappings=array(reviews,'/mappings',128).map(value=>mapping(value as PreviewMapping));instant(evaluatedAt,'/evaluated_at');
  if(evaluatedAt<projection.evaluated_at)fail('context_stale','/native_list/evaluated_at');
  const demand_binding=await demandBinding(current,projection),seenProducts=new Set<string>(),seenObservations=new Set<string>(),seenMappedLines=new Set<string>(),quotedLines=new Set<string>(),comparedLines=new Set<string>();
  for(const product of observations){const key=JSON.stringify(product.product);if(seenProducts.has(key)||seenObservations.has(product.observation_id))fail('duplicate_key','/observations');seenProducts.add(key);seenObservations.add(product.observation_id);}
  for(const review of mappings){if(!seenObservations.has(review.observation_id))fail('foreign_reference','/mapping/observation_id');if(seenMappedLines.has(review.demand_line_id)||mappings.filter(other=>other.observation_id===review.observation_id).length>1)fail('duplicate_key','/mappings');seenMappedLines.add(review.demand_line_id);}
  const products:ProductReviewDraft['products']=[];
  for(const product of observations){const review=mappings.find(review=>review.observation_id===product.observation_id)??null,issues:ReviewIssue[]=[];let quote:ConditionalQuote|null=null,historical_comparison:HistoricalComparison|null=null;
    const row=review?projection.lines.find(row=>row.demand.line_id===review.demand_line_id):undefined;
    if(!review)issues.push('mapping_missing');
    else if(review.demand_binding!==demand_binding||review.observation_binding!==await digest(product)||!row||row.demand.ingredient_id!==review.ingredient_id)issues.push('mapping_stale');
    if(review&&review.reviewed_at>evaluatedAt)issues.push('mapping_in_future');
    if(review&&review.reviewed_at<product.source.observed_at)issues.push('mapping_predates_source');
    if(projection.contains_engineering)issues.push('engineering_demand');
    if(row?.action==='have_enough')issues.push('line_already_covered');
    if(product.source.ingredients.kind!=='known')issues.push(product.source.ingredients.kind==='unknown'?'ingredients_unknown':'ingredients_ambiguous');
    if(product.pack.kind!=='known')issues.push(product.pack.kind==='unknown'?'pack_unknown':'pack_ambiguous');
    // A loyalty-only/offer price is not an observed standard base price. No
    // fallback from titles, offers or PriceTerms can populate this explicit fact.
    if(product.standard_unit_price.kind!=='known')issues.push(product.standard_unit_price.kind==='unknown'?'standard_price_unknown':'standard_price_ambiguous');
    if(!issues.length&&row&&product.pack.kind==='known'&&product.standard_unit_price.kind==='known'){
      try{const required=row.demand.quantity,supply=product.pack.value,standard=product.standard_unit_price.value,whole_packs=minimumPacks(required,supply);
        historical_comparison={status:'as_observed_standard_base_only',observed_at:product.source.observed_at,currentness:'unknown',
          required:copy(required),supply:copy(supply),whole_packs,existing_credit:0,surplus:surplus(required,supply,whole_packs),
          pack_spend:addMoney(Array.from({length:whole_packs},()=>standard),standard.currency),
          offers_applied:false,reusable_for_intent:false};comparedLines.add(row.demand.line_id);
      }catch(error){issues.push((error as {code?:string}).code==='unsupported_conversion'?'pack_measure_mismatch':'quote_unresolved');}
    }
    if(product.price_terms.kind!=='known')issues.push(product.price_terms.kind==='unknown'?'terms_unknown':'terms_ambiguous');
    if(product.source.expires_at===null)issues.push('source_expiry_unknown');
    if(evaluatedAt<product.source.observed_at||product.source.expires_at!==null&&evaluatedAt>=product.source.expires_at)issues.push('source_not_current');
    if(product.price_terms.kind==='known'){const terms=product.price_terms.value;
      if(terms.observed_at!==product.source.observed_at||evaluatedAt<terms.observed_at||evaluatedAt>=terms.valid_until)issues.push('terms_not_current');
      if(product.source.expires_at!==null&&terms.valid_until>product.source.expires_at)issues.push('terms_exceed_source_window');
      if(product.standard_unit_price.kind==='known'&&await digest(terms.unit_price)!==await digest(product.standard_unit_price.value))issues.push('standard_price_terms_mismatch');}
    if(!issues.length&&row&&product.pack.kind==='known'&&product.price_terms.kind==='known'){
      // Outside-account membership is unknown. A caller's loyalty flag cannot
      // lower this conditional subtotal; account-gated offers are not used.
      try{const required=row.demand.quantity,supply=product.pack.value,whole_packs=minimumPacks(required,supply),result=packQuote(required,supply,{...product.price_terms.value,loyalty_eligible:false},0,whole_packs,evaluatedAt);
        quote={status:'conditional_arithmetic',required:copy(required),supply:copy(supply),whole_packs,existing_credit:0,surplus:result.surplus,pack_spend:result.quoted_new_pack_spend,reusable_for_intent:false};quotedLines.add(row.demand.line_id);
      }catch(error){issues.push((error as {code?:string}).code==='unsupported_conversion'?'pack_measure_mismatch':'quote_unresolved');}
    }
    products.push({observation:product,mapping:review,source_status:'unqualified',meaning_status:'unqualified',native_matching:null,historical_comparison,quote,issues});
  }
  const prices=products.flatMap(product=>product.quote?[product.quote.pack_spend]:[]),issues:ReviewIssue[]=[];let quoted_pack_subtotal:Money|null=null;
  if(prices.length){try{quoted_pack_subtotal=addMoney(prices,prices[0]!.currency);}catch(error){if((error as {code?:string}).code==='currency_mismatch')issues.push('currency_mismatch');else throw error;}}
  const historicalPrices=products.flatMap(product=>product.historical_comparison?[product.historical_comparison.pack_spend]:[]);let historical_pack_subtotal:Money|null=null;
  if(historicalPrices.length){try{historical_pack_subtotal=addMoney(historicalPrices,historicalPrices[0]!.currency);}catch(error){if((error as {code?:string}).code==='currency_mismatch'){if(!issues.includes('currency_mismatch'))issues.push('currency_mismatch');}else throw error;}}
  return freeze({format:'mstrchf-product-review-draft',version:1,status:'review_only',owner_binding:current,demand_binding,native_list:projection,evaluated_at:evaluatedAt,
    products,quoted_pack_subtotal,subtotal_scope:'quoted_rows_only',unquoted_line_ids:projection.lines.filter(line=>line.action==='add_to_shop'&&!quotedLines.has(line.demand.line_id)).map(line=>line.demand.line_id),issues,
    historical_pack_subtotal,historical_subtotal_scope:'compared_rows_only',uncompared_line_ids:projection.lines.filter(line=>line.action==='add_to_shop'&&!comparedLines.has(line.demand.line_id)).map(line=>line.demand.line_id),
    baseline:{status:'unknown',accepted_read_ref:null},account_status:'unknown',availability_status:'unknown',product_dates_status:'unknown',delivery_status:'unknown',fees_status:'unknown',effect_enabled:false,reusable_for_intent:false,
    notice:'Historical comparisons use only explicitly declared as-observed standard base prices, with no offers or currentness claim. Current conditional quotes require the declared date/expiry terms. Neither is best-price, checkout or reusable intent evidence. Source and meaning remain unqualified; basket, account, stock, product dates, delivery and fees are unknown. No native match, intent, approval or external effect is created.'});
}

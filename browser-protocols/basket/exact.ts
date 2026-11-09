// Exact purchase-pack/currency arithmetic only. No food/form conversion or FX.
export type Rational={numerator:string;denominator:string};
export type Quantity={amount:Rational;unit:string;form:'raw'|'dry'|'cooked'|'drained'|'whole'|'prepared'};
export type Currency={code:string;minor_exponent:number};
export type Money={currency:Currency;minor_units:string};
export type Offer={offer_id:string;packs:number;total:Money;valid_from:string;valid_until:string;loyalty_required:boolean;delivery_applicable:'yes'|'no'|'unknown'};
export type PriceTerms={unit_price:Money;offers:Offer[];loyalty_eligible:boolean;delivery_at:string;observed_at:string;valid_until:string};
export class BasketProtocolError extends Error{constructor(readonly code:string,readonly path:string){super(`${code}: ${path}`);}}
export const fail=(code:string,path:string):never=>{throw new BasketProtocolError(code,path);};
export const keys=(value:unknown,names:string[],path:string):Record<string,unknown>=>{
 if(!value||typeof value!=='object'||Array.isArray(value)||![Object.prototype,null].includes(Object.getPrototypeOf(value)))return fail('invalid_value',path);
 const actual=Object.keys(value).sort(),expected=[...names].sort();if(actual.length!==expected.length||actual.some((key,index)=>key!==expected[index]))return fail('invalid_shape',path);return value as Record<string,unknown>;
};
export const count=(value:unknown,path:string,max=10000):number=>{if(typeof value!=='number'||!Number.isSafeInteger(value)||Object.is(value,-0)||value<0||value>max)return fail('limit_exceeded',path);return value;};
export const boundedText=(value:unknown,path:string,max=256):string=>{if(typeof value!=='string'||!value.length||value.length>max||/\p{Cc}/u.test(value))return fail('invalid_value',path);return value;};
export const instant=(value:unknown,path:string):string=>{if(typeof value!=='string'||!/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$/.test(value)||!Number.isFinite(Date.parse(value))||new Date(value).toISOString().replace('.000Z','Z')!==value)return fail('invalid_value',path);return value;};
export const decimal=(value:unknown,path:string):bigint=>{if(typeof value!=='string'||! /^(0|[1-9]\d{0,19})$/.test(value))return fail('noncanonical_value',path);const n=BigInt(value);if(n>18446744073709551615n)return fail('arithmetic_overflow',path);return n;};
const checked=(n:bigint,path:string)=>{if(n<0n||n>18446744073709551615n)return fail('arithmetic_overflow',path);return n;};
const gcd=(a:bigint,b:bigint):bigint=>{while(b){[a,b]=[b,a%b];}return a;};
export function rational(value:unknown,path:string,zero=false):Rational{const v=keys(value,['numerator','denominator'],path),n=decimal(v.numerator,path+'/numerator'),d=decimal(v.denominator,path+'/denominator');if(!d||(!zero&&!n)||gcd(n,d)!==1n)return fail('noncanonical_value',path);return {numerator:n.toString(),denominator:d.toString()};}
export function quantity(value:unknown,path:string):Quantity{const v=keys(value,['amount','unit','form'],path);if(!['raw','dry','cooked','drained','whole','prepared'].includes(v.form as string))return fail('invalid_value',path+'/form');return {amount:rational(v.amount,path+'/amount'),unit:boundedText(v.unit,path+'/unit',16),form:v.form as Quantity['form']};}
export function currency(value:unknown,path:string):Currency{const v=keys(value,['code','minor_exponent'],path);if(typeof v.code!=='string'||! /^[A-Z]{3}$/.test(v.code))return fail('invalid_value',path+'/code');return {code:v.code,minor_exponent:count(v.minor_exponent,path+'/minor_exponent',4)};}
export const sameCurrency=(a:Currency,b:Currency)=>a.code===b.code&&a.minor_exponent===b.minor_exponent;
export function money(value:unknown,path:string):Money{const v=keys(value,['currency','minor_units'],path);return {currency:currency(v.currency,path+'/currency'),minor_units:decimal(v.minor_units,path+'/minor_units').toString()};}
export function addMoney(values:Money[],expected:Currency):Money{const unit=currency(expected,'/currency');let total=0n;for(const raw of values){const value=money(raw,'/money');if(!sameCurrency(value.currency,unit))return fail('currency_mismatch','/money');total=checked(total+decimal(value.minor_units,'/money'),'/money/total');}return {currency:unit,minor_units:total.toString()};}
export function formatMoney(value:Money):string{const m=money(value,'/money'),digits=m.currency.minor_exponent,raw=m.minor_units.padStart(digits+1,'0');return `${m.currency.code} ${digits?raw.slice(0,-digits)+'.'+raw.slice(-digits):raw}`;}
export function minimumPacks(required:Quantity,supply:Quantity):number{
 required=quantity(required,'/required');supply=quantity(supply,'/supply');
 if(required.unit!==supply.unit||required.form!==supply.form)return fail('unsupported_conversion','/pack/supply');
 const a=BigInt(required.amount.numerator)*BigInt(supply.amount.denominator),b=BigInt(required.amount.denominator)*BigInt(supply.amount.numerator),n=(a+b-1n)/b;
 if(n>10000n)return fail('limit_exceeded','/pack/count');return Number(n);
}
export function surplus(required:Quantity,supply:Quantity,packs:number):Rational{
 required=quantity(required,'/required');supply=quantity(supply,'/supply');count(packs,'/packs');
 const n=BigInt(supply.amount.numerator)*BigInt(packs)*BigInt(required.amount.denominator)-BigInt(required.amount.numerator)*BigInt(supply.amount.denominator),d=BigInt(supply.amount.denominator)*BigInt(required.amount.denominator);
 if(n<0n)return fail('insufficient_quantity','/pack/coverage');const g=gcd(n,d);return {numerator:checked(n/g,'/pack/surplus').toString(),denominator:checked(d/g,'/pack/surplus').toString()};
}
export function priceTerms(value:unknown,path:string):PriceTerms{
 const v=keys(value,['unit_price','offers','loyalty_eligible','delivery_at','observed_at','valid_until'],path),unit_price=money(v.unit_price,path+'/unit_price');
 if(!Array.isArray(v.offers)||v.offers.length>16||typeof v.loyalty_eligible!=='boolean')return fail('invalid_value',path);
 const seen=new Set<string>();const offers=v.offers.map((entry,index):Offer=>{const p=`${path}/offers/${index}`,o=keys(entry,['offer_id','packs','total','valid_from','valid_until','loyalty_required','delivery_applicable'],p),id=boundedText(o.offer_id,p+'/offer_id');if(seen.has(id))return fail('duplicate_key',p);seen.add(id);const packs=count(o.packs,p+'/packs');if(!packs||typeof o.loyalty_required!=='boolean'||!['yes','no','unknown'].includes(o.delivery_applicable as string))return fail('invalid_value',p);const total=money(o.total,p+'/total');if(!sameCurrency(unit_price.currency,total.currency))return fail('currency_mismatch',p);const valid_from=instant(o.valid_from,p+'/valid_from'),valid_until=instant(o.valid_until,p+'/valid_until');if(valid_from>valid_until)return fail('invalid_value',p);return {offer_id:id,packs,total,valid_from,valid_until,loyalty_required:o.loyalty_required,delivery_applicable:o.delivery_applicable as Offer['delivery_applicable']};});
 const delivery_at=instant(v.delivery_at,path+'/delivery_at'),observed_at=instant(v.observed_at,path+'/observed_at'),valid_until=instant(v.valid_until,path+'/valid_until');if(observed_at>valid_until)return fail('invalid_value',path);
 return {unit_price,offers,loyalty_eligible:v.loyalty_eligible,delivery_at,observed_at,valid_until};
}
// Repeated, independent per-product offers only. Unknown delivery applicability,
// loyalty or date eligibility never lowers a quoted amount. No cross-SKU offers.
export function quotePacks(value:PriceTerms,packs:number,now:string):Money{
 const terms=priceTerms(value,'/price');count(packs,'/packs');instant(now,'/now');if(now<terms.observed_at||now>terms.valid_until)return fail('price_stale','/price');
 const offers=terms.offers.filter(o=>o.delivery_applicable==='yes'&&(!o.loyalty_required||terms.loyalty_eligible)&&terms.delivery_at>=o.valid_from&&terms.delivery_at<=o.valid_until);
 const costs:bigint[]=[0n];for(let n=1;n<=packs;n++){let best=costs[n-1]!+BigInt(terms.unit_price.minor_units);for(const offer of offers)if(offer.packs<=n){const candidate=costs[n-offer.packs]!+BigInt(offer.total.minor_units);if(candidate<best)best=candidate;}costs.push(best);}
 return {currency:terms.unit_price.currency,minor_units:checked(costs[packs]!, '/price/total').toString()};
}
export function packQuote(required:Quantity,supply:Quantity,terms:PriceTerms,credit:number,packs:number,now:string){
 const needed=minimumPacks(quantity(required,'/required'),quantity(supply,'/supply'));count(credit,'/credit');count(packs,'/packs');if(credit+packs<needed)return fail('insufficient_quantity','/pack/coverage');if(credit+packs>10000)return fail('limit_exceeded','/pack/coverage');
 return {minimum_new_packs:Math.max(0,needed-credit),new_packs:packs,credited_existing_packs:credit,surplus:surplus(required,supply,credit+packs),quoted_new_pack_spend:quotePacks(terms,packs,now)};
}

// V2 guide-price arithmetic. Observation time is not a retailer expiry, and an
// operational approval deadline belongs to the intent, never to this price.
export type ObservedSinglePackPrice={kind:'observed_single_pack';unit_price:Money;observed_at:string;retailer_price_guaranteed:false;offers:'excluded';delivery:'unknown';fees:'unknown'};
export function observedSinglePackPrice(raw:unknown,path='/observed_price'):ObservedSinglePackPrice{
 const value=keys(raw,['kind','unit_price','observed_at','retailer_price_guaranteed','offers','delivery','fees'],path);
 if(value.kind!=='observed_single_pack'||value.retailer_price_guaranteed!==false||value.offers!=='excluded'||value.delivery!=='unknown'||value.fees!=='unknown')return fail('unsupported_price_terms',path);
 return {kind:'observed_single_pack',unit_price:money(value.unit_price,path+'/unit_price'),observed_at:instant(value.observed_at,path+'/observed_at'),retailer_price_guaranteed:false,offers:'excluded',delivery:'unknown',fees:'unknown'};
}
export function observedOnePackQuote(required:Quantity,supply:Quantity,raw:ObservedSinglePackPrice,now:string){
 const price=observedSinglePackPrice(raw);instant(now,'/now');if(now<price.observed_at)return fail('invalid_value','/observed_price/time');
 const needed=minimumPacks(required,supply);if(needed!==1)return fail('insufficient_quantity','/observed_price/one_pack_coverage');
 return {new_packs:1 as const,credited_existing_packs:0 as const,surplus:surplus(required,supply,1),observed_new_pack_spend:price.unit_price,retailer_price_guaranteed:false as const};
}

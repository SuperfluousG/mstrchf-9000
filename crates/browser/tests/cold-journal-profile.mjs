import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';
const path=process.argv[2];
const wasm=await readFile(path?pathToFileURL(path):new URL('../target/wasm32-unknown-unknown/release/mstrchf_browser.wasm',import.meta.url));
const state=JSON.parse(await readFile(new URL('./fixtures/archived-conflicts-110.json',import.meta.url),'utf8'));
const now='2026-10-19T12:00:00Z';
const compilation=performance.now(),module=await WebAssembly.compile(wasm);
assert.deepEqual(WebAssembly.Module.imports(module),[]);
console.log(JSON.stringify({stage:'compile',duration_ms:Math.round(performance.now()-compilation),wasm_bytes:wasm.length,fixture_bytes:JSON.stringify(state).length,receipts:state.receipts.length,selections:state.history.selected.length}));
const {exports:e}=await WebAssembly.instantiate(module,{});
function call(request){const bytes=new TextEncoder().encode(JSON.stringify(request)),ptr=e.alloc(bytes.length);new Uint8Array(e.memory.buffer,ptr,bytes.length).set(bytes);const packed=e.run(ptr,bytes.length),out=Number(packed&0xffffffffn),len=Number(packed>>32n);const response=JSON.parse(new TextDecoder().decode(new Uint8Array(e.memory.buffer,out,len)));e.dealloc(ptr,bytes.length);e.dealloc(out,len);assert.equal(response.ok,true,JSON.stringify(response));return response.value;}
let start=performance.now();console.log(JSON.stringify({stage:'cold_validate_started'}));
const restored=call({op:'validate_state',state,now});assert.deepEqual(restored,state);
console.log(JSON.stringify({stage:'cold_validate',duration_ms:Math.round(performance.now()-start),memory_bytes:e.memory.buffer.byteLength}));
start=performance.now();const snapshot=call({op:'snapshot',state,now});assert.equal(snapshot.snapshot_revision,state.snapshot.snapshot_revision);console.log(JSON.stringify({stage:'warm_snapshot',duration_ms:Math.round(performance.now()-start)}));
start=performance.now();const page=call({op:'conflicts',state,now,offset:100,limit:100});assert.equal(page.total,110);assert.equal(page.conflicts.length,10);console.log(JSON.stringify({stage:'warm_conflicts_page_2',duration_ms:Math.round(performance.now()-start),total:page.total,page_size:page.conflicts.length}));

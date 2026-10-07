import { test } from 'node:test';
import assert from 'node:assert/strict';
import { exactToolchain } from '../../scripts/exact-toolchain.mjs';
const channel='1.94.0-x86_64-pc-windows-msvc';
function fixture(change={}) {
  const calls=[];
  const options={home:'C:/synthetic-home',env:{PATH:'foreign-bin',RUSTC_WRAPPER:'bad',RUSTC_WORKSPACE_WRAPPER:'bad',RUSTUP_AUTO_INSTALL:'1'},
    read:()=>`[toolchain]\nchannel = "${channel}"\n`,exists:()=>true,
    spawn:(file,args,opts)=>{
      calls.push({file,args,opts});
      const name=file.includes('rustdoc')?'rustdoc':file.includes('rustc')?'rustc':'cargo';
      return {status:0,stdout:`${name} 1.94.0 (abc)\nrelease: 1.94.0\nhost: x86_64-pc-windows-msvc\ncommit-hash: abc\n`};
    },...change};
  return {options,calls};
}
test('uses only direct pinned sibling tools and suppresses wrappers/install in every probe and child',()=>{
  const {options,calls}=fixture(); const result=exactToolchain('C:/project',options);
  assert.equal(calls.length,3);
  for(const call of calls) {
    assert(call.file.includes(channel)); assert(call.file.endsWith('.exe')); assert.deepEqual(call.args,['--version','--verbose']);
    assert.equal(call.opts.env.RUSTC_WRAPPER,'');assert.equal(call.opts.env.RUSTC_WORKSPACE_WRAPPER,'');assert.equal(call.opts.env.RUSTUP_AUTO_INSTALL,'0');
  }
  assert.equal(result.env.CARGO,result.cargo);assert.equal(result.env.RUSTUP_TOOLCHAIN,channel);
  assert(result.env.PATH.startsWith(result.bin)); assert(!result.cargo.includes('stable'));
});
test('fails closed on absent named toolchain without probing any fallback',()=>{
  const {options,calls}=fixture({exists:()=>false});assert.throws(()=>exactToolchain('C:/project',options),/absent/);assert.equal(calls.length,0);
});
test('rejects floating or changed pins before invoking tools',()=>{
  const {options,calls}=fixture({read:()=> '[toolchain]\nchannel = "stable"'});assert.throws(()=>exactToolchain('C:/project',options),/pin/);assert.equal(calls.length,0);
});
for(const [label,stdout] of [['release','release: 1.99.0\nhost: x86_64-pc-windows-msvc\ncommit-hash: abc'],['host','release: 1.94.0\nhost: other\ncommit-hash: abc'],['identity','release: 1.94.0\nhost: x86_64-pc-windows-msvc\ncommit-hash: foreign']]) {
  test(`rejects rustdoc ${label} drift`,()=>{
    const {options}=fixture();const good=options.spawn;options.spawn=(file,args,opts)=>file.includes('rustdoc')?{status:0,stdout}:good(file,args,opts);
    assert.throws(()=>exactToolchain('C:/project',options),/identity/);
  });
}
test('failed direct compiler probe never falls through to proxy',()=>{
  const {options,calls}=fixture();options.spawn=(file,args,opts)=>{calls.push(file);return {status:1,stdout:''};};
  assert.throws(()=>exactToolchain('C:/project',options),/identity/);assert.equal(calls.length,1);
});

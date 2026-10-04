import { readFileSync, existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { resolve, delimiter } from 'node:path';
import { spawnSync } from 'node:child_process';

// Fail closed: no rustup invocation, floating channel or installation fallback.
export function exactToolchain(root, { home = homedir(), env = process.env,
  read = file => readFileSync(file, 'utf8'), exists = existsSync, spawn = spawnSync } = {}) {
  const channel = read(resolve(root, 'rust-toolchain.toml')).match(/^channel\s*=\s*"([^"]+)"\s*$/m)?.[1];
  if (channel !== '1.94.0-x86_64-pc-windows-msvc') throw new Error('Unexpected exact Rust pin');
  const bin = resolve(home, '.rustup', 'toolchains', channel, 'bin');
  const cargo = resolve(bin, 'cargo.exe'), rustc = resolve(bin, 'rustc.exe'), rustdoc = resolve(bin, 'rustdoc.exe');
  if (![cargo,rustc,rustdoc].every(exists)) throw new Error('Exact installed Rust toolchain absent');
  const childEnv = { ...env, PATH: `${bin}${delimiter}${env.PATH ?? ''}`, CARGO: cargo, RUSTC: rustc, RUSTDOC: rustdoc,
    RUSTUP_TOOLCHAIN: channel, RUSTUP_AUTO_INSTALL: '0', RUSTC_WRAPPER: '', RUSTC_WORKSPACE_WRAPPER: '' };
  const identities = [cargo,rustc,rustdoc].map(tool => {
    const result = spawn(tool, ['--version','--verbose'], { cwd: root, encoding: 'utf8', env: childEnv });
    const field = key => result.stdout?.match(new RegExp(`^${key}: (.+)$`, 'm'))?.[1]?.trim();
    if (result.error || result.status !== 0 || field('release') !== '1.94.0'
      || field('host') !== 'x86_64-pc-windows-msvc' || !field('commit-hash')) throw new Error(`Tool identity mismatch: ${tool}`);
    return { path: tool, commit: field('commit-hash'), version: result.stdout.trim() };
  });
  if (identities[1].commit !== identities[2].commit) throw new Error('Compiler/documentation tool identity mismatch');
  return { bin, cargo, env: childEnv, identities };
}

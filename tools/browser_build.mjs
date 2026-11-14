// Run the website's real worker (web/worker.js) under Bun against a local macOS kit.
//
// usage: bun tools/browser_build.mjs WORKDIR KIT.zip RGFX.EXE OUTPUT [--allow-mismatch]
//
// tools.build_kits prepares WORKDIR with web/worker.js and web/wasi.js unchanged, the wasm-bindgen
// `pkg/` of maiko-patcher and the kit's `dlang.js`. The driver stands in for the browser only
// by serving the kit instead of fetching it, so the build is the one a visitor's browser runs.
//
// Writes OUTPUT.universal (the universal executable the worker linked, saved before the package
// gate compares it) and, when the gate passes, OUTPUT (the package ZIP). With --allow-mismatch a
// kit exported with a placeholder hash is expected to fail the gate with "Compiled executable
// differs from the accepted build"; the saved executable is what the final kit must accept.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';

const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const args = process.argv.slice(2);
const allowMismatch = args.includes('--allow-mismatch');
const [workdir, kitPath, exePath, outPath] = args.filter(arg => !arg.startsWith('--'));
if (!outPath) throw new Error('usage: bun tools/browser_build.mjs WORKDIR KIT.zip RGFX.EXE OUTPUT [--allow-mismatch]');
const work = resolve(workdir);
const kit = readFileSync(kitPath);
const digest = sha256(kit);

const realFetch = globalThis.fetch;
globalThis.fetch = async (url, options) => {
  const target = String(url);
  if (target.endsWith('kits.json')) return new Response(JSON.stringify({ macos: { sha256: digest } }));
  if (target.endsWith('kits/macos.zip')) return new Response(kit);
  if (target.startsWith('file:')) return new Response(readFileSync(new URL(target)), { headers: { 'content-type': 'application/wasm' } });
  return realFetch(url, options);
};

const started = Date.now();
const finished = new Promise(resolveDone => {
  globalThis.self = globalThis;
  globalThis.postMessage = message => {
    if (message.kind === 'progress') console.log(`[${((Date.now() - started) / 1000).toFixed(0)}s] ${message.message}`);
    else resolveDone(message);
  };
});

const pkg = await import(`${work}/pkg/maiko_patcher.js`);
const universal = pkg.Builder.prototype.universal;
pkg.Builder.prototype.universal = function (...parts) {
  const executable = universal.apply(this, parts);
  writeFileSync(`${outPath}.universal`, executable);
  console.log(`universal ${executable.length} ${sha256(executable)}`);
  return executable;
};
await import(`${work}/worker.js`);
const input = readFileSync(exePath);
globalThis.onmessage({ data: { input: input.buffer.slice(input.byteOffset, input.byteOffset + input.length), target: 'macos' } });
const result = await finished;
// The worker keeps handles open after it finishes, so exit explicitly.
if (result.kind === 'complete') {
  const zip = new Uint8Array(result.buffer);
  writeFileSync(outPath, zip);
  console.log(`package ${zip.length} ${sha256(zip)}`);
  process.exit(0);
}
console.error(result.message);
process.exit(allowMismatch && result.message.includes('Compiled executable differs from the accepted build') ? 0 : 1);

// Run the website's real worker (web/worker.js) under Bun against a local kit.
//
// usage: bun tools/browser_build.mjs WORKDIR TARGET KIT.zip RGFX.EXE OUTPUT.zip
//
// WORKDIR holds web/worker.js and web/wasi.js unchanged, the wasm-bindgen `pkg/` of maiko-patcher
// and, for macOS, the kit's `dlang.js`. The driver stands in for the browser only by serving the
// kit instead of fetching it, so the build is the one a visitor's browser runs.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';

const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const [workdir, target, kitPath, exePath, outPath] = process.argv.slice(2);
if (!outPath) throw new Error('usage: bun tools/browser_build.mjs WORKDIR TARGET KIT.zip RGFX.EXE OUTPUT.zip');
const work = resolve(workdir);
const kit = readFileSync(kitPath);

const realFetch = globalThis.fetch;
globalThis.fetch = async (url, options) => {
  const name = String(url);
  if (name.endsWith('kits.json')) return new Response(JSON.stringify({ [target]: { sha256: sha256(kit) } }));
  if (name.endsWith(`kits/${target}.zip`)) return new Response(kit);
  if (name.startsWith('file:')) return new Response(readFileSync(new URL(name)), { headers: { 'content-type': 'application/wasm' } });
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

await import(`${work}/worker.js`);
const input = readFileSync(exePath);
globalThis.onmessage({ data: { input: input.buffer.slice(input.byteOffset, input.byteOffset + input.length), target } });
const result = await finished;
// The worker keeps handles open after it finishes, so exit explicitly.
if (result.kind === 'complete') {
  const zip = new Uint8Array(result.buffer);
  writeFileSync(outPath, zip);
  console.log(`package ${zip.length} ${sha256(zip)}`);
  process.exit(0);
}
console.error(result.message);
process.exit(1);

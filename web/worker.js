import init, { Builder, validate } from './pkg/maiko_patcher.js';
import { runWasi } from './wasi.js';

self.onmessage = async ({ data }) => {
  let builder;
  const logs = [];
  const log = line => { logs.push(line); if (logs.length > 24) logs.shift(); };
  const stage = message => self.postMessage({ kind: 'progress', message });
  try {
    stage('Checking your executable…');
    const wasmUrl = new URL('./pkg/maiko_patcher_bg.wasm', import.meta.url);
    wasmUrl.search = new URL(import.meta.url).search;
    await init({ module_or_path: wasmUrl });
    const input = new Uint8Array(data.input);
    validate(input);
    stage('Downloading build tools…');
    const index = await fetch('./kits.json', { cache: 'no-cache' });
    if (!index.ok) throw new Error('The runtime index is unavailable. Try again later.');
    const kits = await index.json();
    const kit = kits[data.target];
    if (!kit || typeof kit.sha256 !== 'string') throw new Error('This platform does not have a published runtime kit yet.');
    const response = await fetch(`./kits/${data.target}.zip`, { cache: 'no-cache' });
    if (!response.ok) throw new Error('The runtime download failed. Check your connection and try again.');
    const bytes = await response.arrayBuffer();
    const digest = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)), byte => byte.toString(16).padStart(2, '0')).join('');
    if (digest !== kit.sha256) throw new Error('The runtime download failed its checksum check. Reload the page and try again.');
    builder = new Builder(input, new Uint8Array(bytes), data.target);
    stage('Building game engine…');
    let files = builder.prepare();
    const sr = await WebAssembly.compile(builder.tool('sr'));
    files = await runWasi(sr, ['sr', 'RGFX.EXE', 'rgfx.asm'], files, ['rgfx.asm', ...Array.from({ length: 7 }, (_, index) => `seg${String(index + 1).padStart(2, '0')}.inc`)], log);
    files = builder.finish(files);
    const nasm = await WebAssembly.compile(builder.tool('nasm'));
    for (const [args, output] of builder.assembly()) {
      stage(`Assembling ${args[args.length - 3]}…`);
      files = await runWasi(nasm, args, files, [output], log);
    }
    stage('Verifying executable…');
    const executable = builder.relink(files[data.target === 'linux' ? 'rgfx.o' : 'rgfx.obj']);
    stage('Packing your ZIP…');
    const zip = builder.package(executable);
    const buffer = zip.slice().buffer;
    self.postMessage({ kind: 'complete', buffer }, [buffer]);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    self.postMessage({ kind: 'error', message: `${message}${logs.length ? `\n\nCompiler output:\n${logs.join('\n')}` : ''}` });
  } finally {
    builder?.free();
  }
};

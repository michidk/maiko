import init, { Builder, validate } from './pkg/maiko_patcher.js';
import { runWasi, runLlasm } from './wasi.js';

self.onmessage = async ({ data }) => {
  let builder;
  const logs = [];
  // The assembler prints tens of thousands of harmless overflow warnings; count them so that real messages stay visible.
  let warnings = 0;
  const log = line => { if (/: warning: /.test(line)) { warnings++; return; } logs.push(line); if (logs.length > 24) logs.shift(); };
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
    builder = new Builder(input, new Uint8Array(bytes), data.target, Boolean(data.v5Fixes));
    stage('Building game engine…');
    let files = builder.prepare();
    let executable;
    if (data.target === 'macos') {
      const sr = await WebAssembly.compile(builder.tool('sr-llasm'));
      const outputs = ['rgfx.llasm', ...Array.from({ length: 7 }, (_, index) => ['code', 'data'].map(kind => `seg${String(index + 1).padStart(2, '0')}_${kind}.llinc`)).flat()];
      files = await runWasi(sr, ['sr', 'RGFX.EXE', 'rgfx.llasm'], files, outputs, log);
      files = builder.finish(files);
      stage('Translating the game to LLVM…');
      let ir = await runLlasm(builder, files);
      files = null;
      stage('Optimizing the Mac engine…');
      const opt = await WebAssembly.compile(builder.tool('opt'));
      const optimized = await runWasi(opt, builder.optimization(), { 'game.ll': ir }, ['game.bc'], log);
      ir = null;
      const bitcode = optimized['game.bc'];
      delete optimized['game.ll'];
      if (!bitcode?.length) throw new Error('LLVM did not produce the optimized engine.');
      const images = [];
      const modules = new Map();
      for (const arch of ['x86_64', 'arm64']) {
        let current = { 'game.bc': bitcode };
        for (const [tool, args, output] of builder.compilation(arch)) {
          stage(`${{ llc: 'Compiling', lld: 'Linking' }[tool]} for ${arch === 'arm64' ? 'Apple Silicon' : 'Intel'}…`);
          if (!modules.has(tool)) modules.set(tool, await WebAssembly.compile(builder.tool(tool)));
          if (tool === 'lld') Object.assign(current, builder.linker_files(arch));
          const result = await runWasi(modules.get(tool), args, current, [output], log);
          if (!result[output]?.length) throw new Error(`${tool} did not produce ${output}.`);
          current = { [output]: result[output] };
        }
        images.push(Object.values(current)[0]);
      }
      stage('Verifying the universal Mac executable…');
      executable = builder.universal(images[0], images[1]);
    } else {
      const sr = await WebAssembly.compile(builder.tool('sr'));
      files = await runWasi(sr, ['sr', 'RGFX.EXE', 'rgfx.asm'], files, ['rgfx.asm', ...Array.from({ length: 7 }, (_, index) => `seg${String(index + 1).padStart(2, '0')}.inc`)], log);
      files = builder.finish(files);
      const nasm = await WebAssembly.compile(builder.tool('nasm'));
      for (const [args, output] of builder.assembly()) {
        stage(`Assembling ${args[args.length - 3]}…`);
        files = await runWasi(nasm, args, files, [output], log);
      }
      stage('Linking the game…');
      const [args, output, inputs] = builder.link();
      const objects = Object.fromEntries(builder.assembly().map(([, object]) => [object, files[object]]));
      files = null;
      const lld = await WebAssembly.compile(builder.tool('lld'));
      executable = (await runWasi(lld, args, { ...inputs, ...objects }, [], log))[output];
      if (!executable?.length) throw new Error('The linker did not produce the game executable.');
    }
    stage('Packing your ZIP…');
    const zip = builder.package(executable);
    const buffer = zip.slice().buffer;
    self.postMessage({ kind: 'complete', buffer }, [buffer]);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    self.postMessage({ kind: 'error', message: `${message}${logs.length ? `\n\nCompiler output:\n${logs.join('\n')}` : ''}${warnings ? `\n\n(${warnings} compiler warnings hidden; they are expected.)` : ''}` });
  } finally {
    builder?.free();
  }
};

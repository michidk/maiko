import { cp, mkdir, readFile, readdir, rm } from 'node:fs/promises';
import { resolve, join } from 'node:path';

function command(args) {
  const result = Bun.spawnSync(args, { stdout: 'inherit', stderr: 'inherit' });
  if (result.exitCode !== 0) throw new Error(`${args[0]} exited with ${result.exitCode}`);
}
command(['cargo', 'build', '--locked', '--release', '-p', 'maiko-patcher', '--lib', '--target', 'wasm32-unknown-unknown']);
const result = Bun.spawnSync(['cargo', 'metadata', '--format-version=1', '--no-deps', '--locked']);
if (result.exitCode !== 0) throw new Error(new TextDecoder().decode(result.stderr));
const metadata = JSON.parse(new TextDecoder().decode(result.stdout));
command(['wasm-bindgen', '--target', 'web', '--out-dir', 'web/pkg', join(metadata.target_directory, 'wasm32-unknown-unknown/release/maiko_patcher.wasm')]);
await rm('web/dist', { recursive: true, force: true });
await mkdir('web/dist/kits', { recursive: true });
const built = await Bun.build({ entrypoints: ['web/main.js', 'web/worker.js'], outdir: 'web/dist', target: 'browser', minify: true, external: ['./pkg/maiko_patcher.js'] });
if (!built.success) throw new AggregateError(built.logs, 'Website build failed');
for (const name of ['index.html', 'style.css', 'favicon.svg', 'og.png']) await cp(`web/${name}`, `web/dist/${name}`);
await cp('web/pkg', 'web/dist/pkg', { recursive: true });
await cp('web/fonts', 'web/dist/fonts', { recursive: true });
// Kits are release assets. The site takes the latest release's kits and checks each one
// against that release's SHA256SUMS, so there is no pin to maintain.
const releases = process.env.MAIKO_KIT_URL ?? 'https://github.com/michidk/maiko/releases/latest/download';
const local = process.env.MAIKO_KIT_DIRECTORY;
const fetchAsset = async name => {
  if (local) return new Uint8Array(await readFile(resolve(local, name)));
  const url = new URL(`${releases}/${name}`);
  if (url.protocol !== 'https:' || url.hostname !== 'github.com' || !url.pathname.startsWith('/michidk/maiko/releases/')) throw new Error('Runtime kits must come from a Maiko release.');
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Cannot download ${name}: HTTP ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
};
const sums = new Map(new TextDecoder().decode(await fetchAsset('SHA256SUMS')).split('\n').flatMap(line => {
  const [, hash, name] = line.match(/^([a-f0-9]{64}) [ *](.+)$/) ?? [];
  return hash ? [[name, hash]] : [];
}));
const kits = {};
for (const target of ['linux', 'windows', 'macos']) {
  const name = `maiko-runtime-kit-${target}.zip`;
  const bytes = await fetchAsset(name);
  const sha256 = new Bun.CryptoHasher('sha256').update(bytes).digest('hex');
  if (sums.get(name) !== sha256) throw new Error(`${target} runtime kit checksum mismatch`);
  kits[target] = { sha256 };
  await Bun.write(`web/dist/kits/${target}.zip`, bytes);
}
await Bun.write('web/dist/kits.json', JSON.stringify(kits, null, 2) + '\n');
command(['cargo', 'run', '--locked', '--release', '-p', 'maiko-patcher', '--bin', 'maiko-kit', '--', 'web-tools', 'web/dist/kits/macos.zip', 'web/dist']);
const fonts = (await readdir('web/dist/fonts')).sort();
const assets = ['index.html', 'style.css', 'main.js', 'worker.js', 'dlang.js', 'kits.json', 'pkg/maiko_patcher_bg.wasm', ...fonts.map(name => `fonts/${name}`)];
const hasher = new Bun.CryptoHasher('sha256');
for (const name of assets) hasher.update(name).update('\0').update(await readFile(`web/dist/${name}`));
const version = hasher.digest('hex').slice(0, 20);
const html = await readFile('web/dist/index.html', 'utf8');
await Bun.write('web/dist/index.html', html.replace('./style.css', `./style.css?v=${version}`).replace('./main.js', `./main.js?v=${version}`));
let css = await readFile('web/dist/style.css', 'utf8');
for (const name of fonts) css = css.replaceAll(`./fonts/${name}`, `./fonts/${name}?v=${version}`);
await Bun.write('web/dist/style.css', css);
console.info(`Built web/dist with verified runtime kits and asset version ${version}.`);

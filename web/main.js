import { detectPlatform } from './platform.js';

const file = document.querySelector('#executable');
const platforms = document.querySelector('#platforms');
const build = document.querySelector('#build');
const cancel = document.querySelector('#cancel');
const download = document.querySelector('#download');
const progress = document.querySelector('#progress');
const status = document.querySelector('#status');
const error = document.querySelector('#error');
let worker = null;
let downloadUrl = null;
const selectedTarget = () => document.querySelector('input[name="target"]:checked')?.value;

const detected = detectPlatform(navigator.userAgentData?.platform ?? navigator.platform, navigator.userAgent, navigator.maxTouchPoints);
if (detected) document.querySelector(`input[value="${detected}"]`).checked = true;
document.querySelector('#detection').textContent = detected
  ? `${{ windows: 'Windows', linux: 'Linux', macos: 'macOS' }[detected]} selected automatically.`
  : 'Select your target system.';

function resetDownload() {
  if (downloadUrl) URL.revokeObjectURL(downloadUrl);
  downloadUrl = null;
  download.hidden = true;
  download.removeAttribute('href');
}
function controls() {
  const busy = worker !== null;
  const macos = selectedTarget() === 'macos';
  document.querySelector('#browser-build').hidden = macos;
  document.querySelector('#browser-instructions').hidden = macos;
  document.querySelector('#macos-guidance').hidden = !macos;
  file.disabled = busy || macos;
  platforms.disabled = busy;
  cancel.hidden = !busy;
  progress.hidden = !busy;
  build.disabled = busy || macos || !file.files?.length || !selectedTarget();
}
function stop() { worker?.terminate(); worker = null; controls(); }
function failure(message) {
  stop();
  status.textContent = 'Build failed.';
  error.textContent = message;
  error.hidden = false;
}
function changed() {
  resetDownload();
  error.hidden = true;
  status.textContent = file.files?.length ? 'Ready to build.' : '';
  controls();
}
file.addEventListener('change', changed);
platforms.addEventListener('change', changed);
cancel.addEventListener('click', () => { stop(); status.textContent = 'Build cancelled.'; });
build.addEventListener('click', async () => {
  const input = file.files?.[0];
  const target = selectedTarget();
  if (!input || !target || target === 'macos') return;
  resetDownload();
  error.hidden = true;
  if (input.size > 32 * 1024 * 1024) { failure('RGFX.EXE must be smaller than 32 MiB. Select the executable rather than an archive or disc image.'); return; }
  if (typeof WebAssembly === 'undefined' || typeof Worker === 'undefined') { failure('Your browser needs WebAssembly and module workers. Use a current desktop browser or the Rust CLI.'); return; }
  try {
    const workerUrl = new URL('./worker.js', import.meta.url);
    workerUrl.search = new URL(import.meta.url).search;
    const current = new Worker(workerUrl, { type: 'module' });
    worker = current;
    controls();
    status.textContent = 'Reading your executable locally…';
    current.onerror = event => failure(`The build worker stopped: ${event.message || 'reload the page and try again.'}`);
    current.onmessage = ({ data }) => {
      if (worker !== current) return;
      if (data.kind === 'progress') status.textContent = data.message;
      else if (data.kind === 'error') failure(data.message);
      else if (data.kind === 'complete') {
        downloadUrl = URL.createObjectURL(new Blob([data.buffer], { type: 'application/zip' }));
        download.href = downloadUrl;
        download.download = `maiko-${target}-x64.zip`;
        download.hidden = false;
        stop();
        status.textContent = 'Your package is ready.';
        download.focus();
      }
    };
    const buffer = await input.arrayBuffer();
    if (worker === current) current.postMessage({ input: buffer, target }, [buffer]);
  } catch (cause) { failure(cause instanceof Error ? cause.message : String(cause)); }
});
window.addEventListener('pagehide', () => { stop(); resetDownload(); });

controls();

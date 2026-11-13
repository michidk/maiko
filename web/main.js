import { detectPlatform } from './platform.js';

const file = document.querySelector('#executable');
const platforms = document.querySelector('#platforms');
const build = document.querySelector('#build');
const cancel = document.querySelector('#cancel');
const download = document.querySelector('#download');
const progress = document.querySelector('#progress');
const status = document.querySelector('#status');
const logPanel = document.querySelector('#log-panel');
const log = document.querySelector('#log');
const copyLog = document.querySelector('#copy-log');
const dropzone = document.querySelector('#dropzone');
const dzName = document.querySelector('#dz-name');
const dzDetail = document.querySelector('#dz-detail');
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
  document.querySelector('#browser-instructions').hidden = macos;
  document.querySelector('#windows-instructions').hidden = selectedTarget() === 'linux';
  document.querySelector('#linux-instructions').hidden = selectedTarget() !== 'linux';
  document.querySelector('#macos-guidance').hidden = !macos;
  document.querySelector('#macos-instructions').hidden = !macos;
  file.disabled = busy;
  platforms.disabled = busy;
  cancel.hidden = !busy;
  progress.hidden = !busy;
  build.disabled = busy || !file.files?.length || !selectedTarget();
}
function stop() { worker?.terminate(); worker = null; controls(); }
// The log keeps every progress line and any error so that it can be copied and shared whole.
// It follows the newest line unless the reader has scrolled up to look at something earlier.
function logLine(text) {
  const following = log.scrollHeight - log.scrollTop - log.clientHeight < 24;
  log.textContent += (log.textContent ? '\n' : '') + text;
  logPanel.hidden = false;
  if (following) log.scrollTop = log.scrollHeight;
}
function clearLog() {
  log.textContent = '';
  logPanel.hidden = true;
  copyLog.textContent = 'Copy log';
}
function progressLine(message) {
  status.textContent = message;
  logLine(message);
}
function failure(message) {
  stop();
  status.textContent = 'Build failed.';
  logLine(`Build failed.\n${message}`);
  log.scrollTop = log.scrollHeight;
  logPanel.scrollIntoView({ block: 'nearest' });
}
async function copyTheLog() {
  const text = log.textContent;
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    // Without the clipboard API (or permission), select the text and use the older command.
    const range = document.createRange();
    range.selectNodeContents(log);
    const selection = getSelection();
    selection.removeAllRanges();
    selection.addRange(range);
    if (!document.execCommand?.('copy')) { copyLog.textContent = 'Select and copy manually'; return; }
  }
  copyLog.textContent = 'Copied';
  setTimeout(() => { copyLog.textContent = 'Copy log'; }, 2000);
}
copyLog.addEventListener('click', copyTheLog);
function changed() {
  resetDownload();
  clearLog();
  const chosen = file.files?.[0];
  dzName.textContent = chosen ? chosen.name : 'Choose RGFX.EXE or drop it here';
  dzDetail.textContent = chosen ? `${(chosen.size / 1024 / 1024).toFixed(1)} MB · stays on this device` : 'Only this one file is needed.';
  dropzone.classList.toggle('has-file', Boolean(chosen));
  status.textContent = file.files?.length ? 'Ready to build.' : '';
  controls();
}
file.addEventListener('change', changed);
dropzone.addEventListener('dragover', event => { event.preventDefault(); dropzone.classList.add('dragging'); });
dropzone.addEventListener('dragleave', () => dropzone.classList.remove('dragging'));
dropzone.addEventListener('drop', event => {
  event.preventDefault();
  dropzone.classList.remove('dragging');
  if (file.disabled || !event.dataTransfer?.files?.length) return;
  file.files = event.dataTransfer.files;
  changed();
});
platforms.addEventListener('change', changed);
cancel.addEventListener('click', () => { stop(); status.textContent = 'Build cancelled.'; });
build.addEventListener('click', async () => {
  const input = file.files?.[0];
  const target = selectedTarget();
  if (!input || !target) return;
  resetDownload();
  clearLog();
  if (input.size > 32 * 1024 * 1024) { failure('RGFX.EXE must be smaller than 32 MiB. Select the executable rather than an archive or disc image.'); return; }
  if (typeof WebAssembly === 'undefined' || typeof Worker === 'undefined') { failure('Your browser needs WebAssembly and module workers. Use a current desktop browser or the Rust CLI.'); return; }
  try {
    const workerUrl = new URL('./worker.js', import.meta.url);
    workerUrl.search = new URL(import.meta.url).search;
    const current = new Worker(workerUrl, { type: 'module' });
    worker = current;
    controls();
    progressLine('Reading your executable locally…');
    current.onerror = event => failure(`The build worker stopped: ${event.message || 'reload the page and try again.'}`);
    current.onmessage = ({ data }) => {
      if (worker !== current) return;
      if (data.kind === 'progress') progressLine(data.message);
      else if (data.kind === 'error') failure(data.message);
      else if (data.kind === 'complete') {
        downloadUrl = URL.createObjectURL(new Blob([data.buffer], { type: 'application/zip' }));
        download.href = downloadUrl;
        download.download = target === 'macos' ? 'maiko-macos-universal.zip' : `maiko-${target}-x64.zip`;
        download.hidden = false;
        stop();
        progressLine('Your package is ready.');
        download.focus();
      }
    };
    const buffer = await input.arrayBuffer();
    if (worker === current) current.postMessage({ input: buffer, target }, [buffer]);
  } catch (cause) { failure(cause instanceof Error ? cause.message : String(cause)); }
});
window.addEventListener('pagehide', () => { stop(); resetDownload(); });

controls();

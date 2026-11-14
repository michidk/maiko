import { detectPlatform } from './platform.js';
import { STAMP_SIZE, versionStamp } from './stamp.js';

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
const v5Fixes = document.querySelector('#v5-fixes');
const v5Option = document.querySelector('#v5-option');
const v5Included = document.querySelector('#v5-included');
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
  document.querySelector('#browser-instructions').hidden = selectedTarget() === 'macos';
  document.querySelector('#windows-instructions').hidden = selectedTarget() === 'linux';
  document.querySelector('#linux-instructions').hidden = selectedTarget() !== 'linux';
  document.querySelector('#macos-instructions').hidden = selectedTarget() !== 'macos';
  file.disabled = busy;
  v5Fixes.disabled = busy;
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
// The size and the version stamp at the end of the file are read locally; nothing is sent anywhere.
async function describe(chosen) {
  const size = `${(chosen.size / 1024 / 1024).toFixed(1)} MB`;
  let stamp = null;
  try { stamp = versionStamp(new Uint8Array(await chosen.slice(-STAMP_SIZE).arrayBuffer())); } catch { stamp = null; }
  if (file.files?.[0] !== chosen) return;
  dzDetail.textContent = stamp ? `${size} · version ${stamp.version}, build ${stamp.build} · stays on this device` : `${size} · stays on this device`;
  // Build 5.0 already contains the fixes, so the option is only offered for older builds.
  const included = (stamp?.buildMajor ?? 0) >= 5;
  v5Option.hidden = included;
  v5Included.hidden = !included;
  if (included) v5Fixes.checked = false;
}
function changed() {
  resetDownload();
  clearLog();
  const chosen = file.files?.[0];
  dzName.textContent = chosen ? chosen.name : 'Choose RGFX.EXE or drop it here';
  dzDetail.textContent = chosen ? `${(chosen.size / 1024 / 1024).toFixed(1)} MB · stays on this device` : 'Only this one file is needed.';
  dropzone.classList.toggle('has-file', Boolean(chosen));
  v5Option.hidden = false;
  v5Included.hidden = true;
  if (chosen) describe(chosen);
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
    if (v5Fixes.checked) progressLine('Version 5 fixes selected.');
    current.onerror = event => failure(`The build worker stopped: ${event.message || 'reload the page and try again.'}`);
    current.onmessage = ({ data }) => {
      if (worker !== current) return;
      if (data.kind === 'progress') progressLine(data.message);
      else if (data.kind === 'error') failure(data.message);
      else if (data.kind === 'complete') {
        downloadUrl = URL.createObjectURL(new Blob([data.buffer], { type: 'application/zip' }));
        download.href = downloadUrl;
        download.download = `maiko-${target}.zip`;
        download.hidden = false;
        stop();
        progressLine('Your package is ready. The download starts now; use the link if it does not.');
        download.focus();
        download.click();
      }
    };
    const buffer = await input.arrayBuffer();
    if (worker === current) current.postMessage({ input: buffer, target, v5Fixes: v5Fixes.checked }, [buffer]);
  } catch (cause) { failure(cause instanceof Error ? cause.message : String(cause)); }
});
window.addEventListener('pagehide', () => { stop(); resetDownload(); });

controls();

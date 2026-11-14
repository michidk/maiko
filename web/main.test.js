import { expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

test('Mac detection builds locally, supports cancellation and downloads a universal ZIP', async () => {
  const globals = ['document', 'navigator', 'window', 'Worker'].map(name => [name, Object.getOwnPropertyDescriptor(globalThis, name)]);
  class Element extends EventTarget {
    checked = false;
    hidden = false;
    disabled = false;
    files = [];
    textContent = "";
    scrollHeight = 100;
    scrollTop = 0;
    clientHeight = 100;
    classList = { toggle() {}, add() {}, remove() {} };
    removeAttribute() {}
    focus() {}
    clicks = 0;
    click() { this.clicks++; }
  }
  const html = readFileSync(new URL('./index.html', import.meta.url), 'utf8');
  const ids = new Set([...html.matchAll(/id="([^"]+)"/g)].map(match => `#${match[1]}`));
  const elements = new Map();
  const element = key => {
    if (!elements.has(key)) elements.set(key, new Element());
    return elements.get(key);
  };
  const radios = ['windows', 'linux', 'macos'].map(value => Object.assign(new Element(), { value }));
  const document = {
    querySelector(selector) {
      if (selector === 'input[name="target"]:checked') return radios.find(input => input.checked);
      if (selector.startsWith('input[value=')) return radios.find(input => selector === `input[value="${input.value}"]`);
      return ids.has(selector) ? element(selector) : null;
    },
  };
  const workers = [];
  class Worker {
    constructor(url) { this.url = url; workers.push(this); }
    postMessage(data) { this.message = data; }
    terminate() { this.terminated = true; }
  }
  try {
    Object.defineProperties(globalThis, {
      document: { configurable: true, value: document },
      navigator: { configurable: true, value: { platform: 'MacIntel', userAgent: 'Macintosh', maxTouchPoints: 0 } },
      window: { configurable: true, value: new EventTarget() },
      Worker: { configurable: true, value: Worker },
    });
    await import('./main.js');
    expect(element('#detection').textContent).toBe('macOS selected automatically.');
    expect(element('#macos-guidance').hidden).toBe(false);
    expect(element('#browser-build').hidden).toBe(false);
    expect(element('#browser-instructions').hidden).toBe(true);
    expect(element('#macos-instructions').hidden).toBe(false);
    expect(element('#executable').disabled).toBe(false);
    expect(element('#build').disabled).toBe(true);
    let reads = 0;
    element('#executable').files = [{ size: 4, async arrayBuffer() { reads++; return new ArrayBuffer(4); } }];
    element('#executable').dispatchEvent(new Event('change'));
    expect(element('#build').disabled).toBe(false);
    element('#build').dispatchEvent(new Event('click'));
    await Promise.resolve();
    expect(reads).toBe(1);
    expect(workers[0].message.target).toBe('macos');
    expect(workers[0].message.input.byteLength).toBe(4);
    workers[0].onmessage({ data: { kind: 'progress', message: 'Optimizing the Mac engine…' } });
    expect(element('#log').textContent).toContain('Optimizing the Mac engine…');
    expect(element('#log-panel').hidden).toBe(false);
    expect(element('#cancel').hidden).toBe(false);
    expect(element('#executable').disabled).toBe(true);
    element('#cancel').dispatchEvent(new Event('click'));
    expect(workers[0].terminated).toBe(true);
    expect(element('#status').textContent).toBe('Build cancelled.');
    element('#build').dispatchEvent(new Event('click'));
    await Promise.resolve();
    workers[1].onmessage({ data: { kind: 'complete', buffer: new ArrayBuffer(4) } });
    expect(element('#download').hidden).toBe(false);
    expect(element('#download').download).toBe('maiko-macos-universal.zip');
    expect(element('#download').clicks).toBe(1);
    expect(element('#status').textContent).toContain('Your package is ready.');
    for (const radio of radios) radio.checked = radio.value === 'windows';
    element('#platforms').dispatchEvent(new Event('change'));
    expect(element('#download').hidden).toBe(true);
    expect(element('#macos-guidance').hidden).toBe(true);
    expect(element('#macos-instructions').hidden).toBe(true);
    expect(element('#browser-instructions').hidden).toBe(false);
    expect(element('#build').disabled).toBe(false);
  } finally {
    for (const [name, descriptor] of globals) {
      if (descriptor) Object.defineProperty(globalThis, name, descriptor);
      else delete globalThis[name];
    }
  }
});

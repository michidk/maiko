import { expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

test('Mac detection shows guidance, blocks browser builds and allows manual Windows selection', async () => {
  const globals = ['document', 'navigator', 'window'].map(name => [name, Object.getOwnPropertyDescriptor(globalThis, name)]);
  class Element extends EventTarget {
    checked = false;
    hidden = false;
    disabled = false;
    files = [];
    removeAttribute() {}
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
  try {
    Object.defineProperties(globalThis, {
      document: { configurable: true, value: document },
      navigator: { configurable: true, value: { platform: 'MacIntel', userAgent: 'Macintosh', maxTouchPoints: 0 } },
      window: { configurable: true, value: new EventTarget() },
    });
    await import('./main.js');
    expect(element('#detection').textContent).toBe('macOS selected automatically.');
    expect(element('#macos-guidance').hidden).toBe(false);
    expect(element('#browser-build').hidden).toBe(true);
    expect(element('#browser-instructions').hidden).toBe(true);
    expect(element('#build').disabled).toBe(true);
    let read = false;
    element('#executable').files = [{ arrayBuffer() { read = true; throw new Error('Mac input should not be read'); } }];
    element('#build').dispatchEvent(new Event('click'));
    expect(read).toBe(false);
    for (const radio of radios) radio.checked = radio.value === 'windows';
    element('#platforms').dispatchEvent(new Event('change'));
    expect(element('#macos-guidance').hidden).toBe(true);
    expect(element('#browser-build').hidden).toBe(false);
    expect(element('#browser-instructions').hidden).toBe(false);
    expect(element('#executable').disabled).toBe(false);
    expect(element('#build').disabled).toBe(false);
  } finally {
    for (const [name, descriptor] of globals) {
      if (descriptor) Object.defineProperty(globalThis, name, descriptor);
      else delete globalThis[name];
    }
  }
});

import { expect, test } from 'bun:test';
import { readFile } from 'node:fs/promises';
import { injectHead } from './head.js';

const page = await readFile(new URL('./index.html', import.meta.url), 'utf8');

test('an unconfigured build preserves the source page', async () => {
  expect(await injectHead(page)).toBe(page);
  expect(await injectHead(page, '  ')).toBe(page);
});

test('deployment scripts are injected inside head and permitted by CSP', async () => {
  const snippet = '<script defer src="https://metrics.example.test/script" data-website-id="test"></script>';
  const html = await injectHead(page, snippet);
  expect(html.indexOf(snippet)).toBeLessThan(html.indexOf('</head>'));
  expect(html).toContain("script-src 'self' 'wasm-unsafe-eval' https://metrics.example.test;");
  expect(html).toContain("connect-src 'self' https://metrics.example.test;");
  expect(html).toContain("object-src 'none'");
  expect(page).not.toContain(snippet);
});

test('insecure external scripts fail the build', async () => {
  expect(injectHead(page, '<script src="http://metrics.example.test/script"></script>')).rejects.toThrow('HTTPS');
});

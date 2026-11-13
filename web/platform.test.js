import { test, expect } from 'bun:test';
import { detectPlatform } from './platform.js';

test('detects desktop systems and leaves unsupported systems unselected', () => {
  expect(detectPlatform('Win32')).toBe('windows');
  expect(detectPlatform('Windows')).toBe('windows');
  expect(detectPlatform('Linux x86_64')).toBe('linux');
  for (const [platform, agent] of [['Linux', 'Android'], ['MacIntel', 'iPad'], ['MacIntel', 'Macintosh'], ['', ''], ['iPhone', '']]) expect(detectPlatform(platform, agent)).toBeNull();
});

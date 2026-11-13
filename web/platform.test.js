import { test, expect } from 'bun:test';
import { detectPlatform } from './platform.js';

test('detects desktop systems including macOS browser identifiers', () => {
  expect(detectPlatform('Win32')).toBe('windows');
  expect(detectPlatform('Windows')).toBe('windows');
  expect(detectPlatform('Linux x86_64')).toBe('linux');
  for (const [platform, agent] of [
    ['MacIntel', 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)'],
    ['macOS', ''],
    ['', 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)'],
  ]) expect(detectPlatform(platform, agent)).toBe('macos');
});

test('leaves mobile and unknown systems unselected, including iPad desktop mode', () => {
  for (const [platform, agent, touch] of [
    ['Linux', 'Android', 5],
    ['MacIntel', 'iPad', 5],
    ['MacIntel', 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)', 5],
    ['', '', 0],
    ['iPhone', '', 5],
  ]) expect(detectPlatform(platform, agent, touch)).toBeNull();
});

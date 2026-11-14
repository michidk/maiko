import { test, expect } from 'bun:test';
import { versionStamp } from './stamp.js';

const stamp = (...values) => {
  const bytes = new Uint8Array(40);
  bytes.set([...'XVER'].map(c => c.charCodeAt(0)), 12);
  const view = new DataView(bytes.buffer);
  values.forEach((value, index) => view.setUint32(16 + index * 4, value, true));
  return bytes;
};

test('reads the retail and demo version stamps', () => {
  expect(versionStamp(stamp(1, 0, 1, 0, 0, 0))).toEqual({ version: '1.0', build: '1.0', buildMajor: 1 });
  expect(versionStamp(stamp(1, 0, 5, 0, 0, 0))).toEqual({ version: '1.0', build: '5.0', buildMajor: 5 });
});

test('returns null without a stamp', () => {
  expect(versionStamp(new Uint8Array(40))).toBeNull();
  expect(versionStamp(new Uint8Array(10))).toBeNull();
});

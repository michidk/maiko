import { test, expect } from 'bun:test';
import { wasi } from '@bjorn3/browser_wasi_shim';
import { BufferedFile } from './wasi.js';

test('compiler output supports appends, seeks and overwrites without losing bytes', () => {
  const file = new BufferedFile([]);
  const { fd_obj: fd } = file.path_open(0, 0n, 0);
  for (let index = 0; index < 2000; index++) fd.fd_write(new Uint8Array([index % 256]));
  fd.fd_seek(100n, wasi.WHENCE_SET);
  fd.fd_write(new Uint8Array([7, 8, 9]));
  expect(file.size).toBe(2000n);
  const expected = Uint8Array.from({ length: 2000 }, (_, index) => index % 256);
  expected.set([7, 8, 9], 100);
  expect(file.data).toEqual(expected);
  const { fd_obj: truncated } = file.path_open(wasi.OFLAGS_TRUNC, 0n, 0);
  truncated.fd_write(new Uint8Array([1, 2, 3]));
  expect(file.data).toEqual(new Uint8Array([1, 2, 3]));
});

test('refuses an excessive or invalid compiler output size', () => {
  const file = new BufferedFile([]);
  expect(() => file.write(new Uint8Array([1]), 256 * 1024 * 1024)).toThrow('limit');
  expect(() => file.write(new Uint8Array([1]), Number.MAX_SAFE_INTEGER)).toThrow('limit');
});

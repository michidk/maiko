import { WASI, File, OpenFile, ConsoleStdout, PreopenDirectory, Directory } from '@bjorn3/browser_wasi_shim';

// SR and NASM write incrementally. The shim's default files copy all existing
// bytes on each append. Doubling capacity keeps writes linear in output size.
export class BufferedFile extends File {
  get data() { return this.buffer.subarray(0, this.length); }
  set data(value) { this.buffer = new Uint8Array(value); this.length = value.length; }
  write(data, offset) {
    const end = offset + data.length;
    if (offset < 0 || !Number.isSafeInteger(end) || end > 256 * 1024 * 1024) throw new Error('A compiler output exceeded the 256 MiB limit.');
    if (end > this.buffer.length) {
      const buffer = new Uint8Array(Math.max(end, 4096, this.buffer.length * 2));
      buffer.set(this.data);
      this.buffer = buffer;
    }
    this.buffer.set(data, offset);
    this.length = Math.max(this.length, end);
  }
  path_open(...args) {
    const result = super.path_open(...args);
    if (result.fd_obj) {
      const opened = new BufferedOpenFile(this);
      opened.file_pos = result.fd_obj.file_pos;
      result.fd_obj = opened;
    }
    return result;
  }
}
class BufferedOpenFile extends OpenFile {
  fd_write(data) {
    const result = this.fd_pwrite(data, this.file_pos);
    this.file_pos += BigInt(result.nwritten);
    return result;
  }
  fd_pwrite(data, offset) {
    this.file.write(data, Number(offset));
    return { ret: 0, nwritten: data.length };
  }
}

export async function runWasi(module, args, files, outputs, log = () => {}) {
  const contents = new Map();
  for (const [name, bytes] of Object.entries(files)) {
    const parts = name.split('/');
    let directory = contents;
    const basename = parts.pop();
    for (const part of parts) {
      if (!directory.has(part)) directory.set(part, new Directory(new Map()));
      directory = directory.get(part).contents;
    }
    directory.set(basename, new BufferedFile(bytes));
  }
  for (const name of outputs) if (!contents.has(name)) contents.set(name, new BufferedFile([]));
  const root = new PreopenDirectory('.', contents);
  const wasi = new WASI(args, [], [
    new OpenFile(new File([])), ConsoleStdout.lineBuffered(log), ConsoleStdout.lineBuffered(log), root,
  ], { debug: false });
  const instance = await WebAssembly.instantiate(module, { wasi_snapshot_preview1: wasi.wasiImport });
  const code = wasi.start(instance);
  if (code !== 0) throw new Error(`${args[0]} failed with exit code ${code}.`);
  const result = {};
  function collect(directory, prefix = '') {
    for (const [name, inode] of directory) {
      if (inode instanceof Directory) collect(inode.contents, `${prefix}${name}/`);
      else if (inode instanceof File) result[`${prefix}${name}`] = inode.data;
    }
  }
  collect(contents);
  return result;
}

export async function runLlasm(builder, files) {
  const encoder = new TextEncoder();
  const inputs = Object.entries(files).filter(([name]) => /\.(?:llasm|llinc|bin)$/.test(name));
  const chunks = [encoder.encode(`${inputs.length}\n`)];
  let size = chunks[0].length;
  for (const [path, bytes] of inputs) {
    const name = path === 'rgfx.llasm' ? 'test.llasm' : path;
    if (!/^[a-zA-Z0-9_.+-]+$/.test(name)) throw new Error('Invalid LLasm input path.');
    const header = encoder.encode(`${name}\n${bytes.length}\n`);
    chunks.push(header, bytes);
    size += header.length + bytes.length;
  }
  if (size > 128 * 1024 * 1024) throw new Error('The LLasm input exceeds its memory limit.');
  const input = new Uint8Array(size);
  let offset = 0;
  for (const bytes of chunks) { input.set(bytes, offset); offset += bytes.length; }
  const moduleUrl = new URL('./dlang.js', import.meta.url);
  moduleUrl.search = new URL(import.meta.url).search;
  const { compile } = await import(moduleUrl.href);
  const result = await compile(new TextDecoder().decode(builder.asset('llasm.d')), input, builder.tool('dmd'));
  if (result.exitCode !== 0 || result.errors.length || result.stderr) {
    throw new Error(`LLasm failed: ${[...result.errors, result.stderr].filter(Boolean).join('\n') || `exit code ${result.exitCode}`}`);
  }
  if (!result.stdout.trimStart().startsWith('target datalayout = ') || result.stdout.length > 256 * 1024 * 1024) throw new Error('LLasm did not produce valid LLVM input.');
  return encoder.encode(result.stdout);
}

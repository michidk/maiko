// RGFX.EXE ends with a 28-byte version stamp: "XVER" and six little-endian u32 values,
// the game version pair and the build pair first. The title screen shows them as
// "Version 1.0.1.0". https://michidk.github.io/redguard-preservation/engine/startup.html#version-stamp
export const STAMP_SIZE = 28;

export function versionStamp(tail) {
  if (tail.length < STAMP_SIZE) return null;
  const stamp = tail.subarray(tail.length - STAMP_SIZE);
  if (String.fromCharCode(...stamp.subarray(0, 4)) !== 'XVER') return null;
  const view = new DataView(stamp.buffer, stamp.byteOffset, STAMP_SIZE);
  const [major, minor, buildMajor, buildMinor] = [4, 8, 12, 16].map(offset => view.getUint32(offset, true));
  return { version: `${major}.${minor}`, build: `${buildMajor}.${buildMinor}`, buildMajor };
}

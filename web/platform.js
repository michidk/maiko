export function detectPlatform(platform, userAgent = '') {
  const value = `${platform} ${userAgent}`;
  if (/Android|iPhone|iPad|iPod/i.test(value)) return null;
  if (/Windows|Win32|Win64/i.test(value)) return 'windows';
  if (/Linux/i.test(value)) return 'linux';
  return null;
}

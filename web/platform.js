export function detectPlatform(platform, userAgent = '', maxTouchPoints = 0) {
  const value = `${platform} ${userAgent}`;
  if (/Android|iPhone|iPad|iPod/i.test(value)) return null;
  // iPadOS desktop browsing reports MacIntel and a Macintosh user agent.
  if (/Mac/i.test(value) && maxTouchPoints > 1) return null;
  if (/Windows|Win32|Win64/i.test(value)) return 'windows';
  if (/Macintosh|MacIntel|MacPPC|macOS|Mac OS/i.test(value)) return 'macos';
  if (/Linux/i.test(value)) return 'linux';
  return null;
}

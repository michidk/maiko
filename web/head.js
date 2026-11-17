// Deployment-owned HTML stays in Actions variables rather than the source page.
// External scripts also need permission to load and send requests under our CSP.
export async function injectHead(html, extraHead = '') {
  if (!extraHead.trim()) return html;
  const origins = new Set();
  await new HTMLRewriter().on('script[src]', {
    element(element) {
      const src = element.getAttribute('src');
      const url = new URL(src, 'https://localhost');
      if (url.origin === 'https://localhost') return;
      if (url.protocol !== 'https:') throw new Error('Injected scripts must use HTTPS');
      origins.add(url.origin);
    },
  }).transform(new Response(extraHead)).text();
  return new HTMLRewriter()
    .on('meta[http-equiv="Content-Security-Policy"]', {
      element(element) {
        const policy = element.getAttribute('content').split(';').map(directive => {
          const value = directive.trim();
          return /^(script-src|connect-src)\s/.test(value)
            ? [value, ...origins].join(' ') : value;
        }).join('; ');
        element.setAttribute('content', policy);
      },
    })
    .on('head', { element(element) { element.append(`\n${extraHead}\n`, { html: true }); } })
    .transform(new Response(html)).text();
}

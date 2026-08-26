export function recoveryDocumentUrl(originalUrl: string): string {
  const serializedOriginalUrl = JSON.stringify(originalUrl).replace(/</g, "\\u003c");
  const recoveryHtml = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>OrkWorks unavailable</title>
    <style>
      :root { color-scheme: dark; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
      body { min-height: 100vh; margin: 0; display: grid; place-items: center; background: #0c0d10; color: #eceef1; }
      main { width: min(420px, calc(100vw - 48px)); padding: 32px; border: 1px solid #5a2b29; border-radius: 12px; background: #111319; text-align: center; box-sizing: border-box; }
      p { color: #8a909c; line-height: 1.5; }
      button { margin-top: 16px; padding: 9px 18px; border: 1px solid #9dc520; border-radius: 8px; color: #0c0d10; background: #9dc520; font: inherit; font-weight: 600; cursor: pointer; }
      button:hover { background: #b4dd33; }
    </style>
  </head>
  <body><main><h1>OrkWorks is unavailable</h1><p>The application window could not load. Retry to open the application again.</p><button type="button" onclick="location.replace(originalUrl)">Retry</button></main><script>const originalUrl = ${serializedOriginalUrl};</script></body>
</html>`;
  return `data:text/html;charset=utf-8,${encodeURIComponent(recoveryHtml)}`;
}

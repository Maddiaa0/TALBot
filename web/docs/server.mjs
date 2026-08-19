import { INTERNAL_runFetch, unstable_serverEntry } from "./dist/server/index.js";

const { serve } = unstable_serverEntry;
const port = Number.parseInt(process.env.PORT ?? "3000", 10);

if (!Number.isSafeInteger(port) || port < 1 || port > 65_535) {
  throw new Error("PORT must be an integer between 1 and 65535");
}

const securityHeaders = {
  "Content-Security-Policy": [
    "default-src 'self'",
    "base-uri 'self'",
    "connect-src 'self'",
    "font-src 'self'",
    "form-action 'none'",
    "frame-ancestors 'none'",
    "img-src 'self' data:",
    "object-src 'none'",
    "script-src 'self' 'unsafe-inline' 'unsafe-eval'",
    "style-src 'self' 'unsafe-inline'",
  ].join("; "),
  "Cross-Origin-Resource-Policy": "same-origin",
  "Permissions-Policy": "camera=(), geolocation=(), microphone=()",
  "Referrer-Policy": "strict-origin-when-cross-origin",
  "Strict-Transport-Security": "max-age=31536000",
  "X-Content-Type-Options": "nosniff",
  "X-Frame-Options": "DENY",
};

serve({
  fetch: async (request, ...context) => {
    const response = await INTERNAL_runFetch(
      process.env,
      request,
      ...context,
    );
    const headers = new Headers(response.headers);
    const { pathname } = new URL(request.url);

    for (const [name, value] of Object.entries(securityHeaders)) {
      headers.set(name, value);
    }
    headers.set("Vary", "Accept, User-Agent");

    if (pathname.endsWith("/SKILL.md")) {
      headers.set("Content-Type", "text/markdown; charset=utf-8");
    }

    return new Response(response.body, {
      headers,
      status: response.status,
      statusText: response.statusText,
    });
  },
  hostname: process.env.HOST,
  port,
});

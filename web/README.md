# TALBot web workspace

This workspace contains the public web applications for TALBot. The landing
page is currently the only application; the documentation site will be added
after the landing design is approved.

## Commands

Run commands from this directory:

```sh
corepack enable
pnpm install --frozen-lockfile
pnpm dev       # development server
pnpm check     # Astro and TypeScript diagnostics
pnpm build     # static production build
PORT=3000 pnpm start  # serve the production build locally
```

Dependencies and the pnpm version are pinned. Newly published package versions
are quarantined for 14 days by `.npmrc`, and `esbuild` is the only dependency
permitted to run an installation script.

## Railway

Create a Railway service connected to this repository with:

- Root directory: `/web`
- Build command: `pnpm --filter @talbot/landing build`
- Start command: `pnpm --filter @talbot/landing start`
- Custom domain: `talbot.maddiaa.com`
- Watch paths: `/web/landing/**`, `/web/.npmrc`, `/web/package.json`,
  `/web/pnpm-lock.yaml`, and `/web/pnpm-workspace.yaml`

The service must receive no application secrets. The landing page has no
server-side behavior and sends no analytics or other third-party requests.
DNS and TLS provisioning remain owned by the separate infrastructure project.

## Security boundary

Everything under `web` is public at source or build time. In particular,
Astro variables prefixed with `PUBLIC_` are deliberately exposed to browsers.
Do not use them for tokens, credentials, private endpoints, or sensitive
configuration.

Production response headers are defined in `landing/public/serve.json`. Keep
the Content Security Policy synchronized with any intentional new browser
capabilities; do not loosen it to work around an unexpected build change.

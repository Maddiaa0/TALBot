# TALBot web workspace

This workspace contains the public TALBot landing page and documentation site.
They are separate applications so Railway can deploy them independently from
the same repository.

## Commands

Run commands from this directory:

```sh
corepack enable
pnpm install --frozen-lockfile
pnpm dev             # landing development server
pnpm dev:docs        # docs development server
pnpm check           # landing diagnostics and agent-Markdown audit
pnpm build           # build both applications
PORT=3000 pnpm start       # serve the landing build locally
PORT=3000 pnpm start:docs  # serve the docs build locally
```

Dependencies and the pnpm version are pinned. Newly published package versions
are quarantined for 14 days by `.npmrc`, and `esbuild` is the only dependency
permitted to run an installation script.

## Railway

### Landing service

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

### Docs service

Create a second service from the same repository with:

- Root directory: `/web`
- Build command: `pnpm --filter @talbot/docs build`
- Start command: `pnpm --filter @talbot/docs start`
- Service variable: `PORT=8080`
- Custom domain: `docs.talbot.maddiaa.com`, target port `8080`
- Watch paths: `/web/docs/**`, `/web/.npmrc`, `/web/package.json`,
  `/web/pnpm-lock.yaml`, and `/web/pnpm-workspace.yaml`

The docs service uses a Node server because Markdown content negotiation and
the read-only documentation MCP endpoint are dynamic. It receives no Telegram
or GitHub credentials. The production server adds restrictive browser security
headers to every response; the generated runtime is self-contained and does not
load the Vocs build toolchain.

## Security boundary

Everything under `web` is public at source or build time. Do not use it for
tokens, credentials, private endpoints, or sensitive configuration.

Production response headers are defined in `landing/public/serve.json`. Keep
the Content Security Policy synchronized with any intentional new browser
capabilities; do not loosen it to work around an unexpected build change.

The docs MCP server exposes public documentation only. Do not add source
adapters, feedback tools, authenticated repositories, or application secrets
without treating that as a new public security capability.

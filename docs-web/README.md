# docs-web

Herald documentation site. Built with [Fumadocs](https://fumadocs.dev) on TanStack Start (React + Vite), with English / Chinese i18n and Mermaid diagram support.

## Develop

```bash
npm install
npm run dev
```

Open http://localhost:3001 — `/` redirects to `/en`.

- Source content lives in `content/docs/` (English) and `content/docs/zh/` (Chinese), as MDX.
- Sidebar ordering is controlled by `content/docs/meta.json` and `content/docs/zh/meta.json`.

## Build

```bash
npm run build
```

Produces static output in `.output/public/` (SPA prerender) plus a generated `sitemap.xml`.

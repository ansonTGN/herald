import { readFileSync } from "node:fs";
import tailwindcss from "@tailwindcss/vite";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import react from "@vitejs/plugin-react";
import mdx from "fumadocs-mdx/vite";
import { nitro } from "nitro/vite";
import { defineConfig } from "vite";

// OpenAPI pages are virtual (one per operationId in the spec, grouped by tag).
// The prerender crawler can't discover them from virtual pages, so we list them
// explicitly — one per operationId, in both locales. Generated from openapi.json
// so it stays in sync when the backend adds/removes endpoints.
function openapiSlugs(): string[] {
  try {
    const spec = JSON.parse(readFileSync("./openapi.json", "utf-8")) as {
      paths?: Record<string, Record<string, { operationId?: string }>>;
    };
    const ids = new Set<string>();
    for (const ops of Object.values(spec.paths ?? {})) {
      for (const op of Object.values(ops)) {
        if (op.operationId) ids.add(op.operationId);
      }
    }
    return [...ids].sort();
  } catch {
    return [];
  }
}

function openapiPages() {
  return openapiSlugs().flatMap((slug) => [
    { path: `/en/docs/openapi/${slug}` },
    { path: `/zh/docs/openapi/${slug}` },
  ]);
}

export default defineConfig({
  server: {
    port: 3001,
  },
  plugins: [
    mdx(),
    tailwindcss(),
    tanstackStart({
      spa: {
        enabled: true,
        prerender: {
          enabled: true,
          crawlLinks: true,
        },
      },

      pages: [
        {
          path: "/docs",
        },
        {
          path: "/en/docs",
        },
        {
          path: "/zh/docs",
        },
        // Locale root pages
        {
          path: "/en",
        },
        {
          path: "/zh",
        },
        // Legal pages
        {
          path: "/en/privacy",
        },
        {
          path: "/zh/privacy",
        },
        {
          path: "/en/terms",
        },
        {
          path: "/zh/terms",
        },
        // OpenAPI tag pages — virtual, must be listed explicitly (crawler can't render them)
        ...openapiPages(),
        // Nested folder pages (sidebar-only links aren't auto-discovered by the crawler)
        {
          path: "/en/docs/points-grant-redesign",
        },
        {
          path: "/en/docs/points-grant-redesign/getting-started",
        },
        {
          path: "/en/docs/points-grant-redesign/architecture",
        },
        {
          path: "/zh/docs/points-grant-redesign",
        },
        {
          path: "/zh/docs/points-grant-redesign/getting-started",
        },
        {
          path: "/zh/docs/points-grant-redesign/architecture",
        },
        {
          path: "/api/search",
        },
        {
          path: "llms-full.txt",
        },
        {
          path: "llms.txt",
        },
      ],
    }),
    react(),
    // please see https://tanstack.com/start/latest/docs/framework/react/guide/hosting#nitro for guides on hosting
    nitro(),
  ],
  resolve: {
    tsconfigPaths: true,
  },
});

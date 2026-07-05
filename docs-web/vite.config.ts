import tailwindcss from "@tailwindcss/vite";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import react from "@vitejs/plugin-react";
import mdx from "fumadocs-mdx/vite";
import { nitro } from "nitro/vite";
import { defineConfig } from "vite";

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
          path: "/en/docs/points-grant-redesign/api-reference",
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
          path: "/zh/docs/points-grant-redesign/api-reference",
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

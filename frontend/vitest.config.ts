import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import { tanstackRouter } from '@tanstack/router-plugin/vite'
import tailwindcss from '@tailwindcss/vite'
import { paraglideVitePlugin } from '@inlang/paraglide-js'
import path from 'path'

/**
 * Vitest Configuration
 *
 * This file configures Vitest for frontend testing with:
 * - JSDOM environment for fast, isolated testing
 * - MSW for API mocking
 * - React and TanStack Router plugins
 * - Tailwind CSS for styling
 */
export default defineConfig({
  plugins: [
    paraglideVitePlugin({
      project: './project.inlang',
      outdir: './src/paraglide',
      strategy: ['localStorage', 'baseLocale'],
      localStorageKey: 'herald-locale',
      emitTsDeclarations: true,
    }),
    tailwindcss(),
    tanstackRouter({
      target: 'react',
      autoCodeSplitting: true,
      routeFileIgnorePattern: '__tests__',
    }),
    react(),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    // JSDOM environment for fast, isolated testing
    environment: 'jsdom',

    // Ensure proper test isolation
    isolate: true,

    // Enable global test utilities
    globals: true,

    // Minimal reporter: only failed tests and errors, optimized for AI agents
    reporters: ['minimal'],

    // Reduce timeout (no browser overhead)
    testTimeout: 5000,

    // Don't fail tests on unhandled promise rejections (handled by try-catch in components)
    errorOnUnhandledRejections: false,

    // Ensure test files are correctly resolved
    include: ['**/__tests__/**/*.{test,spec}.{js,jsx,ts,tsx}'],
    exclude: [
      '**/node_modules/**',
      '**/dist/**',
      '**/tests/e2e/**',
      '**/demo/**',
      '**/.git/**',
      '**/.vscode/**',
    ],
    // Setup file for MSW and global utilities
    setupFiles: ['./src/test/setup.ts'],
    // Optimize dependency pre-bundling
    optimizeDeps: {
      include: ['class-variance-authority', 'clsx', 'tailwind-merge', 'react', 'react-dom'],
    },
    // Coverage configuration
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/',
        'src/test/',
        '**/__tests__/',
        '*.config.{js,ts}',
        'src/main.tsx',
        'src/vite-env.d.ts',
        '**/*.d.ts',
      ],
    },
  },
})

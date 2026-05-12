/**
 * Global type declarations
 *
 * Extends the global Window interface with custom properties used for
 * debugging and testing.
 */

import type { QueryClient } from '@tanstack/react-query'
import type { Router } from '@tanstack/react-router'

declare global {
  interface Window {
    /**
     * React Router instance exposed for debugging in development
     *
     * @see frontend/src/main.tsx
     */
    router?: Router

    /**
     * React Query Client instance exposed for test fixtures
     *
     * Used by demo/e2e/fixtures/demo-page.fixtures.ts to clear cache
     * between tests to prevent authentication issues from stale data.
     *
     * @see frontend/src/main.tsx
     */
    __REACT_QUERY_CLIENT__?: QueryClient
  }
}

export {}

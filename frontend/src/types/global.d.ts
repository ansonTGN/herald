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

    /**
     * Google Identity Services (GIS) client, populated once the
     * `https://accounts.google.com/gsi/client` script finishes loading. Only
     * the `accounts.id.*` One Tap surface is declared here; other GIS APIs
     * (oauth2, oauth) are out of scope.
     *
     * @see frontend/src/components/auth/one-tap-login.tsx
     */
    google?: GoogleGsiClient
  }

  /**
   * Minimal subset of the Google Identity Services API consumed by the One Tap
   * integration (`one-tap-login.tsx`). Declared inline rather than depending
   * on `@types/google.accounts` to keep the change surgical. See
   * https://developers.google.com/identity/gsi/web/reference/js-reference.
   */
  interface GoogleGsiClient {
    accounts: {
      id: GoogleAccountsId
    }
  }

  interface GoogleAccountsId {
    initialize(config: {
      client_id: string
      callback: (response: GoogleCredentialResponse) => void
      auto_select?: boolean
      cancel_on_tap_outside?: boolean
      context?: 'signin' | 'signup' | 'use'
    }): void
    prompt(listener?: (notification: GooglePromptNotification) => void): void
    cancel(): void
    // Declared for completeness; the login page only uses prompt().
    renderButton(parent: HTMLElement, options: Record<string, unknown>): void
    disableAutoSelect(): void
  }

  interface GoogleCredentialResponse {
    credential: string
    select_by?: string
  }

  interface GooglePromptNotification {
    isNotDisplayed(): boolean
    isSkippedMoment(): boolean
    isDisplayed(): boolean
    getNotDisplayedReason(): string
    getSkippedReason(): string
  }
}

export {}

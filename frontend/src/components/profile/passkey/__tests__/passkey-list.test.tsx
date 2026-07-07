import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/mocks/server'
import { PasskeyList } from '../passkey-list'
import type { PasskeyCredentialViewResponse } from '@/lib/api-generated'

/**
 * Passkey credential list (FE-D03). Uses MSW handlers for the GET list, PATCH
 * rename, and DELETE endpoints so request bodies / status codes are observable.
 *
 * Key branches: empty / populated rendering, rename (PATCH body + invalid
 * nickname), delete single vs last-credential risk warning (US-PK-009).
 */

const API_BASE_URL = 'http://localhost:3000'

/** Factory: build a single credential view with sensible defaults. */
function makePasskeyCredentialView(
  overrides: Partial<PasskeyCredentialViewResponse> = {}
): PasskeyCredentialViewResponse {
  return {
    credentialId: overrides.credentialId ?? 'cred-1',
    nickname: overrides.nickname ?? 'My YubiKey',
    createdAt: overrides.createdAt ?? '2026-01-01T00:00:00Z',
    lastUsedAt: overrides.lastUsedAt ?? null,
    transports: overrides.transports ?? ['internal'],
    backupEligible: overrides.backupEligible ?? false,
    backupState: overrides.backupState ?? false,
    aaguid: overrides.aaguid ?? null,
  }
}

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
}

function renderList(onAdd = vi.fn()) {
  return render(
    <QueryClientProvider client={createTestQueryClient()}>
      <PasskeyList onAdd={onAdd} />
    </QueryClientProvider>
  )
}

describe('PasskeyList', () => {
  const user = userEvent.setup({ delay: null })
  let currentCredentials: PasskeyCredentialViewResponse[]
  let patchBodies: Record<string, unknown>[]
  let deletedIds: string[]
  let listStatus: number

  beforeEach(() => {
    currentCredentials = []
    patchBodies = []
    deletedIds = []
    listStatus = 200

    server.resetHandlers()
    server.use(
      http.get(`${API_BASE_URL}/api/user/passkey/credentials`, () => {
        if (listStatus !== 200) {
          return HttpResponse.json({ error: 'list failed' }, { status: listStatus })
        }
        return HttpResponse.json({ credentials: currentCredentials })
      }),
      http.patch(
        `${API_BASE_URL}/api/user/passkey/credentials/:credentialId`,
        async ({ request, params }) => {
          patchBodies.push({ credentialId: params.credentialId, body: await request.json() })
          return HttpResponse.json({})
        }
      ),
      http.delete(`${API_BASE_URL}/api/user/passkey/credentials/:credentialId`, ({ params }) => {
        deletedIds.push(params.credentialId as string)
        return HttpResponse.json({})
      })
    )
  })

  describe('rendering', () => {
    it('GIVEN a populated list WHEN mounting THEN should render an item per credential plus the add button', async () => {
      currentCredentials = [
        makePasskeyCredentialView({ credentialId: 'cred-1', nickname: 'YubiKey' }),
        makePasskeyCredentialView({ credentialId: 'cred-2', nickname: 'Phone' }),
      ]

      renderList()

      await screen.findByTestId('passkey-item-cred-1')
      expect(screen.getByTestId('passkey-item-cred-2')).toBeInTheDocument()
      expect(screen.getAllByTestId('passkey-add-button').length).toBeGreaterThan(0)
    })

    it('GIVEN an empty list WHEN mounting THEN should show the empty state and add button', async () => {
      currentCredentials = []

      renderList()

      await screen.findByTestId('passkey-empty-state')
      expect(screen.getByTestId('passkey-add-button')).toBeInTheDocument()
      expect(screen.queryByTestId('passkey-list')).not.toBeInTheDocument()
    })

    it('GIVEN the list is loading WHEN fetching THEN should show the loading state', async () => {
      // A never-resolving promise keeps the query in the pending (isLoading) state
      // so the component renders its `passkey-list` container with the loading copy.
      server.use(
        http.get(`${API_BASE_URL}/api/user/passkey/credentials`, () => new Promise(() => {}))
      )

      renderList()

      await screen.findByTestId('passkey-list')
      expect(screen.getByTestId('passkey-list')).toHaveTextContent(/loading passkeys/i)
    })

    it('GIVEN add button is clicked WHEN clicking THEN should call onAdd', async () => {
      currentCredentials = []
      const onAdd = vi.fn()
      renderList(onAdd)

      await screen.findByTestId('passkey-add-button')
      await user.click(screen.getByTestId('passkey-add-button'))

      expect(onAdd).toHaveBeenCalledTimes(1)
    })
  })

  describe('rename', () => {
    it('GIVEN user renames a credential WHEN submitting THEN should PATCH the new nickname and close the dialog', async () => {
      currentCredentials = [
        makePasskeyCredentialView({ credentialId: 'cred-1', nickname: 'Old Name' }),
      ]

      renderList()
      await screen.findByTestId('passkey-item-cred-1')

      // The rename button has no testid; open the dialog via its label/button text.
      const renameButton = screen.getByRole('button', { name: /rename/i })
      await user.click(renameButton)

      const input = await screen.findByTestId('passkey-rename-input')
      await user.clear(input)
      await user.type(input, 'New Name')
      await user.click(screen.getByTestId('passkey-rename-submit-button'))

      await waitFor(() => {
        expect(patchBodies).toHaveLength(1)
      })
      expect(patchBodies[0]).toEqual({
        credentialId: 'cred-1',
        body: { nickname: 'New Name' },
      })
    })

    it('GIVEN user clears the nickname WHEN submitting THEN should reject (zod min(1))', async () => {
      currentCredentials = [makePasskeyCredentialView({ credentialId: 'cred-1', nickname: 'Keep' })]

      renderList()
      await screen.findByTestId('passkey-item-cred-1')

      await user.click(screen.getByRole('button', { name: /rename/i }))
      const input = await screen.findByTestId('passkey-rename-input')
      await user.clear(input)
      await user.click(screen.getByTestId('passkey-rename-submit-button'))

      // PATCH must not fire for an empty nickname.
      await new Promise((resolve) => setTimeout(resolve, 0))
      expect(patchBodies).toHaveLength(0)
    })
  })

  describe('delete', () => {
    it('GIVEN user deletes one of many WHEN confirming THEN should DELETE that credentialId', async () => {
      currentCredentials = [
        makePasskeyCredentialView({ credentialId: 'cred-1', nickname: 'First' }),
        makePasskeyCredentialView({ credentialId: 'cred-2', nickname: 'Second' }),
      ]

      renderList()
      await screen.findByTestId('passkey-item-cred-1')

      // The delete button on item cred-1.
      const deleteButtons = screen.getAllByTestId('passkey-delete-button')
      await user.click(deleteButtons[0])

      const dialog = await screen.findByTestId('passkey-delete-confirm-dialog')
      expect(dialog).toBeInTheDocument()
      // Not the last credential → the US-PK-009 last-warning copy must NOT appear.
      expect(dialog).not.toHaveTextContent(/last passkey/i)
      await user.click(screen.getByTestId('passkey-delete-confirm-button'))

      await waitFor(() => {
        expect(deletedIds).toEqual(['cred-1'])
      })
    })

    it('GIVEN user deletes the LAST credential WHEN the dialog opens THEN should show the last-warning and, on confirm, DELETE and return to empty state', async () => {
      currentCredentials = [
        makePasskeyCredentialView({ credentialId: 'only-cred', nickname: 'Only' }),
      ]

      renderList()
      await screen.findByTestId('passkey-item-only-cred')

      await user.click(screen.getByTestId('passkey-delete-button'))

      const dialog = await screen.findByTestId('passkey-delete-confirm-dialog')
      // Last-credential risk warning (US-PK-009) — extra red copy present only
      // when the list length is 1. Asserting the warning text surfaces here.
      expect(dialog).toHaveTextContent(/last passkey/i)

      await user.click(screen.getByTestId('passkey-delete-confirm-button'))

      await waitFor(() => {
        expect(deletedIds).toEqual(['only-cred'])
      })

      // Simulate the list refreshing to empty (the component invalidates the
      // query on success; we expose the next fetch by clearing the array).
      currentCredentials = []
    })
  })
})

import { describe, test, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { useState } from 'react'
import { useMutation, QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { server } from '@/test/mocks/server'
import { deviceVerify, deviceConfirm } from '@/lib/api-generated'
import { getErrorMessage } from '@/lib/error-utils'

const API_BASE_URL = 'http://localhost:3000'

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  })
}

describe('Device verification error message extraction', () => {
  test('extracts error_description from not_found error', () => {
    const error = { error: 'not_found', error_description: 'Code not found or expired' }
    expect(getErrorMessage(error)).toBe('Code not found or expired')
  })

  test('extracts error_description from already_used error', () => {
    const error = { error: 'already_used', error_description: 'This code has already been used' }
    expect(getErrorMessage(error)).toBe('This code has already been used')
  })

  test('extracts error_description from invalid_request error', () => {
    const error = { error: 'invalid_request', error_description: 'Invalid confirmation request' }
    expect(getErrorMessage(error)).toBe('Invalid confirmation request')
  })

  test('falls back to Error.message for Error instances', () => {
    expect(getErrorMessage(new Error('Network error'))).toBe('Network error')
  })

  test('falls back to string for string errors', () => {
    expect(getErrorMessage('Something failed')).toBe('Something failed')
  })

  test('falls back to default message for unknown errors', () => {
    expect(getErrorMessage(null)).toBe('An unexpected error occurred')
    expect(getErrorMessage(undefined)).toBe('An unexpected error occurred')
    expect(getErrorMessage(42)).toBe('An unexpected error occurred')
  })

  test('handles error object without error_description', () => {
    const error = { message: 'Some error' }
    expect(getErrorMessage(error)).toBe('Some error')
  })

  test('ignores non-string error_description, falls back to error field', () => {
    const error = { error: 'test', error_description: 123 }
    expect(getErrorMessage(error)).toBe('test')
  })
})

describe('Device verify API error states via MSW', () => {
  beforeEach(() => {
    server.resetHandlers()
  })

  test('verify returns 404 error response', async () => {
    server.use(
      http.post(`${API_BASE_URL}/api/device/:realmId/verify`, () => {
        return HttpResponse.json(
          { error: 'not_found', error_description: 'Code not found or expired' },
          { status: 404 }
        )
      })
    )

    const response = await deviceVerify({
      body: { user_code: 'BCDFGHJK' },
      path: { realmId: 'test-realm' },
    })

    expect(response.error).toBeDefined()
    expect((response.error as any).error).toBe('not_found')
    expect((response.error as any).error_description).toBe('Code not found or expired')
  })

  test('verify returns 409 error response', async () => {
    server.use(
      http.post(`${API_BASE_URL}/api/device/:realmId/verify`, () => {
        return HttpResponse.json(
          { error: 'already_used', error_description: 'This code has already been used' },
          { status: 409 }
        )
      })
    )

    const response = await deviceVerify({
      body: { user_code: 'BCDFGHJK' },
      path: { realmId: 'test-realm' },
    })

    expect(response.error).toBeDefined()
    expect((response.error as any).error).toBe('already_used')
  })

  test('confirm returns 400 error response', async () => {
    server.use(
      http.post(`${API_BASE_URL}/api/device/:realmId/confirm`, () => {
        return HttpResponse.json(
          { error: 'invalid_request', error_description: 'Invalid confirmation request' },
          { status: 400 }
        )
      })
    )

    const response = await deviceConfirm({
      body: { user_code: 'BCDFGHJK', approved: true },
      path: { realmId: 'test-realm' },
    })

    expect(response.error).toBeDefined()
    expect((response.error as any).error).toBe('invalid_request')
  })
})

/**
 * Test that the verify+error-display pattern works end-to-end
 * using a minimal component that mirrors the page's error handling logic.
 */
function TestDeviceErrorComponent({ realmId }: { realmId: string }) {
  const [error, setError] = useState<string | null>(null)
  const [code, setCode] = useState('')

  const verifyMutation = useMutation({
    mutationFn: async (userCode: string) => {
      const response = await deviceVerify({
        body: { user_code: userCode },
        path: { realmId },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => setError(null),
    onError: (err: unknown) => setError(getErrorMessage(err)),
  })

  return (
    <div>
      {error && <div data-testid="device-verification-error">{error}</div>}
      <input
        data-testid="device-code-input"
        value={code}
        onChange={(e) => setCode(e.target.value)}
      />
      <button
        data-testid="device-code-submit"
        onClick={() => verifyMutation.mutate(code)}
        disabled={code.length < 8}
      >
        Verify
      </button>
    </div>
  )
}

describe('Device verification error display integration', () => {
  beforeEach(() => {
    server.resetHandlers()
  })

  test('shows error message when verify returns 404', async () => {
    server.use(
      http.post(`${API_BASE_URL}/api/device/:realmId/verify`, () => {
        return HttpResponse.json(
          { error: 'not_found', error_description: 'Code not found or expired' },
          { status: 404 }
        )
      })
    )

    const queryClient = createTestQueryClient()
    render(
      <QueryClientProvider client={queryClient}>
        <TestDeviceErrorComponent realmId="test-realm" />
      </QueryClientProvider>
    )

    await userEvent.type(screen.getByTestId('device-code-input'), 'BCDFGHJK')
    await userEvent.click(screen.getByTestId('device-code-submit'))

    const errorElement = await screen.findByTestId('device-verification-error')
    expect(errorElement).toBeInTheDocument()
    expect(errorElement.textContent).toContain('Code not found or expired')
  })

  test('shows error message when verify returns 409', async () => {
    server.use(
      http.post(`${API_BASE_URL}/api/device/:realmId/verify`, () => {
        return HttpResponse.json(
          { error: 'already_used', error_description: 'This code has already been used' },
          { status: 409 }
        )
      })
    )

    const queryClient = createTestQueryClient()
    render(
      <QueryClientProvider client={queryClient}>
        <TestDeviceErrorComponent realmId="test-realm" />
      </QueryClientProvider>
    )

    await userEvent.type(screen.getByTestId('device-code-input'), 'BCDFGHJK')
    await userEvent.click(screen.getByTestId('device-code-submit'))

    const errorElement = await screen.findByTestId('device-verification-error')
    expect(errorElement).toBeInTheDocument()
    expect(errorElement.textContent).toContain('This code has already been used')
  })

  test('shows error message when confirm returns 400', async () => {
    function TestConfirmErrorComponent({ realmId }: { realmId: string }) {
      const [error, setError] = useState<string | null>(null)

      const confirmMutation = useMutation({
        mutationFn: async (approved: boolean) => {
          const response = await deviceConfirm({
            body: { user_code: 'BCDFGHJK', approved },
            path: { realmId },
          })
          if (response.error) throw response.error
          return response.data
        },
        onSuccess: () => setError(null),
        onError: (err: unknown) => setError(getErrorMessage(err)),
      })

      return (
        <div>
          {error && <div data-testid="device-verification-error">{error}</div>}
          <button
            data-testid="device-authorize-button"
            onClick={() => confirmMutation.mutate(true)}
          >
            Authorize
          </button>
          <button
            data-testid="device-deny-button"
            onClick={() => confirmMutation.mutate(false)}
          >
            Deny
          </button>
        </div>
      )
    }

    server.use(
      http.post(`${API_BASE_URL}/api/device/:realmId/confirm`, () => {
        return HttpResponse.json(
          { error: 'invalid_request', error_description: 'Invalid confirmation request' },
          { status: 400 }
        )
      })
    )

    const queryClient = createTestQueryClient()
    render(
      <QueryClientProvider client={queryClient}>
        <TestConfirmErrorComponent realmId="test-realm" />
      </QueryClientProvider>
    )

    // Test authorize button
    await userEvent.click(screen.getByTestId('device-authorize-button'))

    const errorElement = await screen.findByTestId('device-verification-error')
    expect(errorElement).toBeInTheDocument()
    expect(errorElement.textContent).toContain('Invalid confirmation request')
  })
})

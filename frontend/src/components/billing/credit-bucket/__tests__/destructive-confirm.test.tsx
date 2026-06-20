import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/mocks/server'
import { creditBucketsHandlers } from '@/test/mocks/handlers/credit-buckets'
import { DeleteBucketConfirmDialog } from '../delete-bucket-confirm-dialog'
import { CreditBucketEditor } from '../credit-bucket-editor'
import type { BucketDetailResponse } from '@/lib/api-generated'

// ===== Test helpers =====

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0, staleTime: 0 },
      mutations: { retry: false },
    },
  })
}

function withQueryClient(ui: React.ReactNode) {
  const client = makeQueryClient()
  return render(
    <QueryClientProvider client={client}>{ui}</QueryClientProvider>
  )
}

/**
 * Minimal VALID BucketDetailResponse fixture for the update-mode editor.
 *
 * `clientApps` MUST be non-empty: `updateCreditBucketSchema` enforces
 * `clientAppIds.min(1)`, so an empty coverage set makes the form zod-invalid
 * and submit never fires (no mutation, no 409, no conflict Alert). A real
 * bucket always has at least one covered client app (design §4.2.2).
 */
function makeBucket(overrides: Partial<BucketDetailResponse> = {}): BucketDetailResponse {
  return {
    id: 'b1',
    bucketKey: 'promo-pool',
    name: 'Promo Pool',
    description: null,
    displayOrder: 0,
    enabled: true,
    receivesRegistrationCredits: false,
    clientApps: [{ id: 'app-1' }],
    entitlementMappings: [],
    ...overrides,
  }
}

// ===== Tests =====

describe('Credit Bucket destructive-confirm states', () => {
  describe('DeleteBucketConfirmDialog — 409 bucket_in_use', () => {
    // NOTE (deviation, loud): the item wording suggested driving this state by
    // clicking confirm and letting MSW return the 409. The actual code splits
    // responsibilities: the directory page's confirmDelete() catches the 409
    // and translates it into an `inUseError` PROP handed to this presentational
    // dialog. The dialog itself never calls the mutation. We therefore test the
    // user-observable destructive state directly (the prop-driven branch), which
    // is exactly the surface this item is about — danger alert copy + hidden
    // confirm Action. Exercising the directory-page wiring end-to-end belongs to
    // the Demo slot, not this Vitest.

    it('shows the bucket_in_use reason and hides the destructive confirm action when inUseError is set', () => {
      withQueryClient(
        <DeleteBucketConfirmDialog
          open
          onOpenChange={vi.fn()}
          onConfirm={vi.fn()}
          bucketName="Promo Pool"
          inUseError={{
            code: 'bucket_in_use',
            activeSubscriptions: 2,
            holdersWithBalance: 5,
          }}
        />
      )

      // The dialog's description states the refusal, and a separate destructive
      // Alert carries the counts — both must reach the admin so they understand
      // WHY deletion was refused (design §4.2.3 body fields).
      const dialog = screen.getByTestId('delete-bucket-confirm-dialog')
      expect(dialog).toHaveTextContent(/cannot be deleted/i)
      const alert = screen.getByTestId('delete-bucket-error-message')
      expect(alert).toHaveTextContent('2')
      expect(alert).toHaveTextContent('5')
      expect(alert).toHaveTextContent(/resolve these before deleting/i)

      // Destructive confirm Action is GONE in the blocked state — the admin
      // cannot proceed past the refusal.
      expect(
        screen.queryByTestId('delete-bucket-confirm-button')
      ).not.toBeInTheDocument()
      // Cancel remains available as the exit.
      expect(
        screen.getByTestId('delete-bucket-cancel-button')
      ).toBeInTheDocument()
    })

    it('does not surface the bucket_in_use alert when no inUseError is present', () => {
      withQueryClient(
        <DeleteBucketConfirmDialog
          open
          onOpenChange={vi.fn()}
          onConfirm={vi.fn()}
          bucketName="Promo Pool"
        />
      )

      // Happy-path destructive confirm: confirm action present, no danger alert.
      expect(
        screen.queryByTestId('delete-bucket-error-message')
      ).not.toBeInTheDocument()
      expect(
        screen.getByTestId('delete-bucket-confirm-button')
      ).toBeInTheDocument()
    })
  })

  describe('CreditBucketEditor — 409 registration_pool_conflict', () => {
    // NOTE (deviation, loud): the item wording described the registration-pool
    // conflict as a destructive AlertDialog triggered by the Switch flip. The
    // ACTUAL implementation surfaces it as an INLINE destructive Alert
    // (testid `credit-bucket-editor-registration-conflict`) that appears AFTER
    // the form is submitted and the server returns 409
    // `registration_pool_conflict`. The Switch itself does not trigger a dialog.
    // We test the real user-observable behavior via MSW (no mocked mutation).

    it('surfaces the registration-pool conflict Alert when a submit returns 409 registration_pool_conflict', async () => {
      const user = userEvent.setup()
      // Spy on the real PUT URL so we also assert the request was issued
      // (proves the real mutation fired, not a mocked function).
      const putSpy = vi.fn()
      server.use(
        ...creditBucketsHandlers,
        http.put(
          'http://localhost:3000/api/realms/:realmId/billing/credit-buckets/:bucketId',
          ({ params, request }) => {
            putSpy({ params, url: request.url })
            return HttpResponse.json(
              { code: 'registration_pool_conflict' },
              { status: 409 }
            )
          }
        )
      )

      withQueryClient(
        <CreditBucketEditor
          realmId="r1"
          bucket={makeBucket({ id: 'b1', name: 'Promo Pool' })}
          formKey="b1"
          onSaved={vi.fn()}
        />
      )

      // Wait for the update form to reset with the bucket's name — the editor
      // hydrates the form via an effect on formKey, and submit is rejected by
      // zod until that lands.
      const nameInput = await screen.findByDisplayValue('Promo Pool')
      expect(nameInput).toBeInTheDocument()

      // Flip the registration-pool switch on (accessibility-first query).
      const regSwitch = screen.getByRole('switch', {
        name: /registration/i,
      })
      expect(regSwitch).not.toBeChecked()
      await user.click(regSwitch)
      expect(regSwitch).toBeChecked()

      // No conflict yet — it only surfaces after submit.
      expect(
        screen.queryByTestId('credit-bucket-editor-registration-conflict')
      ).not.toBeInTheDocument()

      // Submit and assert the destructive inline Alert appears.
      await user.click(screen.getByTestId('credit-bucket-editor-submit'))
      const conflictAlert = await screen.findByTestId(
        'credit-bucket-editor-registration-conflict'
      )
      expect(conflictAlert).toHaveAttribute('role', 'alert')
      // The copy instructs the admin to unset the OTHER bucket first
      // (design §4.2.2 — no silent override).
      expect(conflictAlert).toHaveTextContent(
        /another bucket already receives registration credits/i
      )

      // The real PUT went to the right bucket endpoint.
      expect(putSpy).toHaveBeenCalledTimes(1)
      expect(putSpy.mock.calls[0][0]).toMatchObject({
        params: { realmId: 'r1', bucketId: 'b1' },
      })
    })

    it('does not show the conflict Alert when the update succeeds', async () => {
      const user = userEvent.setup()
      server.use(
        ...creditBucketsHandlers,
        // Successful PUT: returns the updated bucket body.
        http.put(
          'http://localhost:3000/api/realms/:realmId/billing/credit-buckets/:bucketId',
          () => HttpResponse.json(makeBucket({ receivesRegistrationCredits: true }))
        )
      )

      withQueryClient(
        <CreditBucketEditor
          realmId="r1"
          bucket={makeBucket({ id: 'b1', name: 'Promo Pool' })}
          formKey="b1"
          onSaved={vi.fn()}
        />
      )

      await screen.findByDisplayValue('Promo Pool')
      await user.click(screen.getByRole('switch', { name: /registration/i }))
      await user.click(screen.getByTestId('credit-bucket-editor-submit'))

      // Allow any pending state to settle; the Alert must never appear.
      expect(
        screen.queryByTestId('credit-bucket-editor-registration-conflict')
      ).not.toBeInTheDocument()
    })
  })
})

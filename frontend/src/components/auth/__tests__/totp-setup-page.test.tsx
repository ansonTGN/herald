import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { TotpSetupPage } from '../totp-setup-page'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { EnableTotpResponse, VerifyTotpSetupResponse } from '@/lib/api-generated'

// Mock API functions
vi.mock('@/lib/api-generated', () => ({
  handleEnableTotp: vi.fn(),
  handleVerifyTotpSetup: vi.fn(),
}))

// Mock useFormMutation hook
vi.mock('@/hooks/use-form-mutation', () => ({
  useFormMutation: vi.fn(),
}))

// Mock QR code library
vi.mock('qrcode.react', () => ({
  QRCodeCanvas: ({ value, dataTestId }: { value: string; dataTestId?: string }) => (
    <div data-testid={dataTestId || 'totp-qr-code'} data-value={value}>
      QR Code
    </div>
  ),
}))

// Mock BackupCodesDisplay
vi.mock('@/components/profile/totp/backup-codes-display', () => ({
  BackupCodesDisplay: ({ backupCodes }: { backupCodes: string[] }) => (
    <div data-testid="backup-codes-display">
      {backupCodes.map((code, index) => (
        <code key={index} data-testid={`backup-code-${index}`}>
          {code}
        </code>
      ))}
    </div>
  ),
}))

// Mock router
const mockNavigate = vi.fn()
vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

// Mock auth store
vi.mock('@/stores/auth-store', () => ({
  useRealmId: () => 'test-realm',
}))

import { handleEnableTotp, handleVerifyTotpSetup } from '@/lib/api-generated'
import { useFormMutation } from '@/hooks/use-form-mutation'

describe('TotpSetupPage', () => {
  let mockQueryClient: QueryClient

  const mockEnableTotpResponse: EnableTotpResponse = {
    secret: 'JBSWY3DPEHPK3PXP',
    qrCodeUrl: 'otpauth://totp/test?secret=JBSWY3DPEHPK3PXP',
    backupCodes: ['code1', 'code2', 'code3'],
    tempToken: 'temp-token-123',
  }

  const mockVerifyTotpResponse: VerifyTotpSetupResponse = {
    success: true,
  }

  beforeEach(() => {
    vi.clearAllMocks()

    // Create QueryClient for each test
    mockQueryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    })

    // Mock useFormMutation implementation
    vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
      return {
        isSubmitting: false,
        mutate: async () => {
          if (onSuccess) {
            onSuccess(mockEnableTotpResponse as any)
          }
        },
      }
    })

    // Mock API handlers
    vi.mocked(handleEnableTotp).mockResolvedValue({
      data: mockEnableTotpResponse,
      error: undefined,
    })

    vi.mocked(handleVerifyTotpSetup).mockResolvedValue({
      data: mockVerifyTotpResponse,
      error: undefined,
    })
  })

  const renderWithQueryClient = (component: React.ReactElement) => {
    return render(<QueryClientProvider client={mockQueryClient}>{component}</QueryClientProvider>)
  }

  describe('rendering', () => {
    it('GIVEN page renders WHEN mounting THEN should display page title and step description', () => {
      renderWithQueryClient(<TotpSetupPage />)

      expect(screen.getByTestId('totp-setup-page')).toBeInTheDocument()
      expect(screen.getByTestId('totp-setup-page-title')).toHaveTextContent(
        'Set Up Two-Factor Authentication'
      )
      expect(screen.getByTestId('totp-setup-page-description')).toHaveTextContent('Step 1 of 3')
    })

    it('GIVEN page renders WHEN mounting THEN should start at password step', () => {
      renderWithQueryClient(<TotpSetupPage />)

      expect(screen.getByTestId('totp-setup-step-password')).toBeInTheDocument()
      expect(screen.getByTestId('totp-setup-password-input')).toBeInTheDocument()
    })

    it('GIVEN password step WHEN rendering THEN should display generate button', () => {
      renderWithQueryClient(<TotpSetupPage />)

      expect(screen.getByTestId('totp-setup-generate-button')).toHaveTextContent('Generate QR Code')
    })
  })

  describe('password confirmation step', () => {
    it('GIVEN user types password WHEN typing THEN should update input value', async () => {
      const user = userEvent.setup()
      renderWithQueryClient(<TotpSetupPage />)

      const input = screen.getByTestId('totp-setup-password-input')
      await user.type(input, 'password123')

      expect(input).toHaveValue('password123')
    })

    it('GIVEN user submits empty password WHEN submitting THEN should show validation error', async () => {
      const user = userEvent.setup()
      renderWithQueryClient(<TotpSetupPage />)

      const submitButton = screen.getByTestId('totp-setup-generate-button')
      await user.click(submitButton)

      await waitFor(() => {
        const errorElement = screen.queryByTestId('totp-password-error')
        expect(errorElement).toBeInTheDocument()
      })
    })

    it('GIVEN user submits valid password WHEN submitting THEN should call enableTotp API', async () => {
      const user = userEvent.setup()

      // Override mock to actually execute mutationFn so handleEnableTotp is called
      vi.mocked(useFormMutation).mockImplementation(({ mutationFn, onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async (variables: any) => {
            const result = await mutationFn(variables)
            if (onSuccess) {
              onSuccess(result as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(handleEnableTotp).toHaveBeenCalledWith({
          body: { password: 'password123' },
        })
      })
    })
  })

  describe('QR code display step', () => {
    it('GIVEN password confirmed WHEN advancing to QR step THEN should display QR code', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-qr-code')).toBeInTheDocument()
        expect(screen.getByTestId('totp-qr-code-container')).toBeInTheDocument()
        expect(screen.getByTestId('totp-qr-code')).toBeInTheDocument()
      })
    })

    it('GIVEN QR code step WHEN rendering THEN should display secret key with copy button', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-qr-code-container')).toBeInTheDocument()
        expect(screen.getByTestId('totp-qr-code-container')).toHaveAttribute(
          'data-secret',
          'JBSWY3DPEHPK3PXP'
        )
      })
    })

    it.todo('GIVEN qr code WHEN rendered THEN should display QR code with secret data attribute')

    it('GIVEN QR code step WHEN rendering THEN should display backup codes', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('backup-codes-display')).toBeInTheDocument()
        expect(screen.getByTestId('backup-code-0')).toHaveTextContent('code1')
        expect(screen.getByTestId('backup-code-1')).toHaveTextContent('code2')
        expect(screen.getByTestId('backup-code-2')).toHaveTextContent('code3')
      })
    })

    it('GIVEN QR code step WHEN rendering THEN should display backup codes confirmation checkbox', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-saved-backup-codes-checkbox')).toBeInTheDocument()
        expect(screen.getByTestId('totp-saved-backup-codes-label')).toHaveTextContent(
          'I have saved my backup codes in a secure location'
        )
      })
    })

    it('GIVEN backup codes not confirmed WHEN clicking Next THEN should disable Next button', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-next-button')).toBeDisabled()
      })
    })

    it('GIVEN backup codes confirmed WHEN clicking checkbox THEN should enable Next button', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-next-button')).toBeDisabled()
      })

      await user.click(screen.getByTestId('totp-saved-backup-codes-checkbox'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-next-button')).not.toBeDisabled()
      })
    })

    it('GIVEN backup codes confirmed WHEN clicking Next THEN should advance to verify step', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-qr-code')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('totp-saved-backup-codes-checkbox'))
      await user.click(screen.getByTestId('totp-setup-next-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-verify')).toBeInTheDocument()
      })
    })

    it('GIVEN QR code step WHEN clicking Back THEN should return to password step', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-qr-code')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('totp-setup-back-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-password')).toBeInTheDocument()
        expect(screen.queryByTestId('totp-setup-step-qr-code')).not.toBeInTheDocument()
      })
    })
  })

  describe('verification code step', () => {
    it('GIVEN verification step WHEN rendering THEN should display 6 OTP input fields', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-qr-code')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('totp-saved-backup-codes-checkbox'))
      await user.click(screen.getByTestId('totp-setup-next-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-verify')).toBeInTheDocument()
        expect(screen.getByTestId('totp-otp-input')).toBeInTheDocument()
        for (let i = 0; i < 6; i++) {
          expect(screen.getByTestId(`totp-otp-digit-${i}`)).toBeInTheDocument()
        }
      })
    })

    it('GIVEN OTP input WHEN typing digit THEN should auto-focus next input', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-qr-code')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('totp-saved-backup-codes-checkbox'))
      await user.click(screen.getByTestId('totp-setup-next-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-verify')).toBeInTheDocument()
      })

      const firstInput = screen.getByTestId('totp-otp-digit-0')
      await user.type(firstInput, '1')

      await waitFor(() => {
        const secondInput = screen.getByTestId('totp-otp-digit-1')
        expect(secondInput).toHaveFocus()
      })
    })

    it('GIVEN OTP input WHEN pressing backspace on empty field THEN should move to previous input', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-qr-code')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('totp-saved-backup-codes-checkbox'))
      await user.click(screen.getByTestId('totp-setup-next-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-verify')).toBeInTheDocument()
      })

      const firstInput = screen.getByTestId('totp-otp-digit-0')
      await user.type(firstInput, '1')

      const secondInput = screen.getByTestId('totp-otp-digit-1')
      await user.click(secondInput)

      await user.keyboard('{Backspace}')

      await waitFor(() => {
        expect(firstInput).toHaveFocus()
        expect(firstInput).toHaveValue('')
      })
    })

    it('GIVEN incomplete code WHEN submitting THEN should disable submit button', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-qr-code')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('totp-saved-backup-codes-checkbox'))
      await user.click(screen.getByTestId('totp-setup-next-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-verify')).toBeInTheDocument()
      })

      // Type only 3 digits
      await user.type(screen.getByTestId('totp-otp-digit-0'), '1')
      await user.type(screen.getByTestId('totp-otp-digit-1'), '2')
      await user.type(screen.getByTestId('totp-otp-digit-2'), '3')

      expect(screen.getByTestId('totp-verify-submit-button')).toBeDisabled()
    })

    it('GIVEN complete code WHEN submitting THEN should call verifyTotpSetup API', async () => {
      const user = userEvent.setup()
      let capturedVariables: any = null

      // Mock verify mutation
      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async (variables: any) => {
            capturedVariables = variables
            if (onSuccess) {
              onSuccess(mockVerifyTotpResponse as any)
            }
          },
        }
      })

      // Mock enable mutation (first call)
      vi.mocked(useFormMutation).mockImplementationOnce(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-qr-code')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('totp-saved-backup-codes-checkbox'))
      await user.click(screen.getByTestId('totp-setup-next-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-verify')).toBeInTheDocument()
      })

      // Type all 6 digits
      await user.type(screen.getByTestId('totp-otp-digit-0'), '1')
      await user.type(screen.getByTestId('totp-otp-digit-1'), '2')
      await user.type(screen.getByTestId('totp-otp-digit-2'), '3')
      await user.type(screen.getByTestId('totp-otp-digit-3'), '4')
      await user.type(screen.getByTestId('totp-otp-digit-4'), '5')
      await user.type(screen.getByTestId('totp-otp-digit-5'), '6')

      await user.click(screen.getByTestId('totp-verify-submit-button'))

      await waitFor(() => {
        expect(capturedVariables).toEqual({
          code: '123456',
          tempToken: 'temp-token-123',
        })
      })
    })

    it('GIVEN verify step WHEN clicking Back THEN should return to QR code step', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-qr-code')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('totp-saved-backup-codes-checkbox'))
      await user.click(screen.getByTestId('totp-setup-next-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-verify')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('totp-verify-back-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-qr-code')).toBeInTheDocument()
        expect(screen.queryByTestId('totp-setup-step-verify')).not.toBeInTheDocument()
      })
    })
  })

  describe('navigation', () => {
    it('GIVEN successful verification WHEN verify succeeds THEN should navigate to security page', async () => {
      const user = userEvent.setup()

      // Mock verify mutation
      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockVerifyTotpResponse as any)
            }
          },
        }
      })

      // Mock enable mutation (first call)
      vi.mocked(useFormMutation).mockImplementationOnce(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-qr-code')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('totp-saved-backup-codes-checkbox'))
      await user.click(screen.getByTestId('totp-setup-next-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-verify')).toBeInTheDocument()
      })

      await user.type(screen.getByTestId('totp-otp-digit-0'), '1')
      await user.type(screen.getByTestId('totp-otp-digit-1'), '2')
      await user.type(screen.getByTestId('totp-otp-digit-2'), '3')
      await user.type(screen.getByTestId('totp-otp-digit-3'), '4')
      await user.type(screen.getByTestId('totp-otp-digit-4'), '5')
      await user.type(screen.getByTestId('totp-otp-digit-5'), '6')

      await user.click(screen.getByTestId('totp-verify-submit-button'))

      await waitFor(() => {
        expect(mockNavigate).toHaveBeenCalledWith({
          to: '/$realmId/user/security',
          params: { realmId: 'test-realm' },
        })
      })
    })

    it('GIVEN back button WHEN clicking THEN should navigate to security page', async () => {
      const user = userEvent.setup()

      renderWithQueryClient(<TotpSetupPage />)

      await user.click(screen.getByTestId('totp-setup-back-to-security'))

      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/$realmId/user/security',
        params: { realmId: 'test-realm' },
      })
    })
  })

  describe('step indicator', () => {
    it('GIVEN page WHEN on password step THEN should show step 1 of 3', () => {
      renderWithQueryClient(<TotpSetupPage />)

      expect(screen.getByTestId('totp-setup-step-indicator')).toHaveAttribute('aria-valuenow', '1')
    })

    it('GIVEN page WHEN on QR code step THEN should show step 2 of 3', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-indicator')).toHaveAttribute(
          'aria-valuenow',
          '2'
        )
      })
    })

    it('GIVEN page WHEN on verify step THEN should show step 3 of 3', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-qr-code')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('totp-saved-backup-codes-checkbox'))
      await user.click(screen.getByTestId('totp-setup-next-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-indicator')).toHaveAttribute(
          'aria-valuenow',
          '3'
        )
      })
    })
  })

  describe('accessibility', () => {
    it('GIVEN password input WHEN rendering THEN should have proper ARIA attributes', () => {
      renderWithQueryClient(<TotpSetupPage />)

      const input = screen.getByTestId('totp-setup-password-input')
      expect(input).toHaveAttribute('aria-required', 'true')
      expect(input).toHaveAttribute('aria-describedby')
      expect(input).toHaveAttribute('aria-invalid', 'false')
    })

    it('GIVEN QR code container WHEN rendering THEN should have proper ARIA attributes', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        const container = screen.getByTestId('totp-qr-code-container')
        expect(container).toHaveAttribute('role', 'img')
        expect(container).toHaveAttribute('aria-label', 'QR code for TOTP setup')
      })
    })

    it('GIVEN OTP inputs WHEN rendering THEN should have proper ARIA labels', async () => {
      const user = userEvent.setup()

      vi.mocked(useFormMutation).mockImplementation(({ onSuccess }) => {
        return {
          isSubmitting: false,
          mutate: async () => {
            if (onSuccess) {
              onSuccess(mockEnableTotpResponse as any)
            }
          },
        }
      })

      renderWithQueryClient(<TotpSetupPage />)

      await user.type(screen.getByTestId('totp-setup-password-input'), 'password123')
      await user.click(screen.getByTestId('totp-setup-generate-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-setup-step-qr-code')).toBeInTheDocument()
      })

      await user.click(screen.getByTestId('totp-saved-backup-codes-checkbox'))
      await user.click(screen.getByTestId('totp-setup-next-button'))

      await waitFor(() => {
        expect(screen.getByTestId('totp-otp-digit-0')).toHaveAttribute('aria-label', 'Digit 1')
        expect(screen.getByTestId('totp-otp-digit-1')).toHaveAttribute('aria-label', 'Digit 2')
      })
    })
  })
})

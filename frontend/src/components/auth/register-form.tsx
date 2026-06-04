import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { z } from 'zod'
import { register } from '@/lib/api-generated'
import type { RegisterRequest } from '@/lib/api-generated'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { PasswordStrengthMeter } from './password-strength-meter'
import { TurnstileWidget } from './turnstile-widget'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { TextField } from '@/components/shared/form-fields/text-field'
import { useQuery } from '@tanstack/react-query'
import { queryKeys, turnstileStatusQueryOptions } from '@/data/query-options'
import { m } from '@/paraglide/messages'

const PASSWORD_MIN_LENGTH = 8
const NICKNAME_MAX_LENGTH = 50

// Local form data type that includes confirmPassword and nickname
type RegisterFormData = {
  email: string
  password: string
  confirmPassword: string
  nickname?: string
  turnstileToken?: string
}

const registerSchema = z
  .object({
    email: z.string().email(m['auth.email_invalid']()),
    password: z.string().min(PASSWORD_MIN_LENGTH, m['auth.password_min_length']()),
    confirmPassword: z.string().min(1, m['auth.confirm_password_required']()),
    nickname: z.string().max(NICKNAME_MAX_LENGTH, m['auth.nickname_max_length']()).optional(),
    turnstileToken: z.string().optional(),
  })
  .refine((data) => data.password === data.confirmPassword, {
    message: m['auth.passwords_dont_match'](),
    path: ['confirmPassword'],
  })

interface RegisterFormProps {
  realmId: string
  onSuccess: (verificationRequired: boolean) => void
}

const PASSWORD_CONFIG = {
  minLength: PASSWORD_MIN_LENGTH,
  requireUppercase: true,
  requireLowercase: true,
  requireNumber: true,
  requireSpecialChar: true,
} as const

export function RegisterForm({ realmId, onSuccess }: RegisterFormProps) {
  const { data: turnstileStatus, isLoading: loadingTurnstile } = useQuery(
    turnstileStatusQueryOptions(realmId)
  )

  const { isSubmitting, mutate } = useFormMutation({
    mutationFn: async (data: RegisterFormData) => {
      // Transform form data to API request format
      const apiData: RegisterRequest = {
        email: data.email,
        password: data.password,
        username: data.nickname || null,
        turnstileToken: data.turnstileToken || null,
      }
      const { data: result, error } = await register({
        path: { realmId },
        body: apiData,
        throwOnError: false,
      })
      if (error) {
        throw error
      }
      if (!result) {
        throw new Error('No data in response')
      }
      return result
    },
    getSuccessMessage: (data) => data.message || m['auth.register.registration_successful'](),
    invalidateQueries: [queryKeys.usersList(realmId)],
    onSuccess: (data) => onSuccess?.(data.verificationRequired),
  })

  const form = useAppForm({
    schema: registerSchema,
    defaultValues: {
      email: '',
      password: '',
      confirmPassword: '',
      nickname: undefined,
      turnstileToken: undefined,
    },
    onSubmit: async ({ value }) => {
      await mutate(value)
    },
  })

  return (
    <AppForm>
      <form
        onSubmit={(e) => {
          e.preventDefault()
          e.stopPropagation()
          form.handleSubmit()
        }}
        className="space-y-4"
      >
        <TextField
          form={form}
          name="email"
          label={m['auth.verify_email.email_label']()}
          type="email"
          dataTestId="register-email-input"
          disabled={isSubmitting}
        />
        <TextField
          form={form}
          name="nickname"
          label={`${m['auth.register.title']()} (${m['common.optional']()})`}
          dataTestId="register-nickname-input"
          disabled={isSubmitting}
        />

        <form.Field name="password">
          {(field) => (
            <div className="space-y-2">
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                type="password"
                value={field.state.value ?? ''}
                onChange={(e) => field.handleChange(e.target.value)}
                disabled={isSubmitting}
                data-testid="register-password-input"
              />
              {(field.state.meta.isTouched || form.state.isSubmitted) &&
                field.state.meta.errors.length > 0 && (
                  <p className="text-sm text-red-500">{getFieldErrorMessage(field.state.meta)}</p>
                )}
              <PasswordStrengthMeter password={field.state.value ?? ''} config={PASSWORD_CONFIG} />
            </div>
          )}
        </form.Field>

        <TextField
          form={form}
          name="confirmPassword"
          label="Confirm Password"
          type="password"
          dataTestId="register-confirm-password-input"
          disabled={isSubmitting}
        />

        {!loadingTurnstile && turnstileStatus?.enabled && (
          <form.Field name="turnstileToken">
            {(field) => (
              <div className="space-y-2">
                <Label>Complete security verification</Label>
                <TurnstileWidget
                  siteKey={turnstileStatus.site_key || ''}
                  onTokenChange={(token) => field.handleChange(token || '')}
                  onError={(error) => {
                    // TanStack Form FieldApi doesn't have setError, so we don't set error
                    // The field will remain empty, causing validation to fail
                    console.error('Turnstile error:', error)
                  }}
                />
                {(field.state.meta.isTouched || form.state.isSubmitted) &&
                  field.state.meta.errors.length > 0 && (
                    <p className="text-sm text-red-500">{getFieldErrorMessage(field.state.meta)}</p>
                  )}
              </div>
            )}
          </form.Field>
        )}

        <Button
          type="submit"
          data-testid="register-submit-button"
          disabled={isSubmitting}
          className="w-full"
        >
          {isSubmitting ? 'Registering...' : 'Register'}
        </Button>
      </form>
    </AppForm>
  )
}

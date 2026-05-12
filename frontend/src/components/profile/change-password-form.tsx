import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { changePasswordSchema, type ChangePasswordFormData } from '@/lib/schemas/common'
import { changeUserPassword } from '@/lib/api-generated'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { queryKeys } from '@/data/query-options'

export function ChangePasswordForm() {
  const form = useAppForm({
    schema: changePasswordSchema,
    defaultValues: {
      oldPass: '',
      newPass: '',
      confirmPass: '',
    },
    onSubmit: async ({ value }) => {
      await mutate(value)
    },
  })

  const { isSubmitting, mutate } = useFormMutation({
    mutationFn: (data: ChangePasswordFormData) =>
      changeUserPassword({
        body: data, // Direct pass - type-safe
      }),
    getSuccessMessage: () => 'Password changed successfully',
    invalidateQueries: [queryKeys.profile()],
    onSuccess: () => {
      form.reset()
    },
  })

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold">Change Password</h2>

      <AppForm>
        <form
          onSubmit={(e) => {
            e.preventDefault()
            e.stopPropagation()
            form.handleSubmit()
          }}
          className="space-y-4"
        >
          <form.Field
            name="oldPass"
            children={(field) => (
              <div className="space-y-2">
                <Label htmlFor="oldPass">Current Password</Label>
                <Input
                  id="oldPass"
                  type="password"
                  value={field.state.value ?? ''}
                  onChange={(e) => field.handleChange(e.target.value)}
                  data-testid="change-password-old-input"
                />
                {(field.state.meta.isTouched || form.state.isSubmitted) &&
                  field.state.meta.errors.length > 0 && (
                    <p className="text-sm text-red-500">{getFieldErrorMessage(field.state.meta)}</p>
                  )}
              </div>
            )}
          />

          <form.Field
            name="newPass"
            children={(field) => (
              <div className="space-y-2">
                <Label htmlFor="newPass">New Password</Label>
                <Input
                  id="newPass"
                  type="password"
                  value={field.state.value ?? ''}
                  onChange={(e) => field.handleChange(e.target.value)}
                  data-testid="change-password-new-input"
                />
                {(field.state.meta.isTouched || form.state.isSubmitted) &&
                  field.state.meta.errors.length > 0 && (
                    <p className="text-sm text-red-500">{getFieldErrorMessage(field.state.meta)}</p>
                  )}
              </div>
            )}
          />

          <form.Field
            name="confirmPass"
            children={(field) => (
              <div className="space-y-2">
                <Label htmlFor="confirmPass">Confirm New Password</Label>
                <Input
                  id="confirmPass"
                  type="password"
                  value={field.state.value ?? ''}
                  onChange={(e) => field.handleChange(e.target.value)}
                  data-testid="change-password-confirm-input"
                />
                {(field.state.meta.isTouched || form.state.isSubmitted) &&
                  field.state.meta.errors.length > 0 && (
                    <p className="text-sm text-red-500">{getFieldErrorMessage(field.state.meta)}</p>
                  )}
              </div>
            )}
          />

          <Button type="submit" disabled={isSubmitting} data-testid="change-password-submit-button">
            {isSubmitting ? 'Changing...' : 'Change Password'}
          </Button>
        </form>
      </AppForm>
    </div>
  )
}

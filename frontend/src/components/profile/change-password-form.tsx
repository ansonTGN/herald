import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { changePasswordSchema, type ChangePasswordFormData } from '@/lib/schemas/common'
import { changeUserPassword } from '@/lib/api-generated'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { TextField } from '@/components/shared/form-fields/text-field'
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
        body: data,
      }),
    getSuccessMessage: () => 'Password changed successfully',
    invalidateQueries: [queryKeys.profile()],
    onSuccess: () => {
      form.reset()
    },
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>Change Password</CardTitle>
      </CardHeader>
      <CardContent>
        <AppForm>
          <form
            onSubmit={(e) => {
              e.preventDefault()
              e.stopPropagation()
              form.handleSubmit()
            }}
            className="max-w-sm space-y-4"
          >
            <TextField
              form={form}
              name="oldPass"
              label="Current Password"
              type="password"
              dataTestId="change-password-old-input"
            />
            <TextField
              form={form}
              name="newPass"
              label="New Password"
              type="password"
              dataTestId="change-password-new-input"
            />
            <TextField
              form={form}
              name="confirmPass"
              label="Confirm New Password"
              type="password"
              dataTestId="change-password-confirm-input"
            />

            <Button
              type="submit"
              disabled={isSubmitting}
              data-testid="change-password-submit-button"
            >
              {isSubmitting ? 'Changing...' : 'Change Password'}
            </Button>
          </form>
        </AppForm>
      </CardContent>
    </Card>
  )
}

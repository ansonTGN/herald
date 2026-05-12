import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { createRealmSchema, type CreateRealmFormData } from '@/lib/schemas/realm'
import { createRealm } from '@/lib/api-generated'
import { useFormMutation } from '@/hooks/use-form-mutation'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { queryKeys } from '@/data/query-options'

interface CreateRealmDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function CreateRealmDialog({ open, onOpenChange }: CreateRealmDialogProps) {
  const { isSubmitting, mutate } = useFormMutation({
    mutationFn: (data: CreateRealmFormData) =>
      createRealm({
        body: data,
      }),
    getSuccessMessage: (response) => {
      const name = response.data?.name ?? 'Realm'
      return `Realm "${name}" created successfully`
    },
    invalidateQueries: [queryKeys.realmsList()],
    onSuccess: () => {
      onOpenChange(false)
    },
  })

  const form = useAppForm({
    schema: createRealmSchema,
    defaultValues: {
      name: '',
      adminUser: {
        email: '',
        password: '',
      },
    },
    onSubmit: async ({ value }) => {
      await mutate(value)
    },
  })

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]" data-testid="dialog">
        <DialogHeader>
          <DialogTitle data-testid="dialog-title">Create New Realm</DialogTitle>
          <DialogDescription>
            Fill in the realm information below. An admin user will be created automatically.
          </DialogDescription>
        </DialogHeader>

        <AppForm>
          <form
            className="space-y-4"
            onSubmit={(e) => {
              e.preventDefault()
              e.stopPropagation()
              form.handleSubmit()
            }}
          >
            {/* Realm ID */}
            <form.Field
              name="id"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor="realm-id">Realm ID</Label>
                  <Input
                    id="realm-id"
                    value={field.state.value ?? ''}
                    onChange={(e) => field.handleChange(e.target.value)}
                    data-testid="realm-create-id-input"
                  />
                  {(field.state.meta.isTouched || form.state.isSubmitted) &&
                    field.state.meta.errors.length > 0 && (
                      <p className="text-sm text-destructive">
                        {getFieldErrorMessage(field.state.meta)}
                      </p>
                    )}
                </div>
              )}
            />

            {/* Realm Name */}
            <form.Field
              name="name"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor="realm-name">Realm Name</Label>
                  <Input
                    id="realm-name"
                    value={field.state.value ?? ''}
                    onChange={(e) => field.handleChange(e.target.value)}
                    data-testid="realm-create-name-input"
                  />
                  {(field.state.meta.isTouched || form.state.isSubmitted) &&
                    field.state.meta.errors.length > 0 && (
                      <p className="text-sm text-destructive">
                        {getFieldErrorMessage(field.state.meta)}
                      </p>
                    )}
                </div>
              )}
            />

            {/* Admin Email */}
            <form.Field
              name="adminUser.email"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor="admin-email">Admin Email</Label>
                  <Input
                    id="admin-email"
                    type="email"
                    value={field.state.value ?? ''}
                    onChange={(e) => field.handleChange(e.target.value)}
                    data-testid="realm-create-admin-email-input"
                  />
                  {(field.state.meta.isTouched || form.state.isSubmitted) &&
                    field.state.meta.errors.length > 0 && (
                      <p className="text-sm text-destructive">
                        {getFieldErrorMessage(field.state.meta)}
                      </p>
                    )}
                </div>
              )}
            />

            {/* Admin Password */}
            <form.Field
              name="adminUser.password"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor="admin-password">Admin Password</Label>
                  <Input
                    id="admin-password"
                    type="password"
                    value={field.state.value ?? ''}
                    onChange={(e) => field.handleChange(e.target.value)}
                    data-testid="realm-create-admin-password-input"
                  />
                  {(field.state.meta.isTouched || form.state.isSubmitted) &&
                    field.state.meta.errors.length > 0 && (
                      <p className="text-sm text-destructive">
                        {getFieldErrorMessage(field.state.meta)}
                      </p>
                    )}
                </div>
              )}
            />

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
                data-testid="dialog-cancel-button"
              >
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting} data-testid="dialog-submit-button">
                {isSubmitting ? 'Creating...' : 'Create Realm'}
              </Button>
            </DialogFooter>
          </form>
        </AppForm>
      </DialogContent>
    </Dialog>
  )
}

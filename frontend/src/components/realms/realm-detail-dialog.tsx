import { useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { updateRealmSchema, type UpdateRealmFormData } from '@/lib/schemas/realm'
import { updateRealm } from '@/lib/api-generated'
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
import { Textarea } from '@/components/ui/textarea'
import { Label } from '@/components/ui/label'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { queryKeys, realmQueryOptions } from '@/data/query-options'
import { m } from '@/paraglide/messages'

interface RealmDetailDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  realmId: string | null
}

export function RealmDetailDialog({ open, onOpenChange, realmId }: RealmDetailDialogProps) {
  const { data: realm, isLoading } = useQuery({
    ...realmQueryOptions(realmId ?? ''),
    enabled: open && !!realmId,
  })

  const { isSubmitting, mutate } = useFormMutation({
    mutationFn: (data: UpdateRealmFormData) => {
      if (!realmId) throw new Error('Realm ID is required')
      return updateRealm({
        path: { realmId },
        body: data,
      })
    },
    getSuccessMessage: () => m['realms.realm_updated_success'](),
    invalidateQueries: [queryKeys.realmsList(), queryKeys.realm(realmId)],
    onSuccess: () => {
      onOpenChange(false)
    },
  })

  const form = useAppForm({
    schema: updateRealmSchema,
    defaultValues: {
      name: realm?.name ?? '',
      description: realm?.description ?? '',
    },
    onSubmit: async ({ value }) => {
      await mutate(value)
    },
  })

  useEffect(() => {
    if (realm?.name) {
      form.setFieldValue('name', realm.name)
    }
    if (realm) {
      form.setFieldValue('description', realm.description ?? '')
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [realm?.name, realm?.description, form])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]" data-testid="dialog">
        <DialogHeader>
          <DialogTitle data-testid="dialog-title">{m['realms.edit_title']()}</DialogTitle>
          <DialogDescription>{m['realms.edit_description']()}</DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="py-4" data-testid="realm-detail-loading">
            {m['common.loading']()}
          </div>
        ) : realm ? (
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>{m['realms.realm_id_label']()}</Label>
              <Input value={realm.id} disabled data-testid="realm-detail-id" />
            </div>

            <div className="space-y-2">
              <Label>{m['realms.created_at_label']()}</Label>
              <Input
                value={new Date(realm.createdAt).toLocaleString()}
                disabled
                data-testid="realm-detail-created-at"
              />
            </div>

            <div className="space-y-2">
              <Label>{m['realms.updated_at_label']()}</Label>
              <Input
                value={new Date(realm.updatedAt).toLocaleString()}
                disabled
                data-testid="realm-detail-updated-at"
              />
            </div>

            <AppForm>
              <form
                id="realm-edit-form"
                onSubmit={async (e) => {
                  e.preventDefault()
                  await form.handleSubmit()
                }}
              >
                <form.Field
                  name="name"
                  children={(field) => (
                    <div className="space-y-2">
                      <Label htmlFor="realm-name">{m['realms.realm_name_label']()}</Label>
                      <Input
                        id="realm-name"
                        value={field.state.value ?? ''}
                        onChange={(e) => field.handleChange(e.target.value)}
                        data-testid="realm-detail-name-input"
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
                <form.Field
                  name="description"
                  children={(field) => (
                    <div className="space-y-2 mt-4">
                      <Label htmlFor="realm-description">{m['realms.description_label']()}</Label>
                      <Textarea
                        id="realm-description"
                        value={field.state.value ?? ''}
                        onChange={(e) => field.handleChange(e.target.value)}
                        data-testid="realm-detail-description-input"
                        rows={3}
                      />
                    </div>
                  )}
                />
              </form>
            </AppForm>

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => {
                  form.reset()
                  onOpenChange(false)
                }}
                data-testid="dialog-cancel-button"
              >
                {m['realms.cancel']()}
              </Button>
              <Button
                type="submit"
                form="realm-edit-form"
                disabled={isSubmitting}
                data-testid="dialog-submit-button"
              >
                {isSubmitting ? m['realms.saving']() : m['realms.save']()}
              </Button>
            </DialogFooter>
          </div>
        ) : null}
      </DialogContent>
    </Dialog>
  )
}

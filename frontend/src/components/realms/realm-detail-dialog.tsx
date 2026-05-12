import { useState, useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { updateRealmSchema, type UpdateRealmFormData } from '@/lib/schemas/realm'
import { getRealm, updateRealm } from '@/lib/api-generated'
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

interface RealmDetailDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  realmId: string | null
}

export function RealmDetailDialog({ open, onOpenChange, realmId }: RealmDetailDialogProps) {
  const [isEditing, setIsEditing] = useState(false)

  // 获取 Realm 详情
  const { data: realm, isLoading } = useQuery({
    queryKey: queryKeys.realm(realmId),
    queryFn: () => {
      if (!realmId) throw new Error('Realm ID is required')
      return getRealm({ path: { realmId } })
    },
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
    getSuccessMessage: () => 'Realm updated successfully',
    invalidateQueries: [queryKeys.realmsList(), queryKeys.realm(realmId)],
    onSuccess: () => {
      setIsEditing(false)
      onOpenChange(false)
    },
  })

  const form = useAppForm({
    schema: updateRealmSchema,
    defaultValues: {
      name: realm?.data?.name ?? '',
    },
    onSubmit: async ({ value }) => {
      await mutate(value)
    },
  })

  // 当 realm 数据加载完成后更新表单
  useEffect(() => {
    if (realm?.data?.name) {
      form.setFieldValue('name', realm.data.name)
    }
  }, [realm?.data, form])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]" data-testid="dialog">
        <DialogHeader>
          <DialogTitle data-testid="dialog-title">Realm Details</DialogTitle>
          <DialogDescription>View and edit realm information</DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="py-4" data-testid="realm-detail-loading">
            Loading...
          </div>
        ) : realm?.data ? (
          <div className="space-y-4">
            {/* Realm ID (Read-only) */}
            <div className="space-y-2">
              <Label>Realm ID</Label>
              <Input value={realm.data.id} disabled data-testid="realm-detail-id" />
            </div>

            {/* Created At (Read-only) */}
            <div className="space-y-2">
              <Label>Created At</Label>
              <Input
                value={new Date(realm.data.createdAt).toLocaleString()}
                disabled
                data-testid="realm-detail-created-at"
              />
            </div>

            {/* Updated At (Read-only) */}
            <div className="space-y-2">
              <Label>Updated At</Label>
              <Input
                value={new Date(realm.data.updatedAt).toLocaleString()}
                disabled
                data-testid="realm-detail-updated-at"
              />
            </div>

            {/* Realm Name (Editable) */}
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
                      <Label htmlFor="realm-name">Realm Name</Label>
                      <Input
                        id="realm-name"
                        value={field.state.value ?? ''}
                        onChange={(e) => field.handleChange(e.target.value)}
                        disabled={!isEditing}
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
              </form>
            </AppForm>

            <DialogFooter>
              {isEditing ? (
                <>
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => {
                      setIsEditing(false)
                      form.reset()
                    }}
                    data-testid="dialog-cancel-button"
                  >
                    Cancel
                  </Button>
                  <Button
                    type="submit"
                    form="realm-edit-form"
                    disabled={isSubmitting}
                    data-testid="dialog-submit-button"
                  >
                    {isSubmitting ? 'Saving...' : 'Save'}
                  </Button>
                </>
              ) : (
                <Button
                  type="button"
                  onClick={() => setIsEditing(true)}
                  data-testid="realm-detail-edit-button"
                >
                  Edit
                </Button>
              )}
            </DialogFooter>
          </div>
        ) : null}
      </DialogContent>
    </Dialog>
  )
}

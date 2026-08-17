import { useEffect, useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Plus } from 'lucide-react'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { NumberField, TextField } from '@/components/shared'
import { getFieldErrorMessage } from '@/lib/form-utils'
import {
  createCreditBucketSchema,
  updateCreditBucketSchema,
} from '@/lib/schemas/credit-bucket-forms'
import type {
  CreateCreditBucketFormData,
  UpdateCreditBucketFormData,
} from '@/lib/schemas/credit-bucket-forms'
import { clientAppsQueryOptions } from '@/data/query-options'
import { useCreateCreditBucket, useUpdateCreditBucket } from '@/data/credit-bucket-mutations'
import type { BucketDetailResponse } from '@/lib/api-generated'
import { usePermission } from '@/hooks/use-permission'
import { PERMISSION } from '@/lib/constants/auth-constants'
import {
  ADMIN_WEB_CONSOLE_CLIENT_ID,
  USER_ACCOUNT_CENTER_CLIENT_ID,
  ADMIN_API_CLIENT_ID,
} from '@/lib/constants/auth-constants'
import { m } from '@/paraglide/messages'
import { CreditBucketCoverageMultiselect } from './credit-bucket-coverage-multiselect'
import { CreateClientAppDialog } from './create-client-app-dialog'

/**
 * Realm-infrastructure clients seeded at realm creation. Coverage must only
 * reference self-created apps, so these are excluded from the selectable
 * options (apps already covering a bucket stay listed so existing selections
 * remain visible and removable).
 */
const BUILT_IN_CLIENT_IDS = new Set([
  ADMIN_WEB_CONSOLE_CLIENT_ID,
  USER_ACCOUNT_CENTER_CLIENT_ID,
  ADMIN_API_CLIENT_ID,
])

interface CreditBucketEditorProps {
  realmId: string
  /** Detail payload when editing; null when creating. */
  bucket: BucketDetailResponse | null
  /** Keyed on the bucket id (or 'new'); change to reset the form. */
  formKey: string
  onSaved: () => void
}

/**
 * Right-column editor of the Bucket directory Master-Detail.
 *
 * Two modes share this component:
 * - create (`bucket === null`) → `useCreateCreditBucket`, bucketKey editable
 * - update (`bucket !== null`) → `useUpdateCreditBucket(realmId, bucket.id)`,
 *   bucketKey read-only (immutable identity)
 *
 * Registration and subscription grants are configured by distribution rules,
 * so this editor only owns bucket identity, display, and client-app coverage.
 */
export function CreditBucketEditor({ realmId, bucket, formKey, onSaved }: CreditBucketEditorProps) {
  const isCreate = bucket === null
  const bucketId = bucket?.id ?? ''

  const createMutation = useCreateCreditBucket(realmId)
  const updateMutation = useUpdateCreditBucket(realmId, bucketId)

  // Coverage-set option source. Built-in clients are filtered out; ids the
  // bucket already covers stay selectable so legacy selections (made before
  // the filter existed) can still be seen and removed.
  const { data: clientAppsData, isLoading: clientAppsLoading } = useQuery({
    ...clientAppsQueryOptions(realmId, { pageSize: 100 }),
  })

  const clientAppOptions = useMemo(() => {
    const coveredIds = new Set((bucket?.clientApps ?? []).map((c) => c.id))
    return (clientAppsData?.items ?? [])
      .filter((app) => coveredIds.has(app.id) || !BUILT_IN_CLIENT_IDS.has(app.clientId))
      .map((app) => ({ id: app.id, label: app.name, hint: app.clientId }))
  }, [clientAppsData, bucket])
  const createDefaults: CreateCreditBucketFormData = {
    bucketKey: '',
    name: '',
    description: null,
    displayOrder: 0,
    enabled: true,
    clientAppIds: [],
  }
  const updateDefaults: UpdateCreditBucketFormData = {
    name: '',
    description: null,
    displayOrder: 0,
    enabled: true,
    clientAppIds: [],
  }

  // Use a distinct form instance per mode to avoid schema/value shape drift.
  const createForm = useAppForm({
    schema: createCreditBucketSchema,
    defaultValues: createDefaults,
    onSubmit: async ({ value }) => {
      try {
        await createMutation.mutateAsync(value)
        toast.success(m['credit_buckets.create_success']())
        onSaved()
      } catch (error) {
        handleSubmissionError(error)
      }
    },
  })
  const updateForm = useAppForm({
    schema: updateCreditBucketSchema,
    defaultValues: updateDefaults,
    onSubmit: async ({ value }) => {
      try {
        await updateMutation.mutateAsync(value)
        toast.success(m['credit_buckets.update_success']())
        onSaved()
      } catch (error) {
        handleSubmissionError(error)
      }
    },
  })

  // Reset update form whenever the selected bucket changes.
  // `keepDefaultValues: true` is load-bearing: without it, `reset` overwrites
  // `form.options.defaultValues` with the bucket values, and because
  // `updateDefaults` below is a fresh object each render, the next render's
  // `formApi.update(opts)` deep-compares the (empty) incoming `defaultValues`
  // against the (populated) stored one, sees a mismatch, and — while the form
  // is still untouched — wipes `state.values` back to the empty defaults,
  // discarding the bucket data the moment any interaction triggers a re-render.
  // Keeping the defaultValues slot stable lets the effect be the sole owner of
  // `state.values`.
  useEffect(() => {
    if (!bucket) return
    updateForm.reset(
      {
        name: bucket.name,
        description: bucket.description ?? null,
        displayOrder: bucket.displayOrder,
        enabled: bucket.enabled,
        clientAppIds: bucket.clientApps.map((c) => c.id),
      },
      { keepDefaultValues: true }
    )
    // eslint-disable-next-line react-hooks/exhaustive-deps -- reset only on bucket/form identity change
  }, [formKey])

  function handleSubmissionError(error: unknown) {
    toast.error(readErrorMessage(error))
  }

  if (isCreate) {
    return (
      <EditorShell
        title={m['credit_buckets.editor_new_title']()}
        data-testid="credit-bucket-editor"
      >
        <AppForm>
          <form
            onSubmit={(e) => {
              e.preventDefault()
              createForm.handleSubmit()
            }}
            className="space-y-4"
          >
            <TextField
              form={createForm}
              name="bucketKey"
              label={m['credit_buckets.field_bucket_key']()}
              inputId="bucket-key"
              dataTestId="credit-bucket-editor-bucket-key"
              placeholder="e.g. promo-pool"
              required
              helpText={m['credit_buckets.field_bucket_key_help']()}
            />
            <BucketFieldsBody
              form={createForm}
              realmId={realmId}
              clientAppOptions={clientAppOptions}
              multiselectsLoading={clientAppsLoading}
            />
            <SubmitButton
              isPending={createMutation.isPending}
              label={m['credit_buckets.create_button']()}
            />
          </form>
        </AppForm>
      </EditorShell>
    )
  }

  return (
    <EditorShell title={m['credit_buckets.editor_edit_title']()} data-testid="credit-bucket-editor">
      <AppForm>
        <form
          onSubmit={(e) => {
            e.preventDefault()
            updateForm.handleSubmit()
          }}
          className="space-y-4"
        >
          {/* bucketKey is immutable on update — read-only display */}
          <div className="space-y-2">
            <Label htmlFor="bucket-key-readonly">{m['credit_buckets.field_bucket_key']()}</Label>
            <Input
              id="bucket-key-readonly"
              value={bucket?.bucketKey ?? ''}
              readOnly
              disabled
              data-testid="credit-bucket-editor-bucket-key"
            />
          </div>
          <BucketFieldsBody
            form={updateForm}
            realmId={realmId}
            clientAppOptions={clientAppOptions}
            multiselectsLoading={clientAppsLoading}
          />
          <div className="space-y-2" data-testid="credit-bucket-rule-references">
            <Label>Distribution rule references</Label>
            {bucket.ruleReferences.length === 0 ? (
              <p className="text-sm text-muted-foreground">No rules reference this account.</p>
            ) : (
              <ul className="space-y-1 text-sm">
                {bucket.ruleReferences.map((reference) => (
                  <li key={reference.ruleId} className="flex flex-wrap items-center gap-2">
                    <span className="font-mono text-xs">{reference.ruleId}</span>
                    <span>{reference.ownerType.replaceAll('_', ' ')}</span>
                    <span>{reference.triggerSources.join(', ')}</span>
                    {!reference.enabled && <span className="text-muted-foreground">disabled</span>}
                  </li>
                ))}
              </ul>
            )}
          </div>
          <SubmitButton
            isPending={updateMutation.isPending}
            label={m['credit_buckets.update_button']()}
          />
        </form>
      </AppForm>
    </EditorShell>
  )
}

/** Shared body (everything below the bucketKey/name identity row). */
function BucketFieldsBody({
  form,
  realmId,
  clientAppOptions,
  multiselectsLoading,
}: {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  form: any
  realmId: string
  clientAppOptions: { id: string; label: string; hint?: string }[]
  multiselectsLoading: boolean
}) {
  const { hasPermission } = usePermission()
  const canCreateClientApp = hasPermission(PERMISSION.CLIENTS_MANAGE)
  const [createDialogOpen, setCreateDialogOpen] = useState(false)

  return (
    <>
      <TextField
        form={form}
        name="name"
        label={m['credit_buckets.field_name']()}
        inputId="bucket-name"
        dataTestId="credit-bucket-editor-name"
        required
      />

      <form.Field name="description">
        {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
        {(field: any) => (
          <div className="space-y-2">
            <Label htmlFor="bucket-description">{m['credit_buckets.field_description']()}</Label>
            <Textarea
              id="bucket-description"
              data-testid="credit-bucket-editor-description"
              placeholder={m['credit_buckets.field_description_placeholder']()}
              rows={2}
              value={field.state.value ?? ''}
              onChange={(e) => field.handleChange(e.target.value === '' ? null : e.target.value)}
            />
            {(field.state.meta.isTouched || form.state.isSubmitted) &&
              field.state.meta.errors.length > 0 && (
                <p className="text-sm text-destructive" role="alert">
                  {getFieldErrorMessage(field.state.meta)}
                </p>
              )}
          </div>
        )}
      </form.Field>

      <NumberField
        form={form}
        name="displayOrder"
        label={m['credit_buckets.field_display_order']()}
        inputId="bucket-display-order"
        dataTestId="credit-bucket-editor-display-order"
      />

      <form.Field name="enabled">
        {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
        {(field: any) => (
          <div className="flex items-center gap-2">
            <Switch
              id="bucket-enabled"
              checked={field.state.value}
              onCheckedChange={field.handleChange}
              data-testid="credit-bucket-editor-enabled"
            />
            <Label htmlFor="bucket-enabled">{m['credit_buckets.field_enabled']()}</Label>
          </div>
        )}
      </form.Field>

      {multiselectsLoading ? (
        <Skeleton className="h-10 w-full" />
      ) : (
        <form.Field name="clientAppIds">
          {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
          {(field: any) => (
            <div className="space-y-1.5">
              <div className="flex items-center justify-between">
                <Label>{m['credit_buckets.field_coverage']()}</Label>
                {canCreateClientApp && (
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="h-7 gap-1 px-2 text-xs"
                    onClick={() => setCreateDialogOpen(true)}
                    data-testid="bucket-coverage-create-client-app"
                  >
                    <Plus className="h-3.5 w-3.5" />
                    {m['credit_buckets.coverage_create_button']()}
                  </Button>
                )}
              </div>
              <CreditBucketCoverageMultiselect
                options={clientAppOptions}
                value={field.state.value}
                onChange={field.handleChange}
                error={
                  (field.state.meta.isTouched || form.state.isSubmitted) &&
                  field.state.meta.errors.length > 0
                    ? getFieldErrorMessage(field.state.meta)
                    : undefined
                }
              />
              <CreateClientAppDialog
                realmId={realmId}
                open={createDialogOpen}
                onOpenChange={setCreateDialogOpen}
                onCreated={(clientAppId) => {
                  if (clientAppId && !field.state.value.includes(clientAppId)) {
                    field.handleChange([...field.state.value, clientAppId])
                  }
                }}
              />
            </div>
          )}
        </form.Field>
      )}
    </>
  )
}

function EditorShell({
  title,
  children,
  ...rest
}: {
  title: string
  children: React.ReactNode
} & React.HTMLAttributes<HTMLDivElement>) {
  return (
    <Card {...rest}>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent>{children}</CardContent>
    </Card>
  )
}

function SubmitButton({ isPending, label }: { isPending: boolean; label: string }) {
  return (
    <Button type="submit" disabled={isPending} data-testid="credit-bucket-editor-submit">
      {isPending ? m['credit_buckets.saving']() : label}
    </Button>
  )
}

function readErrorMessage(error: unknown): string {
  if (typeof error === 'string') return error
  if (error && typeof error === 'object') {
    if ('message' in error && typeof (error as { message: unknown }).message === 'string') {
      return (error as { message: string }).message
    }
  }
  return m['error.generic']()
}

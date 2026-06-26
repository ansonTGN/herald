import { useEffect, useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Alert, AlertDescription } from '@/components/ui/alert'
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
import { clientAppsQueryOptions, entitlementMappingsQueryOptions } from '@/data/query-options'
import { useCreateCreditBucket, useUpdateCreditBucket } from '@/data/credit-bucket-mutations'
import type { BucketDetailResponse, EntitlementMappingResponse } from '@/lib/api-generated'
import { m } from '@/paraglide/messages'
import { CreditBucketCoverageMultiselect } from './credit-bucket-coverage-multiselect'
import { CreditBucketMappingsMultiselect } from './credit-bucket-mappings-multiselect'

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
 * `receivesRegistrationCredits` Switch: when the server returns 409
 * `registration_pool_conflict` (another bucket already holds the registration
 * pool), a destructive Alert surfaces the conflict and instructs the admin to
 * unset it on the other bucket first — no silent override. NO isDefault
 * control anywhere.
 */
export function CreditBucketEditor({ realmId, bucket, formKey, onSaved }: CreditBucketEditorProps) {
  const isCreate = bucket === null
  const bucketId = bucket?.id ?? ''

  const createMutation = useCreateCreditBucket(realmId)
  const updateMutation = useUpdateCreditBucket(realmId, bucketId)

  // Coverage-set option source + mappings option source (query options).
  const { data: clientAppsData, isLoading: clientAppsLoading } = useQuery({
    ...clientAppsQueryOptions(realmId, { pageSize: 100 }),
  })
  const { data: mappingsData, isLoading: mappingsLoading } = useQuery({
    ...entitlementMappingsQueryOptions(realmId, { pageSize: 100 }),
  })

  const clientAppOptions = useMemo(
    () =>
      (clientAppsData?.items ?? []).map((app) => ({
        id: app.id,
        label: app.name,
        hint: app.clientId,
      })),
    [clientAppsData]
  )
  const mappingOptions = useMemo(
    () =>
      ((mappingsData as { items?: EntitlementMappingResponse[] } | undefined)?.items ?? []).map(
        (mp) => ({
          id: mp.id,
          label: mp.entitlementKey,
          hint: mp.externalProductId,
        })
      ),
    [mappingsData]
  )

  const createDefaults: CreateCreditBucketFormData = {
    bucketKey: '',
    name: '',
    description: null,
    displayOrder: 0,
    enabled: true,
    receivesRegistrationCredits: false,
    clientAppIds: [],
    entitlementMappingIds: [],
  }
  const updateDefaults: UpdateCreditBucketFormData = {
    name: '',
    description: null,
    displayOrder: 0,
    enabled: true,
    receivesRegistrationCredits: false,
    clientAppIds: [],
    entitlementMappingIds: [],
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
        receivesRegistrationCredits: bucket.receivesRegistrationCredits,
        clientAppIds: bucket.clientApps.map((c) => c.id),
        entitlementMappingIds: bucket.entitlementMappings.map((mp) => mp.id),
      },
      { keepDefaultValues: true }
    )
    // eslint-disable-next-line react-hooks/exhaustive-deps -- reset only on bucket/form identity change
  }, [formKey])

  const [registrationConflict, setRegistrationConflict] = useState<string | null>(null)

  function handleSubmissionError(error: unknown) {
    const code = readErrorCode(error)
    if (code === 'registration_pool_conflict') {
      setRegistrationConflict(m['credit_buckets.registration_pool_conflict']())
      return
    }
    toast.error(readErrorMessage(error))
  }

  // Clear the conflict hint whenever the toggle flips back off.
  useEffect(() => {
    setRegistrationConflict(null)
  }, [formKey])

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
              clientAppOptions={clientAppOptions}
              mappingOptions={mappingOptions}
              multiselectsLoading={clientAppsLoading || mappingsLoading}
              registrationConflict={registrationConflict}
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
            clientAppOptions={clientAppOptions}
            mappingOptions={mappingOptions}
            multiselectsLoading={clientAppsLoading || mappingsLoading}
            registrationConflict={registrationConflict}
          />
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
  clientAppOptions,
  mappingOptions,
  multiselectsLoading,
  registrationConflict,
}: {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  form: any
  clientAppOptions: { id: string; label: string; hint?: string }[]
  mappingOptions: { id: string; label: string; hint?: string }[]
  multiselectsLoading: boolean
  registrationConflict: string | null
}) {
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

      <form.Field name="receivesRegistrationCredits">
        {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
        {(field: any) => (
          <div className="space-y-1.5">
            <div className="flex items-center gap-2">
              <Switch
                id="bucket-registration"
                checked={field.state.value}
                onCheckedChange={field.handleChange}
                data-testid="credit-bucket-editor-registration"
              />
              <Label htmlFor="bucket-registration">
                {m['credit_buckets.field_registration']()}
              </Label>
            </div>
            <p className="text-xs text-muted-foreground">
              {m['credit_buckets.field_registration_help']()}
            </p>
          </div>
        )}
      </form.Field>

      {registrationConflict && (
        <Alert variant="destructive" data-testid="credit-bucket-editor-registration-conflict">
          <AlertDescription>{registrationConflict}</AlertDescription>
        </Alert>
      )}

      {multiselectsLoading ? (
        <Skeleton className="h-10 w-full" />
      ) : (
        <form.Field name="clientAppIds">
          {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
          {(field: any) => (
            <div className="space-y-1.5">
              <Label>{m['credit_buckets.field_coverage']()}</Label>
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
            </div>
          )}
        </form.Field>
      )}

      {multiselectsLoading ? (
        <Skeleton className="h-10 w-full" />
      ) : (
        <form.Field name="entitlementMappingIds">
          {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
          {(field: any) => (
            <div className="space-y-1.5">
              <Label>{m['credit_buckets.field_mappings']()}</Label>
              <CreditBucketMappingsMultiselect
                options={mappingOptions}
                value={field.state.value ?? []}
                onChange={field.handleChange}
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

/** Extract an error `code` from a thrown API error object (409 bodies, etc.). */
function readErrorCode(error: unknown): string | undefined {
  if (error && typeof error === 'object') {
    if ('code' in error && typeof (error as { code: unknown }).code === 'string') {
      return (error as { code: string }).code
    }
    const content = error as { content?: unknown; error?: unknown }
    if (content.content && typeof content.content === 'object') {
      return readErrorCode(content.content)
    }
  }
  return undefined
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

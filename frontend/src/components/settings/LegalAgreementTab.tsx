import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { z } from 'zod'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { TextareaField } from '@/components/shared/form-fields/textarea-field'
import { TextField } from '@/components/shared/form-fields/text-field'
import {
  legalAdminAgreementsQueryOptions,
  queryKeys,
  publishCustomAgreementMutation,
  revertToDefaultAgreementMutation,
} from '@/data/query-options'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { m } from '@/paraglide/messages'
import { formatDate } from '@/lib/date-utils'
import { AlertCircle } from 'lucide-react'
import type { AdminAgreementView, AgreementType, PublishCustomRequest } from '@/lib/api-generated'

const publishSchema = z
  .object({
    versionLabel: z.string().max(100, {
      error: () => m['settings.legal.version_label_max_length'](),
    }),
    contentEn: z.string(),
  })
  .refine((value) => value.contentEn.trim().length > 0, {
    error: () => m['settings.legal.content_required'](),
    path: ['contentEn'],
  })

const SUPPORTED_LOCALES = ['en'] as const

function getAgreementTitle(agreementType: AgreementType): string {
  if (agreementType === 'terms_of_service') {
    return m['legal.terms_of_service']()
  }
  if (agreementType === 'privacy_policy') {
    return m['legal.privacy_policy']()
  }
  return agreementType
}

function SourceBadge({ source }: { source: 'default' | 'custom' }) {
  const isCustom = source === 'custom'
  return (
    <Badge variant={isCustom ? 'secondary' : 'default'} data-testid={`source-badge-${source}`}>
      {isCustom ? m['settings.legal.source_custom']() : m['settings.legal.source_default']()}
    </Badge>
  )
}

function HistoryTable({ view }: { view: AdminAgreementView }) {
  if (view.history.length === 0) {
    return (
      <p
        className="text-sm text-muted-foreground"
        data-testid={`legal-history-empty-${view.agreement_type}`}
      >
        {m['settings.legal.history_empty']()}
      </p>
    )
  }

  return (
    <Table data-testid={`legal-history-table-${view.agreement_type}`}>
      <TableHeader>
        <TableRow>
          <TableHead>{m['settings.legal.history_version']()}</TableHead>
          <TableHead>{m['settings.legal.version_label_label']()}</TableHead>
          <TableHead>{m['settings.legal.history_source']()}</TableHead>
          <TableHead>{m['settings.legal.history_effective_date']()}</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {view.history.map((entry) => (
          <TableRow
            key={entry.version_id}
            data-testid={`legal-history-row-${view.agreement_type}-${entry.version_id}`}
          >
            <TableCell>{entry.version_no}</TableCell>
            <TableCell>{entry.version_label ?? '-'}</TableCell>
            <TableCell>
              <SourceBadge source={entry.source} />
            </TableCell>
            <TableCell>{formatDate(entry.effective_at)}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}

function AgreementCard({
  view,
  realmId,
  canManage,
}: {
  view: AdminAgreementView
  realmId: string
  canManage: boolean
}) {
  const [revertOpen, setRevertOpen] = useState(false)
  const agreementType = view.agreement_type
  const title = getAgreementTitle(agreementType)

  const publish = useFormMutation({
    mutationFn: (data: PublishCustomRequest) =>
      publishCustomAgreementMutation(realmId, agreementType, data),
    getSuccessMessage: () => m['settings.legal.publish_success'](),
    invalidateQueries: [
      queryKeys.legalAdminAgreements(realmId),
      queryKeys.legalAgreements(realmId),
      queryKeys.consentStatus(realmId),
    ],
  })

  const revert = useFormMutation({
    mutationFn: () => revertToDefaultAgreementMutation(realmId, agreementType),
    getSuccessMessage: () => m['settings.legal.revert_success'](),
    invalidateQueries: [
      queryKeys.legalAdminAgreements(realmId),
      queryKeys.legalAgreements(realmId),
      queryKeys.consentStatus(realmId),
    ],
    onSuccess: () => setRevertOpen(false),
  })

  const form = useAppForm({
    schema: publishSchema,
    defaultValues: {
      versionLabel: '',
      contentEn: '',
    },
    onSubmit: async ({ value }) => {
      const content: Record<string, string> = {}
      if (value.contentEn.trim()) {
        content[SUPPORTED_LOCALES[0]] = value.contentEn.trim()
      }

      await publish.mutate({
        content,
        version_label: value.versionLabel.trim() || null,
      })
      form.reset({ versionLabel: '', contentEn: '' })
    },
  })

  async function handleRevert() {
    await revert.mutate(undefined)
  }

  return (
    <Card data-testid={`legal-agreement-card-${agreementType}`}>
      <CardHeader>
        <div className="flex items-center justify-between gap-4">
          <CardTitle data-testid={`legal-agreement-title-${agreementType}`}>{title}</CardTitle>
          <SourceBadge source={view.source} />
        </div>
        <CardDescription data-testid={`legal-agreement-meta-${agreementType}`}>
          {m['settings.legal.version_no_label']()}: {view.current_version.version_no} •{' '}
          {m['settings.legal.effective_date_label']()}:{' '}
          {formatDate(view.current_version.effective_at)}
        </CardDescription>
      </CardHeader>

      <CardContent className="space-y-6">
        {canManage ? (
          <AppForm>
            <form
              id={`legal-publish-form-${agreementType}`}
              onSubmit={(e) => {
                e.preventDefault()
                void form.handleSubmit()
              }}
              className="space-y-4"
            >
              <TextField
                form={form}
                name="versionLabel"
                label={m['settings.legal.version_label_label']()}
                inputId={`legal-version-label-${agreementType}`}
                dataTestId={`legal-version-label-input-${agreementType}`}
                disabled={publish.isSubmitting}
                placeholder={m['settings.legal.version_label_placeholder']()}
              />
              <TextareaField
                form={form}
                name="contentEn"
                label={m['settings.legal.content_en_label']()}
                inputId={`legal-content-en-${agreementType}`}
                dataTestId={`legal-content-en-input-${agreementType}`}
                disabled={publish.isSubmitting}
                rows={6}
              />
              <Button
                type="submit"
                disabled={publish.isSubmitting}
                data-testid={`legal-publish-button-${agreementType}`}
              >
                {publish.isSubmitting
                  ? m['settings.legal.publishing_button']()
                  : m['settings.legal.publish_button']()}
              </Button>
            </form>
          </AppForm>
        ) : (
          <p
            className="text-sm text-muted-foreground"
            data-testid={`legal-agreement-view-only-${agreementType}`}
          >
            {m['settings.legal.view_only_notice']()}
          </p>
        )}

        {canManage && view.source === 'custom' && (
          <div>
            <Button
              variant="outline"
              onClick={() => setRevertOpen(true)}
              disabled={revert.isSubmitting}
              data-testid={`legal-revert-button-${agreementType}`}
            >
              {revert.isSubmitting
                ? m['settings.legal.reverting_button']()
                : m['settings.legal.revert_button']()}
            </Button>
          </div>
        )}

        <div>
          <h4
            className="text-sm font-medium mb-2"
            data-testid={`legal-history-title-${agreementType}`}
          >
            {m['settings.legal.history_title']()}
          </h4>
          <HistoryTable view={view} />
        </div>

        <AlertDialog open={revertOpen} onOpenChange={setRevertOpen}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle data-testid={`legal-revert-dialog-title-${agreementType}`}>
                {m['settings.legal.revert_dialog_title']()}
              </AlertDialogTitle>
              <AlertDialogDescription
                data-testid={`legal-revert-dialog-description-${agreementType}`}
              >
                {m['settings.legal.revert_dialog_description']()}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel
                onClick={() => setRevertOpen(false)}
                disabled={revert.isSubmitting}
                data-testid={`legal-revert-cancel-${agreementType}`}
              >
                {m['common.cancel']()}
              </AlertDialogCancel>
              <AlertDialogAction
                onClick={handleRevert}
                disabled={revert.isSubmitting}
                data-testid={`legal-revert-confirm-${agreementType}`}
              >
                {revert.isSubmitting
                  ? m['settings.legal.reverting_button']()
                  : m['settings.legal.revert_button']()}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </CardContent>
    </Card>
  )
}

export function LegalAgreementTab({ realmId, canManage }: { realmId: string; canManage: boolean }) {
  const { data, isLoading, error, refetch } = useQuery(legalAdminAgreementsQueryOptions(realmId))

  if (isLoading) {
    return (
      <div className="text-sm text-muted-foreground" data-testid="legal-agreements-loading">
        {m['settings.legal.loading']()}
      </div>
    )
  }

  if (error) {
    return (
      <Alert variant="destructive" data-testid="legal-agreements-error">
        <AlertCircle className="h-4 w-4" />
        <AlertTitle>{m['settings.legal.error']()}</AlertTitle>
        <AlertDescription className="space-y-2">
          <p>{error instanceof Error ? error.message : m['error.generic']()}</p>
          <Button
            variant="outline"
            size="sm"
            onClick={() => refetch()}
            data-testid="legal-agreements-retry"
          >
            {m['settings.legal.error_retry']()}
          </Button>
        </AlertDescription>
      </Alert>
    )
  }

  const agreements = data?.agreements ?? []

  if (agreements.length === 0) {
    return (
      <div className="text-sm text-muted-foreground" data-testid="legal-agreements-empty">
        {m['settings.legal.empty']()}
      </div>
    )
  }

  return (
    <div className="space-y-6" data-testid="legal-agreements-tab">
      {!canManage && (
        <Alert data-testid="legal-agreements-view-only">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>{m['settings.legal.view_only_title']()}</AlertTitle>
          <AlertDescription>{m['settings.legal.view_only_notice']()}</AlertDescription>
        </Alert>
      )}
      {agreements.map((view) => (
        <AgreementCard
          key={view.agreement_type}
          view={view}
          realmId={realmId}
          canManage={canManage}
        />
      ))}
    </div>
  )
}

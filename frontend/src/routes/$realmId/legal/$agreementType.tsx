import { createFileRoute, Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { AuthPageWrapper } from '@/components/auth/auth-page-wrapper'
import { legalAgreementQueryOptions } from '@/data/query-options'
import { useLocale } from '@/components/shared/locale-provider'
import { m } from '@/paraglide/messages'
import type { LegalAgreementDetail } from '@/lib/api-generated'

const VALID_AGREEMENT_TYPES: readonly string[] = ['terms_of_service', 'privacy_policy']

export const Route = createFileRoute('/$realmId/legal/$agreementType')({
  component: LegalAgreementPage,
})

function formatEffectiveDate(isoDate: string): string {
  try {
    return new Date(isoDate).toLocaleDateString()
  } catch {
    return isoDate
  }
}

function getAgreementTitle(agreementType: string): string {
  if (agreementType === 'terms_of_service') {
    return m['legal.terms_of_service']()
  }
  if (agreementType === 'privacy_policy') {
    return m['legal.privacy_policy']()
  }
  return agreementType
}

function renderBody(content: LegalAgreementDetail['content']): React.ReactNode {
  if (typeof content === 'string') {
    return (
      <div className="whitespace-pre-wrap text-sm leading-relaxed" data-testid="agreement-body">
        {content}
      </div>
    )
  }

  if (content === null || content === undefined) {
    return (
      <p className="text-muted-foreground" data-testid="agreement-empty-body">
        {m['legal.empty_body']()}
      </p>
    )
  }

  return (
    <pre className="whitespace-pre-wrap text-sm leading-relaxed" data-testid="agreement-body">
      {JSON.stringify(content, null, 2)}
    </pre>
  )
}

export function LegalAgreementPage() {
  const { realmId, agreementType } = Route.useParams()
  const { locale } = useLocale()

  const isValidType = VALID_AGREEMENT_TYPES.includes(agreementType)

  const { data, isLoading, error } = useQuery(
    legalAgreementQueryOptions(realmId, agreementType, locale)
  )

  if (!isValidType) {
    return (
      <AuthPageWrapper>
        <Card className="w-full max-w-2xl" data-testid="agreement-invalid-type">
          <CardHeader>
            <CardTitle>{m['legal.invalid_type_title']()}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <p className="text-muted-foreground">{m['legal.invalid_type_description']()}</p>
            <Button asChild variant="outline">
              <Link to="/$realmId/auth/login" params={{ realmId }}>
                {m['auth.register.return_to_login']()}
              </Link>
            </Button>
          </CardContent>
        </Card>
      </AuthPageWrapper>
    )
  }

  if (isLoading) {
    return (
      <AuthPageWrapper>
        <Card className="w-full max-w-2xl" data-testid="agreement-loading">
          <CardContent className="py-8">
            <p className="text-center text-muted-foreground">{m['common.loading']()}</p>
          </CardContent>
        </Card>
      </AuthPageWrapper>
    )
  }

  if (error) {
    return (
      <AuthPageWrapper>
        <Card className="w-full max-w-2xl" data-testid="agreement-error">
          <CardHeader>
            <CardTitle>{m['legal.error_title']()}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <p className="text-muted-foreground">
              {error instanceof Error ? error.message : m['error.generic']()}
            </p>
            <Button asChild variant="outline">
              <Link to="/$realmId/auth/login" params={{ realmId }}>
                {m['auth.register.return_to_login']()}
              </Link>
            </Button>
          </CardContent>
        </Card>
      </AuthPageWrapper>
    )
  }

  if (!data) {
    return (
      <AuthPageWrapper>
        <Card className="w-full max-w-2xl" data-testid="agreement-not-found">
          <CardHeader>
            <CardTitle>{m['legal.not_found_title']()}</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-muted-foreground">{m['legal.not_found_description']()}</p>
          </CardContent>
        </Card>
      </AuthPageWrapper>
    )
  }

  return (
    <AuthPageWrapper>
      <Card className="w-full max-w-2xl" data-testid="agreement-card">
        <CardHeader className="space-y-2">
          <CardTitle data-testid="agreement-title">{getAgreementTitle(agreementType)}</CardTitle>
          <div className="text-sm text-muted-foreground space-y-1">
            <p data-testid="agreement-version">
              {m['legal.version_label']()}: {data.version_no}
            </p>
            <p data-testid="agreement-effective-date">
              {m['legal.effective_date_label']()}: {formatEffectiveDate(data.effective_at)}
            </p>
          </div>
        </CardHeader>
        <CardContent>{renderBody(data.content)}</CardContent>
      </Card>
    </AuthPageWrapper>
  )
}

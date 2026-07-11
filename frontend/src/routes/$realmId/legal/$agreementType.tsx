import { createFileRoute, Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { legalAgreementQueryOptions } from '@/data/query-options'
import { MarkdownContent } from '@/components/legal/MarkdownContent'
import { m } from '@/paraglide/messages'
import {
  realmPath,
  useOptionalRouteParams,
  usePathSegments,
  useResolvedRealmContext,
} from '@/lib/realm-routing'
import type { LegalAgreementDetail } from '@/lib/api-generated'
import { getErrorMessage } from '@/lib/error-utils'

const VALID_AGREEMENT_TYPES: readonly string[] = ['terms_of_service', 'privacy_policy']

export const Route = createFileRoute('/$realmId/legal/$agreementType')({
  component: LegalAgreementPage,
})

function renderBody(content: LegalAgreementDetail['content']): React.ReactNode {
  if (typeof content === 'string') {
    return <MarkdownContent content={content} />
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
  const routeParams = useOptionalRouteParams<{ realmId?: string; agreementType?: string }>(Route)
  const resolvedRealmContext = useResolvedRealmContext()
  const realmContext = routeParams.realmId
    ? { ...resolvedRealmContext, realmId: routeParams.realmId, isCustomDomain: false }
    : resolvedRealmContext
  const realmId = realmContext.realmId
  const lastSegment = usePathSegments().at(-1) ?? ''
  const agreementType = routeParams.agreementType ?? lastSegment

  const isValidType = VALID_AGREEMENT_TYPES.includes(agreementType)

  // Legal agreements are English-only; ignore the UI locale when fetching content.
  const { data, isLoading, error } = useQuery(
    legalAgreementQueryOptions(realmId, agreementType, 'en')
  )

  if (!isValidType) {
    return (
      <Card className="w-full max-w-2xl" data-testid="agreement-invalid-type">
        <CardHeader>
          <CardTitle>{m['legal.invalid_type_title']()}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-muted-foreground">{m['legal.invalid_type_description']()}</p>
          <Button asChild variant="outline">
            <Link to={realmPath(realmContext, '/auth/login')}>
              {m['auth.register.return_to_login']()}
            </Link>
          </Button>
        </CardContent>
      </Card>
    )
  }

  if (isLoading) {
    return (
      <Card className="w-full max-w-2xl" data-testid="agreement-loading">
        <CardContent className="py-8">
          <p className="text-center text-muted-foreground">{m['common.loading']()}</p>
        </CardContent>
      </Card>
    )
  }

  if (error) {
    return (
      <Card className="w-full max-w-2xl" data-testid="agreement-error">
        <CardHeader>
          <CardTitle>{m['legal.error_title']()}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-muted-foreground">{getErrorMessage(error)}</p>
          <Button asChild variant="outline">
            <Link to={realmPath(realmContext, '/auth/login')}>
              {m['auth.register.return_to_login']()}
            </Link>
          </Button>
        </CardContent>
      </Card>
    )
  }

  if (!data) {
    return (
      <Card className="w-full max-w-2xl" data-testid="agreement-not-found">
        <CardHeader>
          <CardTitle>{m['legal.not_found_title']()}</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-muted-foreground">{m['legal.not_found_description']()}</p>
        </CardContent>
      </Card>
    )
  }

  if (data.mode === 'link' && data.external_url) {
    return (
      <Card className="w-full max-w-2xl" data-testid="agreement-external-link">
        <CardContent className="space-y-4 pt-6">
          <p className="text-muted-foreground">{m['legal.external_link_description']()}</p>
          <Button asChild>
            <a href={data.external_url} target="_blank" rel="noopener noreferrer">
              {m['legal.external_link_button']()}
            </a>
          </Button>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card className="w-full max-w-3xl" data-testid="agreement-card">
      <CardContent className="pt-6">{renderBody(data.content)}</CardContent>
    </Card>
  )
}

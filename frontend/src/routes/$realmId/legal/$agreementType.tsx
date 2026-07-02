import { createFileRoute, Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { legalAgreementQueryOptions } from '@/data/query-options'
import { MarkdownContent } from '@/components/legal/MarkdownContent'
import { m } from '@/paraglide/messages'
import type { LegalAgreementDetail } from '@/lib/api-generated'

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
  const { realmId, agreementType } = Route.useParams()

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
            <Link to="/$realmId/auth/login" params={{ realmId }}>
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

  return (
    <Card className="w-full max-w-3xl" data-testid="agreement-card">
      <CardContent className="pt-6">{renderBody(data.content)}</CardContent>
    </Card>
  )
}

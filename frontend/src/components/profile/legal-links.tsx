import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { AgreementLinks } from '@/components/legal/AgreementLinks'
import { m } from '@/paraglide/messages'

interface LegalLinksProps {
  realmId: string
}

export function LegalLinks({ realmId }: LegalLinksProps) {
  return (
    <Card data-testid="legal-links-card">
      <CardHeader>
        <CardTitle data-testid="legal-links-title">
          {m['legal.terms_of_service']()} {m['legal.and_separator']()} {m['legal.privacy_policy']()}
        </CardTitle>
      </CardHeader>
      <CardContent data-testid="legal-links-content">
        <AgreementLinks realmId={realmId} />
      </CardContent>
    </Card>
  )
}

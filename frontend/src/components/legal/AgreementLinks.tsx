import { Link } from '@tanstack/react-router'
import { m } from '@/paraglide/messages'

export type AgreementType = 'terms_of_service' | 'privacy_policy'

interface AgreementLinksProps {
  realmId: string
  agreementType?: AgreementType
  beforeText?: string
  className?: string
  linkClassName?: string
}

function getAgreementLabel(type: AgreementType): string {
  if (type === 'terms_of_service') {
    return m['legal.terms_of_service']()
  }
  return m['legal.privacy_policy']()
}

export function AgreementLinks({
  realmId,
  agreementType,
  beforeText,
  className,
  linkClassName,
}: AgreementLinksProps) {
  const linkBaseClasses =
    linkClassName ?? 'text-primary hover:text-primary/80 underline underline-offset-2'

  if (agreementType) {
    const testId =
      agreementType === 'terms_of_service' ? 'terms-of-service-link' : 'privacy-policy-link'
    return (
      <span className={className}>
        {beforeText}
        <Link
          to="/$realmId/legal/$agreementType"
          params={{ realmId, agreementType }}
          className={linkBaseClasses}
          data-testid={testId}
        >
          {getAgreementLabel(agreementType)}
        </Link>
      </span>
    )
  }

  return (
    <span className={className}>
      {beforeText}
      <Link
        to="/$realmId/legal/$agreementType"
        params={{ realmId, agreementType: 'terms_of_service' }}
        className={linkBaseClasses}
        data-testid="terms-of-service-link"
      >
        {m['legal.terms_of_service']()}
      </Link>
      {m['legal.and_separator']()}
      <Link
        to="/$realmId/legal/$agreementType"
        params={{ realmId, agreementType: 'privacy_policy' }}
        className={linkBaseClasses}
        data-testid="privacy-policy-link"
      >
        {m['legal.privacy_policy']()}
      </Link>
    </span>
  )
}

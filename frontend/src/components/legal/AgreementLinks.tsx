import { Link } from '@tanstack/react-router'
import { useEffect, useState } from 'react'
import { listAgreements } from '@/lib/api-generated'
import type { LegalAgreementSummary } from '@/lib/api-generated'
import { m } from '@/paraglide/messages'

export type AgreementType = 'terms_of_service' | 'privacy_policy'

interface AgreementLinksProps {
  realmId: string
  agreementType?: AgreementType
  beforeText?: string
  className?: string
  linkClassName?: string
  agreements?: LegalAgreementSummary[]
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
  agreements: providedAgreements,
}: AgreementLinksProps) {
  const [fetched, setFetched] = useState<LegalAgreementSummary[] | undefined>(undefined)
  useEffect(() => {
    if (providedAgreements) return
    let active = true
    void listAgreements({ path: { realmId } }).then((response) => {
      if (active && response.data) setFetched(response.data.agreements)
    })
    return () => {
      active = false
    }
  }, [providedAgreements, realmId])
  const agreements = providedAgreements ?? fetched
  const linkBaseClasses =
    linkClassName ?? 'text-primary hover:text-primary/80 underline underline-offset-2'

  if (agreementType) {
    const testId =
      agreementType === 'terms_of_service' ? 'terms-of-service-link' : 'privacy-policy-link'
    const agreement = agreements?.find((item) => item.agreement_type === agreementType)
    if (agreement?.mode === 'link' && agreement.external_url) {
      return (
        <span className={className}>
          {beforeText}
          <a
            href={agreement.external_url}
            target="_blank"
            rel="noopener noreferrer"
            className={linkBaseClasses}
            data-testid={testId}
          >
            {getAgreementLabel(agreementType)}
          </a>
        </span>
      )
    }
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

  const external = Object.fromEntries((agreements ?? []).map((item) => [item.agreement_type, item]))
  const renderAnchor = (type: AgreementType, testId: string) => {
    const item = external[type]
    if (item?.mode === 'link' && item.external_url) {
      return (
        <a
          href={item.external_url}
          target="_blank"
          rel="noopener noreferrer"
          className={linkBaseClasses}
          data-testid={testId}
        >
          {getAgreementLabel(type)}
        </a>
      )
    }
    return (
      <Link
        to="/$realmId/legal/$agreementType"
        params={{ realmId, agreementType: type }}
        className={linkBaseClasses}
        data-testid={testId}
      >
        {getAgreementLabel(type)}
      </Link>
    )
  }

  return (
    <span className={className}>
      {beforeText}
      {renderAnchor('terms_of_service', 'terms-of-service-link')}
      {m['legal.and_separator']()}
      {renderAnchor('privacy_policy', 'privacy-policy-link')}
    </span>
  )
}

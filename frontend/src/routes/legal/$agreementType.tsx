import { createFileRoute } from '@tanstack/react-router'
import { LegalAgreementPage } from '@/routes/$realmId/legal/$agreementType'

export const Route = createFileRoute('/legal/$agreementType')({
  component: LegalAgreementPage,
})

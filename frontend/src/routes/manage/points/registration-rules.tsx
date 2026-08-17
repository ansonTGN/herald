import { createFileRoute } from '@tanstack/react-router'
import { RegistrationRulesPage } from '@/routes/$realmId/manage/points/registration-rules'

export const Route = createFileRoute('/manage/points/registration-rules')({
  component: RegistrationRulesPage,
})

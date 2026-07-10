import { createFileRoute } from '@tanstack/react-router'
import { UserInvoicesIndexRoute } from '@/routes/$realmId/user/invoices/'

export const Route = createFileRoute('/user/invoices/')({
  component: UserInvoicesIndexRoute,
})

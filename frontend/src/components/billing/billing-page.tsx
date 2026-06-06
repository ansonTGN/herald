import { Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { CreditCard, FileText, History, Layers, ListChecks } from 'lucide-react'
import { featureAvailabilityQueryOptions } from '@/data/query-options'
import { m } from '@/paraglide/messages'
import type { LucideIcon } from 'lucide-react'

interface BillingPageProps {
  realmId: string
}

interface NavCard {
  title: string
  description: string
  route: string
  icon: LucideIcon
  visible: boolean
  testId: string
}

export function BillingPage({ realmId }: BillingPageProps) {
  const { data: features } = useQuery(featureAvailabilityQueryOptions(realmId))
  const adminFeatures = features?.admin

  const cards: NavCard[] = [
    {
      title: m['billing.nav_entitlement_mappings'](),
      description: m['billing.nav_entitlement_mappings_desc'](),
      route: `/${realmId}/manage/billing/entitlement-mappings`,
      icon: Layers,
      visible: adminFeatures?.entitlementMappingsVisible ?? true,
      testId: 'billing-nav-entitlement-mappings',
    },
    {
      title: m['billing.nav_subscriptions'](),
      description: m['billing.nav_subscriptions_desc'](),
      route: `/${realmId}/manage/billing/subscriptions`,
      icon: ListChecks,
      visible: adminFeatures?.billingVisible ?? true,
      testId: 'billing-nav-subscriptions',
    },
    {
      title: m['billing.nav_payment_providers'](),
      description: m['billing.nav_payment_providers_desc'](),
      route: `/${realmId}/manage/billing/payment-providers`,
      icon: CreditCard,
      visible: adminFeatures?.billingConfigVisible ?? true,
      testId: 'billing-nav-payment-providers',
    },
    {
      title: m['billing.nav_invoices'](),
      description: m['billing.nav_invoices_desc'](),
      route: `/${realmId}/manage/billing/invoices`,
      icon: FileText,
      visible: adminFeatures?.invoicesVisible ?? true,
      testId: 'billing-nav-invoices',
    },
    {
      title: m['billing.nav_subscription_history'](),
      description: m['billing.nav_subscription_history_desc'](),
      route: `/${realmId}/manage/subscription-history`,
      icon: History,
      visible: adminFeatures?.subscriptionHistoryVisible ?? true,
      testId: 'billing-nav-subscription-history',
    },
  ]

  const visibleCards = cards.filter((card) => card.visible)

  return (
    <div data-testid="billing-page" className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">{m['billing.page_title']()}</h1>
        <p className="text-muted-foreground mt-1">{m['billing.page_description']()}</p>
      </div>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {visibleCards.map((card) => {
          const Icon = card.icon
          return (
            <Link
              key={card.testId}
              to={card.route}
              className="group rounded-lg border bg-card p-6 transition-colors hover:bg-accent hover:text-accent-foreground"
              data-testid={card.testId}
            >
              <div className="flex items-center gap-3">
                <div className="rounded-md bg-primary/10 p-2">
                  <Icon className="size-5 text-primary" />
                </div>
                <h3 className="font-semibold">{card.title}</h3>
              </div>
              <p className="mt-2 text-sm text-muted-foreground">{card.description}</p>
            </Link>
          )
        })}
      </div>
    </div>
  )
}

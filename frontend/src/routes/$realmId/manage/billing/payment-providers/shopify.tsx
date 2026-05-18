import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { lazy, Suspense } from 'react'
import { SpinnerFallback } from '@/components/shared'
import { listPaymentProviders, getShopifyConfig } from '@/lib/api-generated'
import type { ShopifyConfigForm } from '@/lib/schemas/billing-forms'

const ShopifyConfigFormPage = lazy(() =>
  import('@/components/billing/ShopifyConfigForm').then((m) => ({
    default: m.ShopifyConfigFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/billing/payment-providers/shopify')({
  component: ShopifyConfigRoute,
})

function ShopifyConfigRoute() {
  const { realmId } = Route.useParams()

  const { data: providers, isLoading } = useQuery({
    queryKey: ['payment-providers', realmId],
    queryFn: async () => {
      const result = await listPaymentProviders({ path: { realmId } })
      return result.data?.providers ?? []
    },
  })

  const shopifyProvider = providers?.find((p) => p.platform === 'shopify')
  const mode = shopifyProvider ? 'edit' : 'create'

  const { data: configData } = useQuery({
    queryKey: ['shopify-config', realmId],
    queryFn: async () => {
      if (!shopifyProvider) return null
      const result = await getShopifyConfig({ path: { realmId } })
      return result.data
    },
    enabled: !!shopifyProvider,
  })

  const initialValues: Partial<ShopifyConfigForm> | undefined = configData
    ? {
        shopDomain: configData.shopDomain || shopifyProvider?.shopDomain || '',
        adminAccessToken: '',
        storefrontAccessToken: '',
        appClientSecret: '',
        apiVersion: configData.apiVersion,
        webhookSubscriptionMode: configData.webhookSubscriptionMode as 'admin_api' | 'event_bridge',
        timeout: configData.timeout,
      }
    : undefined

  if (isLoading) {
    return <SpinnerFallback />
  }

  return (
    <Suspense fallback={<SpinnerFallback />}>
      <ShopifyConfigFormPage realmId={realmId} mode={mode} initialValues={initialValues} />
    </Suspense>
  )
}

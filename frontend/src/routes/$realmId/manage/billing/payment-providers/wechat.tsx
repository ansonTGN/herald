import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { lazy, Suspense } from 'react'
import { SpinnerFallback } from '@/components/shared'
import { listPaymentProviders, getWechatConfig } from '@/lib/api-generated'
import type { WechatConfigForm } from '@/lib/schemas/billing-forms'

const WechatConfigFormPage = lazy(() =>
  import('@/components/billing/WechatConfigForm').then((m) => ({ default: m.WechatConfigFormPage }))
)

export const Route = createFileRoute('/$realmId/manage/billing/payment-providers/wechat')({
  component: WechatConfigRoute,
})

function WechatConfigRoute() {
  const { realmId } = Route.useParams()

  const { data: providers, isLoading } = useQuery({
    queryKey: ['payment-providers', realmId],
    queryFn: async () => {
      const result = await listPaymentProviders({ path: { realmId } })
      return result.data?.providers ?? []
    },
  })

  const wechatProvider = providers?.find((p) => p.platform === 'wechat')
  const mode = wechatProvider ? 'edit' : 'create'

  const { data: configData } = useQuery({
    queryKey: ['wechat-config', realmId],
    queryFn: async () => {
      if (!wechatProvider) return null
      const result = await getWechatConfig({ path: { realmId } })
      return result.data
    },
    enabled: !!wechatProvider,
  })

  const initialValues: Partial<WechatConfigForm> | undefined = configData
    ? {
        appId: configData.appId || '',
        mchId: configData.mchId || '',
        privateKey: '',
        serialNo: configData.serialNo || '',
        v3Key: '',
        notifyUrl: configData.notifyUrl || '',
      }
    : undefined

  if (isLoading) {
    return <SpinnerFallback />
  }

  return (
    <Suspense fallback={<SpinnerFallback />}>
      <WechatConfigFormPage realmId={realmId} mode={mode} initialValues={initialValues} />
    </Suspense>
  )
}

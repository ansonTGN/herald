import { createFileRoute } from '@tanstack/react-router'
import { WechatConfigRoute } from '@/routes/$realmId/manage/billing/payment-providers/wechat'

export const Route = createFileRoute('/manage/billing/payment-providers/wechat')({
  component: WechatConfigRoute,
})

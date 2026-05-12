import { http, HttpResponse } from 'msw'

/**
 * MSW handlers for WeChat Pay API endpoints
 *
 * These handlers mock the backend WeChat Pay integration endpoints for testing purposes.
 */

export const wechatPayHandlers = [
  // Create WeChat Pay order
  http.post('/api/third/pay/:realmId/wechat/create-order', async ({ params }) => {
    const { realmId } = params

    return HttpResponse.json({
      orderId: 'order-123',
      outTradeNo: `CAS_${realmId}_abc123`,
      codeUrl: 'weixin://wxpay/bizpayurl?pr=abc123',
      expiresAt: new Date(Date.now() + 2 * 60 * 60 * 1000).toISOString(),
    })
  }),

  // Query order status
  http.get('/api/third/pay/:realmId/wechat/order-status/:orderId', async ({ params }) => {
    const { orderId } = params

    // Mock different states based on orderId
    if (orderId === 'order-paid') {
      return HttpResponse.json({
        orderId: 'order-paid',
        status: 'paid',
        tradeState: 'SUCCESS',
      })
    }

    if (orderId === 'order-failed') {
      return HttpResponse.json({
        orderId: 'order-failed',
        status: 'failed',
        tradeState: 'PAYERROR',
      })
    }

    if (orderId === 'order-expired') {
      return HttpResponse.json({
        orderId: 'order-expired',
        status: 'expired',
      })
    }

    if (orderId === 'order-closed') {
      return HttpResponse.json({
        orderId: 'order-closed',
        status: 'closed',
      })
    }

    // Default pending state
    return HttpResponse.json({
      orderId,
      status: 'pending',
    })
  }),

  // Close order
  http.post('/api/third/pay/:realmId/wechat/close-order/:orderId', () => {
    return HttpResponse.json({
      orderId: 'order-123',
      status: 'closed',
    })
  }),

  // Create WeChat Pay config
  http.post('/api/third/pay/:realmId/providers/wechat', async ({ request }) => {
    const body = (await request.json()) as any

    // Validation errors
    if (body.appId && !body.appId.startsWith('wx')) {
      return HttpResponse.json({ message: 'App ID must start with "wx"' }, { status: 400 })
    }

    if (body.mchId && !body.mchId.match(/^\d+$/)) {
      return HttpResponse.json({ message: 'Merchant ID must be numeric' }, { status: 400 })
    }

    if (body.v3Key && body.v3Key.length !== 32) {
      return HttpResponse.json({ message: 'API v3 Key must be exactly 32 bytes' }, { status: 400 })
    }

    if (body.notifyUrl && !body.notifyUrl.startsWith('https://')) {
      return HttpResponse.json({ message: 'Notify URL must use HTTPS' }, { status: 400 })
    }

    // Simulate conflict error
    if (body.simulateConflict) {
      return HttpResponse.json(
        { message: 'WeChat Pay configuration already exists' },
        { status: 409 }
      )
    }

    return HttpResponse.json({
      platform: 'wechat',
      appId: body.appId,
      mchId: body.mchId,
      serialNo: body.serialNo,
      v3Key: 'my_v3_*******************',
      privateKey: '*********** (configured)',
      notifyUrl: body.notifyUrl,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    })
  }),

  // Get WeChat Pay config
  http.get('/api/third/pay/:realmId/providers/wechat', () => {
    return HttpResponse.json({
      platform: 'wechat',
      appId: 'wx1234567890abcdef',
      mchId: '1234567890',
      serialNo: '1A2B3C4D5E6F',
      v3Key: 'my_v3_*******************',
      privateKey: '*********** (configured)',
      notifyUrl: 'https://example.com/api/webhook',
      createdAt: '2026-04-05T10:00:00Z',
      updatedAt: '2026-04-05T10:00:00Z',
    })
  }),

  // Update WeChat Pay config
  http.put('/api/third/pay/:realmId/providers/wechat', async ({ request }) => {
    const body = (await request.json()) as any

    return HttpResponse.json({
      platform: 'wechat',
      appId: body.appId || 'wx1234567890abcdef',
      mchId: body.mchId || '1234567890',
      serialNo: body.serialNo || '1A2B3C4D5E6F',
      v3Key: 'my_v3_*******************',
      privateKey: '*********** (configured)',
      notifyUrl: body.notifyUrl || 'https://example.com/api/webhook',
      createdAt: '2026-04-05T10:00:00Z',
      updatedAt: new Date().toISOString(),
    })
  }),

  // Delete WeChat Pay config
  http.delete('/api/third/pay/:realmId/providers/wechat', async ({ request }) => {
    const url = new URL(request.url)
    if (url.searchParams.get('hasActiveSubscriptions') === 'true') {
      return HttpResponse.json(
        { message: 'Cannot delete configuration with active subscriptions' },
        { status: 409 }
      )
    }

    return new HttpResponse(null, { status: 204 })
  }),
]

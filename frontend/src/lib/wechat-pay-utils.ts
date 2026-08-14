import type { WechatJsapiParams } from '@/lib/api-generated'

export type WechatSceneDecision =
  | { scene: 'native' }
  | { scene: 'jsapi'; openid: string }
  | { scene: 'jsapi_unavailable' }

// WeChat's in-app browser (identifiable only via the MicroMessenger UA token)
// is the sole environment where JSAPI payment can be invoked; every other
// browser must use the Native scan-to-pay QR code.
export function isWeChatBrowser(userAgent: string): boolean {
  return /MicroMessenger/i.test(userAgent)
}

// The openid required by JSAPI is caller-provided (URL search param); when it
// is absent inside WeChat, ordering must be refused instead of silently
// falling back to a QR the same device cannot scan.
export function resolveWechatScene(
  openid: string | null | undefined,
  userAgent: string
): WechatSceneDecision {
  if (!isWeChatBrowser(userAgent)) {
    return { scene: 'native' }
  }
  if (openid) {
    return { scene: 'jsapi', openid }
  }
  return { scene: 'jsapi_unavailable' }
}

export type WechatJsapiInvokeResult = 'ok' | 'cancel' | 'fail' | 'bridge_unavailable'

interface WeixinJSBridge {
  invoke: (
    api: string,
    params: Record<string, unknown>,
    callback: (res: { err_msg?: string }) => void
  ) => void
}

declare global {
  interface Window {
    WeixinJSBridge?: WeixinJSBridge
  }
}

// Invoke WeChat's built-in payment sheet. Always resolves (never rejects) so
// callers can render feedback for every outcome, including the bridge being
// absent outside a real WeChat webview.
export function invokeWechatJsapiPay(params: WechatJsapiParams): Promise<WechatJsapiInvokeResult> {
  return new Promise((resolve) => {
    const bridge = window.WeixinJSBridge
    if (!bridge) {
      resolve('bridge_unavailable')
      return
    }
    bridge.invoke('getBrandWCPayRequest', { ...params }, (res) => {
      const errMsg = res.err_msg ?? ''
      if (errMsg === 'get_brand_wcpay_request:ok') {
        resolve('ok')
      } else if (errMsg === 'get_brand_wcpay_request:cancel') {
        resolve('cancel')
      } else {
        resolve('fail')
      }
    })
  })
}

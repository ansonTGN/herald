import { describe, it, expect, vi, afterEach } from 'vitest'
import type { WechatJsapiParams } from '@/lib/api-generated'
import { isWeChatBrowser, resolveWechatScene, invokeWechatJsapiPay } from '../wechat-pay-utils'

const IOS_WECHAT_UA =
  'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148 MicroMessenger/8.0.40(0x18002830) NetType/WIFI Language/zh_CN'
const ANDROID_WECHAT_UA =
  'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/126.0.0.0 Mobile Safari/537.36 MicroMessenger/8.0.40.2460(0x28002835) XWEB/1260065'
const DESKTOP_CHROME_UA =
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36'

describe('isWeChatBrowser', () => {
  it.each([
    ['iOS WeChat webview', IOS_WECHAT_UA],
    ['Android WeChat webview', ANDROID_WECHAT_UA],
    ['bare MicroMessenger token', 'MicroMessenger/8.0.40'],
  ])('detects %s', (_label, ua) => {
    expect(isWeChatBrowser(ua)).toBe(true)
  })

  it.each([
    ['desktop Chrome', DESKTOP_CHROME_UA],
    [
      'mobile Safari without WeChat',
      'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
    ],
    ['empty string', ''],
  ])('does not detect %s', (_label, ua) => {
    expect(isWeChatBrowser(ua)).toBe(false)
  })
})

describe('resolveWechatScene', () => {
  it('uses the Native QR scene outside WeChat, even when an openid was passed', () => {
    // JSAPI cannot be invoked outside WeChat's webview, so a stray openid
    // (e.g. a stale link) must not flip the scene.
    expect(resolveWechatScene('openid-1', DESKTOP_CHROME_UA)).toEqual({ scene: 'native' })
  })

  it('uses JSAPI inside WeChat when the caller provided an openid', () => {
    expect(resolveWechatScene('openid-1', IOS_WECHAT_UA)).toEqual({
      scene: 'jsapi',
      openid: 'openid-1',
    })
  })

  it.each([
    ['null', null],
    ['undefined', undefined],
    ['empty string', ''],
  ])('refuses to order inside WeChat when openid is %s', (_label, openid) => {
    // In-WeChat the same device cannot scan a Native QR, so the missing
    // openid must surface as an explicit unavailable state rather than a
    // silent fallback to the QR scene.
    expect(resolveWechatScene(openid, IOS_WECHAT_UA)).toEqual({
      scene: 'jsapi_unavailable',
    })
  })
})

const JSAPI_PARAMS: WechatJsapiParams = {
  appId: 'wx1234',
  timeStamp: '1723600000',
  nonceStr: 'nonce',
  package: 'prepay_id=wx123',
  signType: 'RSA',
  paySign: 'signature',
}

function mockBridge(errMsg: string | undefined) {
  const invoke = vi.fn(
    (
      _api: string,
      _params: Record<string, unknown>,
      callback: (res: { err_msg?: string }) => void
    ) => {
      callback(errMsg === undefined ? {} : { err_msg: errMsg })
    }
  )
  window.WeixinJSBridge = { invoke }
  return invoke
}

afterEach(() => {
  delete window.WeixinJSBridge
  vi.restoreAllMocks()
})

describe('invokeWechatJsapiPay', () => {
  it('resolves bridge_unavailable (never rejects) when WeixinJSBridge is absent', async () => {
    delete window.WeixinJSBridge
    await expect(invokeWechatJsapiPay(JSAPI_PARAMS)).resolves.toBe('bridge_unavailable')
  })

  it.each([
    ['get_brand_wcpay_request:ok', 'ok'],
    ['get_brand_wcpay_request:cancel', 'cancel'],
    ['get_brand_wcpay_request:fail', 'fail'],
    ['missing err_msg', 'fail'],
  ])('maps bridge result %s to %s', async (errMsg, expected) => {
    mockBridge(errMsg === 'missing err_msg' ? undefined : errMsg)
    await expect(invokeWechatJsapiPay(JSAPI_PARAMS)).resolves.toBe(expected)
  })

  it('invokes the official payment API with the prepay params', async () => {
    const invoke = mockBridge('get_brand_wcpay_request:ok')
    await invokeWechatJsapiPay(JSAPI_PARAMS)
    expect(invoke).toHaveBeenCalledTimes(1)
    expect(invoke.mock.calls[0][0]).toBe('getBrandWCPayRequest')
    expect(invoke.mock.calls[0][1]).toEqual({ ...JSAPI_PARAMS })
  })
})

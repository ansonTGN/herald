/**
 * WeChat Pay v3 Local Mock Server
 *
 * Simulates the two WeChat Pay v3 order endpoints the backend calls during a
 * demo purchase, so the unified-order flow can run without the real
 * api.mch.weixin.qq.com (no merchant qualification required).
 *
 * Reachability: the demo backend runs on the host (scripts/lib/demo_env.py
 * starts it via cargo; only Postgres/Redis are dockerized), so a mock bound to
 * 127.0.0.1 is directly reachable from the backend. The seeded realm_config
 * `base_url` key (`seedWechatConfig`) points the backend at this mock.
 *
 * Endpoints:
 * - POST /v3/pay/transactions/native → { code_url } (records the order)
 * - POST /v3/pay/transactions/jsapi  → { prepay_id } (records payer openid)
 *
 * Request signatures are accepted without verification — the demo asserts the
 * backend's OUTBOUND behavior (order body) and drives the INBOUND webhook via
 * `wechat-webhook-simulation.ts`, where signature crypto IS exercised for real.
 */

import { createServer, type IncomingMessage, type Server } from 'node:http'
import { randomUUID } from 'crypto'

/** One recorded Native (QR) unified order. */
export interface RecordedNativeOrder {
  outTradeNo: string
  appid: string
  mchid: string
  description: string
  amountTotal: number
  currency: string
  timeExpire: string
  notifyUrl: string
}

/** One recorded JSAPI unified order (carries the payer openid). */
export interface RecordedJsapiOrder extends RecordedNativeOrder {
  payerOpenid: string
}

export interface WechatPayMock {
  /** Base URL to seed into realm_config `base_url` (http://127.0.0.1:<port>). */
  url: string
  port: number
  /** Native orders in arrival order. */
  nativeOrders: RecordedNativeOrder[]
  /** JSAPI orders in arrival order. */
  jsapiOrders: RecordedJsapiOrder[]
  /** Resolves when the mock server has stopped. */
  close(): Promise<void>
}

interface OrderRequestBody {
  appid?: string
  mchid?: string
  description?: string
  out_trade_no?: string
  time_expire?: string
  notify_url?: string
  amount?: { total?: number; currency?: string }
  payer?: { openid?: string }
}

function readBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = []
    req.on('data', (chunk: Buffer) => chunks.push(chunk))
    req.on('end', () => resolve(Buffer.concat(chunks).toString('utf8')))
    req.on('error', reject)
  })
}

function toNativeOrder(body: OrderRequestBody): RecordedNativeOrder {
  if (!body.out_trade_no) {
    throw new Error('wechat-pay-mock: order body missing out_trade_no')
  }
  return {
    outTradeNo: body.out_trade_no,
    appid: body.appid ?? '',
    mchid: body.mchid ?? '',
    description: body.description ?? '',
    amountTotal: body.amount?.total ?? 0,
    currency: body.amount?.currency ?? '',
    timeExpire: body.time_expire ?? '',
    notifyUrl: body.notify_url ?? '',
  }
}

/**
 * Start the mock on an ephemeral port. Call `close()` in afterAll/afterEach.
 */
export async function startWechatPayMock(): Promise<WechatPayMock> {
  const nativeOrders: RecordedNativeOrder[] = []
  const jsapiOrders: RecordedJsapiOrder[] = []

  const server: Server = createServer((req, res) => {
    void (async () => {
      const url = req.url ?? ''
      if (req.method === 'POST' && url === '/v3/pay/transactions/native') {
        const order = toNativeOrder(JSON.parse(await readBody(req)) as OrderRequestBody)
        nativeOrders.push(order)
        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify({ code_url: `weixin://wxpay/demo/${randomUUID()}` }))
        return
      }
      if (req.method === 'POST' && url === '/v3/pay/transactions/jsapi') {
        const body = JSON.parse(await readBody(req)) as OrderRequestBody
        const order: RecordedJsapiOrder = {
          ...toNativeOrder(body),
          payerOpenid: body.payer?.openid ?? '',
        }
        jsapiOrders.push(order)
        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify({ prepay_id: `demo_prepay_${randomUUID()}` }))
        return
      }
      res.writeHead(404, { 'Content-Type': 'application/json' })
      res.end(JSON.stringify({ code: 'NOT_FOUND', message: `no mock route for ${req.method} ${url}` }))
    })().catch((err: unknown) => {
      res.writeHead(500, { 'Content-Type': 'application/json' })
      res.end(JSON.stringify({ code: 'MOCK_ERROR', message: String(err) }))
    })
  })

  await new Promise<void>((resolve, reject) => {
    server.once('error', reject)
    server.listen(0, '127.0.0.1', () => resolve())
  })

  const address = server.address()
  if (address === null || typeof address === 'string') {
    server.close()
    throw new Error(`wechat-pay-mock: unexpected listen address ${String(address)}`)
  }

  return {
    url: `http://127.0.0.1:${address.port}`,
    port: address.port,
    nativeOrders,
    jsapiOrders,
    close: () =>
      new Promise<void>((resolve, reject) => {
        server.close((err) => (err ? reject(err) : resolve()))
      }),
  }
}

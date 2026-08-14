/**
 * WeChat Pay v3 Webhook Simulation Helper
 *
 * Sends signed synthetic WeChat payment-result notifications to
 * `POST /api/third/pay/{realmId}/wechat/webhooks`, driving the real
 * verify → decrypt → idempotency → amount-check → fulfil pipeline without a
 * real WeChat merchant account.
 *
 * -------------------------------------------------------------------------
 * SOURCE OF TRUTH — backend verification:
 *
 * - Signature: `backend/infra-wechatpay/src/client.rs::verify_callback`
 *   builds `{timestamp}\n{nonce}\n{body}\n` (note the TRAILING newline) and
 *   delegates to `signing.rs::verify_callback_signature`
 *   (RSA-SHA256/PKCS1v15 over that UTF-8 message, base64 signature).
 *   Headers: `Wechatpay-Timestamp` / `Wechatpay-Nonce` / `Wechatpay-Signature`
 *   / `Wechatpay-Serial`.
 * - Platform key: the `platform_public_key` realm_config override is preferred
 *   over the auto-downloaded certificate cache
 *   (`client.rs::get_platform_public_key`), so seeding the override with a
 *   locally generated RSA keypair makes verification work with zero network —
 *   mirrors `backend/api/src/tests/helpers/wechat_mocks.rs`.
 * - Resource decryption: `client.rs::decrypt_resource` → AES-256-GCM
 *   (key = APIv3 Key, 12-byte nonce, AAD = `associated_data`) and the
 *   `EncryptedResource` shape is `{ ciphertext, associated_data, nonce }`.
 * - Rejections: signature/decrypt/amount/lookup failures reply
 *   422 `{"code":"FAIL"}` and MUST NOT mutate attempt state
 *   (`wechat_webhook_handlers.rs::handle_wechat_webhook`).
 *
 * -------------------------------------------------------------------------
 * BYTE-CONSISTENCY CAVEAT (load-bearing, mirrors webhook-renewal-simulation):
 * The signature is computed over the RAW body bytes that hit the wire. The
 * forge helper serializes the payload ONCE with `JSON.stringify` and the
 * deliver helper passes the resulting UTF-8 Buffer as Playwright's
 * `request.post(..., { data: Buffer })`. Passing a Buffer (not an object)
 * prevents Playwright from re-serializing the body and invalidating the
 * signature. Callers MUST NOT swap `data: rawBody` for `data: payload`.
 */

import { createCipheriv, createSign, generateKeyPairSync, randomBytes } from 'crypto'
import { type APIRequestContext, type APIResponse } from '@playwright/test'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'
const API_TIMEOUT = 10_000

/** Webhook endpoint path (backend/api-billing/src/routes.rs). */
export const WECHAT_WEBHOOK_ROUTE = (realmId: string): string =>
  `/api/third/pay/${realmId}/wechat/webhooks`

/** RSA keypair used for WeChat-side crypto in the demo (PKCS#8 / SPKI PEM). */
export interface WechatRsaKeyPair {
  privateKeyPem: string
  publicKeyPem: string
}

export function generateWechatRsaKeyPair(): WechatRsaKeyPair {
  const { publicKey, privateKey } = generateKeyPairSync('rsa', {
    modulusLength: 2048,
    publicKeyEncoding: { type: 'spki', format: 'pem' },
    privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
  })
  return { privateKeyPem: privateKey, publicKeyPem: publicKey }
}

/** All key material a wechat demo realm needs, generated in one call. */
export interface WechatDemoKeyMaterial {
  /** Merchant private key (order signing; only key validity matters for demo). */
  merchantPrivateKeyPem: string
  /** Platform keypair: public half is seeded as the `platform_public_key`
   * override, private half signs the forged callbacks. */
  platform: WechatRsaKeyPair
  /** Exactly 32 bytes — enforced by `WechatPayClient` construction. */
  apiV3Key: string
}

export function generateWechatDemoKeyMaterial(): WechatDemoKeyMaterial {
  return {
    merchantPrivateKeyPem: generateWechatRsaKeyPair().privateKeyPem,
    platform: generateWechatRsaKeyPair(),
    apiV3Key: randomBytes(16).toString('hex'), // 32 hex chars = 32 bytes
  }
}

/** Plaintext transaction the backend parses after decryption
 * (`models.rs::DecryptedResource`, snake_case). */
export interface WechatCallbackInput {
  /** WeChat notification id — the `payment_event` idempotency key. */
  eventId: string
  outTradeNo: string
  transactionId: string
  /** e.g. 'SUCCESS' (fulfils) or anything else (no state change). */
  tradeState: string
  /** Total in fen; MUST equal `payment_attempts.amount` or the callback is
   * rejected with 422 (amount mismatch). */
  amountTotal: number
  currency?: string
  apiV3Key: string
  /** Platform private key matching the seeded `platform_public_key` override —
   * signs the callback. */
  platformPrivateKeyPem: string
  /** Override the signing key (e.g. a rogue keypair) to force the
   * signature-rejection scenario. */
  signingPrivateKeyPem?: string
}

export interface ForgedWechatCallback {
  rawBody: Buffer
  headers: Record<string, string>
}

function encryptResource(plaintextJson: string, apiV3Key: string): {
  ciphertext: string
  associated_data: string
  nonce: string
} {
  const associatedData = 'transaction'
  // The WeChat protocol transmits the resource nonce as a PLAIN 12-character
  // string — the backend consumes it as raw UTF-8 bytes (`nonce.as_bytes()`
  // in signing.rs::decrypt_aes_gcm), NOT base64. 6 random bytes hex-encoded
  // yield exactly 12 ASCII characters.
  const nonce = randomBytes(6).toString('hex')
  const cipher = createCipheriv('aes-256-gcm', Buffer.from(apiV3Key, 'utf8'), Buffer.from(nonce, 'utf8'))
  cipher.setAAD(Buffer.from(associatedData, 'utf8'))
  const encrypted = Buffer.concat([cipher.update(plaintextJson, 'utf8'), cipher.final()])
  const tag = cipher.getAuthTag()
  return {
    ciphertext: Buffer.concat([encrypted, tag]).toString('base64'),
    associated_data: associatedData,
    nonce,
  }
}

/**
 * Build a fully signed + encrypted WeChat payment notification. The returned
 * `rawBody` Buffer must be delivered verbatim (see byte-consistency caveat).
 */
export function forgeWechatCallback(input: WechatCallbackInput): ForgedWechatCallback {
  const resource = {
    out_trade_no: input.outTradeNo,
    transaction_id: input.transactionId,
    trade_state: input.tradeState,
    amount: { total: input.amountTotal, currency: input.currency ?? 'CNY' },
    success_time: new Date().toISOString(),
  }
  const encrypted = encryptResource(JSON.stringify(resource), input.apiV3Key)
  const payload = {
    id: input.eventId,
    create_time: new Date().toISOString(),
    resource_type: 'encrypt-resource',
    event_type: 'TRANSACTION.SUCCESS',
    summary: '支付成功',
    resource: encrypted,
  }
  const rawBody = Buffer.from(JSON.stringify(payload), 'utf8')

  const timestamp = Math.floor(Date.now() / 1000).toString()
  const nonce = randomBytes(16).toString('hex')
  // Trailing newline is part of the signed message (client.rs::verify_callback).
  const message = `${timestamp}\n${nonce}\n${rawBody.toString('utf8')}\n`
  const signer = createSign('RSA-SHA256')
  signer.update(message, 'utf8')
  const signature = signer.sign(
    input.signingPrivateKeyPem ?? input.platformPrivateKeyPem,
    'base64',
  )

  return {
    rawBody,
    headers: {
      'Content-Type': 'application/json',
      'Wechatpay-Timestamp': timestamp,
      'Wechatpay-Nonce': nonce,
      'Wechatpay-Signature': signature,
      'Wechatpay-Serial': 'DEMO-PLATFORM-SERIAL',
    },
  }
}

/**
 * Deliver a forged callback. Returns the APIResponse so callers assert the
 * backend's SUCCESS (200) / FAIL (422) protocol responses.
 */
export async function deliverWechatCallback(
  request: APIRequestContext,
  realmId: string,
  forged: ForgedWechatCallback,
): Promise<APIResponse> {
  return request.post(`${BASE_URL}${WECHAT_WEBHOOK_ROUTE(realmId)}`, {
    headers: forged.headers,
    data: forged.rawBody, // Buffer — see byte-consistency caveat
    timeout: API_TIMEOUT,
  })
}

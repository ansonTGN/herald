/**
 * Shared realm-level setup for the WeChat Pay demo tests.
 *
 * One call provisions everything a wechat purchase demo needs in a realm:
 * 1. a local WeChat Pay v3 mock server (`wechat-pay-mock.ts`);
 * 2. generated key material (merchant key, platform keypair, 32-byte APIv3 key);
 * 3. the realm_config wechat rows seeded via the batch configs API
 *    (`seedWechatConfig`), pointing `base_url` at the mock and the
 *    `platform_public_key` override at the generated platform key.
 *
 * Config seeding follows the `realm-seed.ts` API precedent (requires an
 * authenticated admin API context); the admin login itself uses the LoginPage
 * POM so the browser Bearer token is captured via `getAccessToken()`.
 */

import type { Page } from '@playwright/test'
import type { LoginPage } from '../pages/login-page'
import { createBearerApiContext, REALM_ADMINS } from './auth'
import { seedWechatConfig } from '../secrets/realm-seed'
import { startWechatPayMock, type WechatPayMock } from './wechat-pay-mock'
import { generateWechatDemoKeyMaterial, type WechatDemoKeyMaterial } from './wechat-webhook-simulation'

/**
 * Identifiers of the wechat demo mappings seeded by
 * `scripts/lib/demo_seed.py::_ensure_wechat_pay_demo_data`.
 * `priceId` doubles as the purchase price-card testid suffix
 * (`purchase-price-card-${priceId}`); amounts are `provider_product_info`
 * `price` in fen and must equal the forged callback `amount.total`.
 */
export const WECHAT_DEMO = {
  /** one_time points pack (1000 fen -> 100 points). */
  PRICE_ID: 'demo-wechat-pay-points-price',
  AMOUNT_FEN: 1000,
  /** non_renewing 30-day membership (2000 fen, repeatable by design). */
  MEMBERSHIP_PRICE_ID: 'demo-wechat-pay-membership-30d',
  MEMBERSHIP_AMOUNT_FEN: 2000,
} as const

export interface WechatPayDemoEnv {
  mock: WechatPayMock
  material: WechatDemoKeyMaterial
}

export async function setupWechatPayRealm(
  page: Page,
  loginPage: LoginPage,
  realmId: string,
): Promise<WechatPayDemoEnv> {
  const mock = await startWechatPayMock()
  const material = generateWechatDemoKeyMaterial()

  const admin = REALM_ADMINS[realmId]
  if (!admin) {
    throw new Error(`[wechat-demo-setup] no seeded admin credentials for realm "${realmId}"`)
  }
  await loginPage.loginAsAdmin(admin.email, admin.password, realmId)
  const apiContext = await createBearerApiContext(loginPage.getAccessToken())
  await seedWechatConfig(apiContext, realmId, {
    appId: 'wx-demo-appid-001',
    mchId: '1900000109',
    privateKeyPem: material.merchantPrivateKeyPem,
    serialNo: 'DEMO-MCH-SERIAL-0001',
    apiV3Key: material.apiV3Key,
    notifyUrl: `https://demo.invalid/api/third/pay/${realmId}/wechat/webhooks`,
    platformPublicKeyPem: material.platform.publicKeyPem,
    baseUrl: mock.url,
  })
  await apiContext.dispose()

  return { mock, material }
}

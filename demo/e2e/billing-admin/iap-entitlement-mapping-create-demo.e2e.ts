/**
 * IAP Entitlement Mapping Create Demo (US-IAP-002)
 *
 * User story source: docs/user-stories/billing/support-iap.md
 *   - US-IAP-002 (P0): Create IAP entitlement mappings (recurring + one_time +
 *     duplicate→409 inline error).
 *
 * Contract anchors: the (provider, external_product_id) pair is unique — a
 * duplicate create returns 409. Provider display names are apple → 'App
 * Store', google → 'Google Play' (these drive the Radix select option names).
 *
 * Coverage:
 *   - S1: recurring IAP mapping (Apple App Store) — create succeeds, the form
 *     closes, new product row appears in the mapping list.
 *   - S2: one_time IAP mapping (Google Play) — create succeeds, form returns,
 *     new product row appears.
 *   - S3: duplicate provider + externalProductId → backend 409 surfaces the
 *     inline `create-mapping-submit-error` region (NOT a toast); the form
 *     REMAINS open on the failed submit.
 *
 * NOT covered (declared gap):
 *   - The 23514 / non-4xx `create_mapping_config_error` branch (DB CHECK /
 *     server defense). That is owned by backend tests; it is
 *     NOT exercised here via a crafted payload. If it surfaces incidentally
 *     during S3, the run records the observation but no assertion targets it.
 *
 * Assertion discipline:
 *   - Success assertions land on PERSISTENT state: the form page closing AND the
 *     new product row (`mapping-product-row-${externalProductId}`) becoming
 *     visible in the master list (the mutation invalidates
 *     `['entitlement-mappings']` and the list refreshes).
 *   - The duplicate-409 assertion lands on the inline `create-mapping-submit-
 *     error` region — toasts (sonner) are NEVER the primary assertion (auto-
 *     dismissed, locale/volatile).
 *
 * Fixture discipline:
 *   - Unified fixture (`test` / `expect` / `cleanupTestData` from
 *     demo-page.fixtures). `demoLogger` is auto-finalized inside the fixture —
 *     tests MUST NOT call `logger.finalize()` manually.
 *   - `beforeEach`: login admin realm + `verifyTestEnvironment`.
 *   - `afterEach`: `cleanupTestData(page, 'admin', { timestamp })`.
 *   - `beforeAll`: ensures an Apple AND a Google IAP provider are configured
 *     (reuse `PaymentProvidersPage.configureIapProvider` — idempotent; the edit
 *     branch re-saves when already configured). DE-D02 does NOT rely on DE-D01
 *     test state; it provisions its own providers.
 *
 * Bucket dependency:
 *   - The seeded default bucket in the admin realm is the registration pool
 *     'Primary Pool' (display name) — `scripts/lib/demo_seed.py::
 *     CREDIT_BUCKET_NAME_PRIMARY`, mirrored in
 *     `helpers/bucket-seed-ids.ts::CREDIT_BUCKET_NAMES.PRIMARY_POOL`. Resolved
 *     by DISPLAY NAME (bucket ids are dynamic UUIDs). If the seed bucket display
 *     name is not stable across envs, pass NO bucketName and the helper falls
 *     back to selecting the first bucket option (recorded gap).
 *
 * Runner (DE-D03 executes; this item is compile-only):
 *   uv run scripts/demo-test-runner.py \
 *     "demo/e2e/billing-admin/iap-entitlement-mapping-create-demo.e2e.ts" \
 *     --run-id <RUN_ID> --grep "<test title>" --no-ngrok
 *   (--no-ngrok: IAP create is management-path only, no webhook dependency.)
 */

import { test, cleanupTestData, expect } from '../fixtures/demo-page.fixtures'
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { loginAsAdmin, DEMO_ADMIN } from '../helpers/auth'
import { PaymentProvidersPage } from '../pages/payment-providers-page'
import { EntitlementMappingsPage } from '../pages/entitlement-mappings-page'
import { CREDIT_BUCKET_NAMES } from '../helpers/bucket-seed-ids'
import { SELECTORS } from '../selectors'

/**
 * The seeded registration-pool bucket display name in the admin realm.
 *
 * Source: `scripts/lib/demo_seed.py::CREDIT_BUCKET_NAME_PRIMARY` ('Primary
 * Pool'). If this drifts, the `fillCreateMappingForm` helper falls back to the
 * first bucket option (bucketName omitted) — see the EntitlementMappingsPage
 * `selectCreateMappingBucket` fallback + the gap note above.
 */
const SEED_BUCKET_NAME = CREDIT_BUCKET_NAMES.PRIMARY_POOL

test.describe('[Billing Admin] IAP entitlement mapping 创建 (US-IAP-002)', () => {
  let testStartTime: number
  let mappingsPage: EntitlementMappingsPage

  test.beforeAll(async () => {
    // Provision BOTH providers in a throwaway browser context so the per-test
    // admin login still carries a clean session. `configureIapProvider` is
    // idempotent (configured → edit branch re-save; unconfigured → create).
    const { chromium } = await import('@playwright/test')
    const browser = await chromium.launch()
    const context = await browser.newContext()
    const page = await context.newPage()
    try {
      await loginAsAdmin(page, { realmId: DEMO_ADMIN.realmId })
      const providersPage = new PaymentProvidersPage(page)

      // Apple — production-shaped credentials (no live App Store API calls on
      // the management path; the backend stores the config).
      await providersPage.goto(DEMO_ADMIN.realmId)
      await providersPage.configureIapProvider('apple', {
        bundleId: 'com.cas.iap.demo',
        issuerId: 'iap-apple-issuer-demo',
        keyId: 'IAPDEMOKEY1',
        privateKeyP8: [
          '-----BEGIN PRIVATE KEY-----',
          'MIGTAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBHkwdwIBAQQgiapdemoiapdemoiapdem',
          'Pa5eiTcXkDTUkJiumGmfvNq-1h8WodLJvL3ObG02Il2CgYIKoZIzj0DAQehRANCAAS',
          '-----END PRIVATE KEY-----',
        ].join('\n'),
        environment: 'production',
      })

      // Google — structure-shaped service-account JSON (no live API calls).
      await providersPage.goto(DEMO_ADMIN.realmId)
      await providersPage.configureIapProvider('google', {
        packageName: 'com.cas.iap.demo',
        serviceAccountJson: JSON.stringify({
          type: 'service_account',
          project_id: 'cas-iap-demo',
          private_key_id: 'iapgooglekeydemo',
          private_key:
            '-----BEGIN PRIVATE KEY-----\nMIIBdemo\n-----END PRIVATE KEY-----\n',
          client_email: 'iap-demo@cas-iap-demo.iam.gserviceaccount.com',
          client_id: 'iap-demo-client',
          auth_uri: 'https://accounts.google.com/o/oauth2/auth',
          token_uri: 'https://oauth2.googleapis.com/token',
        }),
      })
    } finally {
      await context.close()
      await browser.close()
    }
  })

  test.beforeEach(async ({ page, demoLogger }) => {
    testStartTime = Date.now()
    await verifyTestEnvironment(page, {
      requiredRealms: [DEMO_ADMIN.realmId],
      requiredUsers: [DEMO_ADMIN.email],
    })
    await loginAsAdmin(page, { realmId: DEMO_ADMIN.realmId })
    mappingsPage = new EntitlementMappingsPage(page, demoLogger)
    await mappingsPage.goto(DEMO_ADMIN.realmId)
    await mappingsPage.waitForDataLoaded()
    await demoLogger.testCode.log('On entitlement-mappings page (admin persona)')
  })

  test.afterEach(async ({ page, demoLogger }) => {
    await cleanupTestData(page, DEMO_ADMIN.realmId, {
      keepUsers: [DEMO_ADMIN.email],
      timestamp: testStartTime,
    })
    await demoLogger.testCode.log('Test data cleaned up')
  })

  // ==========================================================================
  // S1: recurring IAP mapping (Apple App Store)
  // US-IAP-002 S1 — create succeeds, page returns to the list, product row appears.
  // ==========================================================================

  test('[US-IAP-002 S1] recurring IAP mapping (Apple App Store)', async ({
    page,
    demoLogger,
  }) => {
    const runId = Date.now()
    const externalProductId = `com.example.pro.monthly.${runId}`
    const entitlementKey = `iap-pro-${runId}`

    await test.step('When: fill create-mapping form (recurring apple)', async () => {
      await mappingsPage.fillCreateMappingForm({
        provider: 'apple',
        externalProductId,
        entitlementKey,
        bucketName: SEED_BUCKET_NAME,
        billingType: 'recurring',
        billingPeriod: 'monthly',
        pointRuleTriggers: ['subscription_initial', 'subscription_renewal'],
        pointsAmount: 100,
      })
      await demoLogger.testCode.log('Create-mapping form filled (recurring apple)')
    })

    await test.step('Then: submit returns to the list', async () => {
      await mappingsPage.submitCreateMapping()
      await mappingsPage.expectCreateMappingFormClosed()
      await demoLogger.testCode.log('Create-mapping page left after submit')
    })

    await test.step('Then: new product row appears in the mapping list', async () => {
      // Persistent-state assertion: the master list refreshes (the create
      // mutation invalidates ['entitlement-mappings']) and the row keyed by
      // externalProductId becomes visible. Assert on the list, NOT a toast.
      const row = page.locator(
        SELECTORS.multiPriceMapping.mappingProductRow(externalProductId),
      )
      await expect(row).toBeVisible({ timeout: 15000 })
      await demoLogger.testCode.log(
        `New mapping product row visible: ${externalProductId}`,
      )
    })
  })

  // ==========================================================================
  // S2: one_time IAP mapping (Google Play)
  // US-IAP-002 S2 — create succeeds, page returns to the list, product row appears.
  // (No billing-period field for one_time; validity-days only renders for
  //  one_time + canManagePoints — admin has points.manage.)
  // ==========================================================================

  test('[US-IAP-002 S2] one_time IAP mapping (Google Play)', async ({
    page,
    demoLogger,
  }) => {
    const runId = Date.now()
    const externalProductId = `com.example.pack.${runId}`
    const entitlementKey = `iap-pack-${runId}`

    await test.step('When: fill create-mapping form (one_time google)', async () => {
      await mappingsPage.fillCreateMappingForm({
        provider: 'google',
        externalProductId,
        entitlementKey,
        bucketName: SEED_BUCKET_NAME,
        billingType: 'one_time',
        pointRuleTriggers: ['topup'],
        pointsAmount: 1000,
        validityDays: 30,
      })
      await demoLogger.testCode.log('Create-mapping form filled (one_time google)')
    })

    await test.step('Then: submit returns to the list', async () => {
      await mappingsPage.submitCreateMapping()
      await mappingsPage.expectCreateMappingFormClosed()
      await demoLogger.testCode.log('Create-mapping page left after submit')
    })

    await test.step('Then: new product row appears in the mapping list', async () => {
      const row = page.locator(
        SELECTORS.multiPriceMapping.mappingProductRow(externalProductId),
      )
      await expect(row).toBeVisible({ timeout: 15000 })
      await demoLogger.testCode.log(
        `New mapping product row visible: ${externalProductId}`,
      )
    })
  })

  // ==========================================================================
  // S3: duplicate provider + externalProductId → 409 inline error
  // US-IAP-002 S3 — self-contained: create once, then attempt the same
  // provider + externalProductId again (with a fresh entitlementKey, since the
  // unique constraint is on provider + external_product_id).
  // The backend 409 surfaces the inline `create-mapping-submit-error` region
  // (NOT a toast); the form REMAINS on the page on the failed submit.
  // ==========================================================================

  test('[US-IAP-002 S3] duplicate provider+product id → 409 inline error', async ({
    page,
    demoLogger,
  }) => {
    const runId = Date.now()
    const externalProductId = `com.example.dup.${runId}`

    await test.step('Given: create the first mapping (apple recurring)', async () => {
      await mappingsPage.fillCreateMappingForm({
        provider: 'apple',
        externalProductId,
        entitlementKey: `iap-dup-first-${runId}`,
        bucketName: SEED_BUCKET_NAME,
        billingType: 'recurring',
        billingPeriod: 'monthly',
        pointRuleTriggers: ['subscription_initial', 'subscription_renewal'],
        pointsAmount: 50,
      })
      await mappingsPage.submitCreateMapping()
      await mappingsPage.expectCreateMappingFormClosed()
      await demoLogger.testCode.log('First (seed) mapping created for duplicate test')
    })

    await test.step('When: re-create with the SAME provider + externalProductId', async () => {
      // Fresh entitlementKey — the unique constraint is on provider +
      // external_product_id, NOT entitlement_key.
      await mappingsPage.fillCreateMappingForm({
        provider: 'apple',
        externalProductId,
        entitlementKey: `iap-dup-second-${runId}`,
        bucketName: SEED_BUCKET_NAME,
        billingType: 'recurring',
        billingPeriod: 'monthly',
        pointRuleTriggers: ['subscription_initial', 'subscription_renewal'],
        pointsAmount: 50,
      })
      await mappingsPage.submitCreateMapping()
      await demoLogger.testCode.log('Duplicate create submitted')
    })

    await test.step('Then: 409 inline error visible + form stays on the page', async () => {
      // Assert on the inline error region — NOT a toast. The form remains on the page
      // on the failed submit (no navigation, no dismiss).
      await mappingsPage.expectCreateMappingDuplicateError()
      await mappingsPage.expectCreateMappingFormOpen()
      await demoLogger.testCode.log(
        '409 duplicate inline error surfaced; form stayed on the page',
      )
    })
  })
})

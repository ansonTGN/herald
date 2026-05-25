import { describe, it, expect } from 'vitest'
import {
  shopifyConfigSchema,
  claimSubscriptionSchema,
  wechatConfigSchema,
  getShopifyConfigDefaults,
  getClaimSubscriptionDefaults,
  getWechatConfigDefaults,
} from '../billing-forms'

describe('shopifyConfigSchema', () => {
  describe('business validation', () => {
    it('should accept valid complete config', () => {
      const result = shopifyConfigSchema.safeParse({
        shopDomain: 'demo-store.myshopify.com',
        adminAccessToken: 'shpat_xxx',
        storefrontAccessToken: 'shp_xxx',
        appClientSecret: 'secret_long_enough',
      })

      expect(result.success).toBe(true)
    })

    it('should enforce .myshopify.com domain suffix', () => {
      const result = shopifyConfigSchema.safeParse({
        shopDomain: 'example.com',
        adminAccessToken: 'shpat_xxx',
        storefrontAccessToken: 'shp_xxx',
        appClientSecret: 'secret_long_enough',
      })

      expect(result.success).toBe(false)
    })

    it('should enforce required token prefixes', () => {
      const result = shopifyConfigSchema.safeParse({
        shopDomain: 'demo-store.myshopify.com',
        adminAccessToken: 'invalid_token',
        storefrontAccessToken: 'shp_xxx',
        appClientSecret: 'secret_long_enough',
      })

      expect(result.success).toBe(false)
    })

    it('should apply default values for optional fields', () => {
      const result = shopifyConfigSchema.safeParse({
        shopDomain: 'demo-store.myshopify.com',
        adminAccessToken: 'shpat_xxx',
        storefrontAccessToken: 'shp_xxx',
        appClientSecret: 'secret_long_enough',
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.apiVersion).toBe('2024-01')
        expect(result.data.webhookSubscriptionMode).toBe('admin_api')
        expect(result.data.timeout).toBe(30)
      }
    })
  })
})

describe('claimSubscriptionSchema', () => {
  describe('conditional validation', () => {
    it('should require either customer ID or contract ID', () => {
      const result = claimSubscriptionSchema.safeParse({
        shopifyCustomerId: '',
        contractId: '',
      })

      expect(result.success).toBe(false)
    })

    it('should accept when only customer ID is provided', () => {
      const result = claimSubscriptionSchema.safeParse({
        shopifyCustomerId: 'customer_123',
        contractId: '',
      })

      expect(result.success).toBe(true)
    })

    it('should accept when only contract ID is provided', () => {
      const result = claimSubscriptionSchema.safeParse({
        shopifyCustomerId: '',
        contractId: 'gid://shopify/SubscriptionContract/123',
      })

      expect(result.success).toBe(true)
    })

    it('should accept when both IDs are provided', () => {
      const result = claimSubscriptionSchema.safeParse({
        shopifyCustomerId: 'customer_123',
        contractId: 'gid://shopify/SubscriptionContract/123',
      })

      expect(result.success).toBe(true)
    })

    it('should default grantCurrentPeriod to true', () => {
      const result = claimSubscriptionSchema.safeParse({
        shopifyCustomerId: 'customer_123',
      })

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.grantCurrentPeriod).toBe(true)
      }
    })
  })
})

describe('wechatConfigSchema', () => {
  describe('business validation', () => {
    it('should accept valid complete config', () => {
      const result = wechatConfigSchema.safeParse({
        appId: 'wx1234567890abcdef',
        mchId: '1234567890',
        privateKey:
          '-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQ\n-----END PRIVATE KEY-----',
        serialNo: '1A2B3C4D5E6F',
        v3Key: '0123456789abcdefghijklmnopqrstuv',
        platformPublicKey: '',
        notifyUrl: 'https://example.com/api/webhook',
      })

      expect(result.success).toBe(true)
    })

    it('should enforce wx prefix for App ID', () => {
      const result = wechatConfigSchema.safeParse({
        appId: 'abc123',
        mchId: '1234567890',
        privateKey:
          '-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQ\n-----END PRIVATE KEY-----',
        serialNo: '1A2B3C4D5E6F',
        v3Key: '0123456789abcdefghijklmnopqrstuv',
        notifyUrl: 'https://example.com/api/webhook',
      })

      expect(result.success).toBe(false)
    })

    it('should enforce numeric Merchant ID', () => {
      const result = wechatConfigSchema.safeParse({
        appId: 'wx1234567890abcdef',
        mchId: 'abc123',
        privateKey:
          '-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQ\n-----END PRIVATE KEY-----',
        serialNo: '1A2B3C4D5E6F',
        v3Key: '0123456789abcdefghijklmnopqrstuv',
        notifyUrl: 'https://example.com/api/webhook',
      })

      expect(result.success).toBe(false)
    })

    it('should enforce PEM format for private key', () => {
      const result = wechatConfigSchema.safeParse({
        appId: 'wx1234567890abcdef',
        mchId: '1234567890',
        privateKey: 'just_a_random_string',
        serialNo: '1A2B3C4D5E6F',
        v3Key: '0123456789abcdefghijklmnopqrstuv',
        notifyUrl: 'https://example.com/api/webhook',
      })

      expect(result.success).toBe(false)
    })

    it('should enforce 32-byte v3Key', () => {
      const result = wechatConfigSchema.safeParse({
        appId: 'wx1234567890abcdef',
        mchId: '1234567890',
        privateKey:
          '-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQ\n-----END PRIVATE KEY-----',
        serialNo: '1A2B3C4D5E6F',
        v3Key: 'short_key',
        notifyUrl: 'https://example.com/api/webhook',
      })

      expect(result.success).toBe(false)
    })

    it('should enforce HTTPS for notify URL', () => {
      const result = wechatConfigSchema.safeParse({
        appId: 'wx1234567890abcdef',
        mchId: '1234567890',
        privateKey:
          '-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQ\n-----END PRIVATE KEY-----',
        serialNo: '1A2B3C4D5E6F',
        v3Key: '0123456789abcdefghijklmnopqrstuv',
        notifyUrl: 'http://example.com/api/webhook',
      })

      expect(result.success).toBe(false)
    })
  })
})

describe('Default values functions', () => {
  it('should return Shopify config defaults', () => {
    const defaults = getShopifyConfigDefaults()

    expect(defaults.shopDomain).toBe('')
    expect(defaults.adminAccessToken).toBe('')
    expect(defaults.storefrontAccessToken).toBe('')
    expect(defaults.appClientSecret).toBe('')
    expect(defaults.apiVersion).toBe('2024-01')
    expect(defaults.webhookSubscriptionMode).toBe('admin_api')
    expect(defaults.timeout).toBe(30)
  })

  it('should merge partial Shopify config with defaults', () => {
    const partialConfig = {
      shopDomain: 'test-store.myshopify.com',
      apiVersion: '2024-04',
    }

    const defaults = getShopifyConfigDefaults(partialConfig)

    expect(defaults.shopDomain).toBe('test-store.myshopify.com')
    expect(defaults.apiVersion).toBe('2024-04')
    expect(defaults.timeout).toBe(30)
  })

  it('should return claim subscription defaults', () => {
    const defaults = getClaimSubscriptionDefaults()

    expect(defaults.shopifyCustomerId).toBe('')
    expect(defaults.contractId).toBe('')
    expect(defaults.grantCurrentPeriod).toBe(true)
  })

  it('should return WeChat config defaults', () => {
    const defaults = getWechatConfigDefaults()

    expect(defaults.appId).toBe('')
    expect(defaults.mchId).toBe('')
    expect(defaults.privateKey).toBe('')
    expect(defaults.serialNo).toBe('')
    expect(defaults.v3Key).toBe('')
    expect(defaults.notifyUrl).toBe('')
  })
})

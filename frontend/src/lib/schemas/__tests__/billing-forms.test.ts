import { describe, it, expect } from 'vitest'
import { wechatConfigSchema, getWechatConfigDefaults } from '../billing-forms'

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

    it('should enforce numeric Merchant id', () => {
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

import { describe, it, expect } from 'vitest'
import { formatProviderName } from '../format-provider-name'

describe('formatProviderName', () => {
  describe('known provider names', () => {
    it('GIVEN wechat provider WHEN formatted THEN returns WeChat Pay', () => {
      expect(formatProviderName('wechat')).toBe('WeChat Pay')
    })

    it('GIVEN stripe provider WHEN formatted THEN returns Stripe', () => {
      expect(formatProviderName('stripe')).toBe('Stripe')
    })

    it('GIVEN shopify provider WHEN formatted THEN returns Shopify', () => {
      expect(formatProviderName('shopify')).toBe('Shopify')
    })

    it('GIVEN creem provider WHEN formatted THEN returns Creem', () => {
      expect(formatProviderName('creem')).toBe('Creem')
    })
  })

  describe('unknown provider names', () => {
    it('GIVEN unknown provider WHEN formatted THEN capitalizes input', () => {
      expect(formatProviderName('unknown')).toBe('Unknown')
    })

    it('GIVEN provider with underscores WHEN formatted THEN capitalizes correctly', () => {
      expect(formatProviderName('my_custom_provider')).toBe('My_custom_provider')
    })

    it('GIVEN provider with hyphens WHEN formatted THEN capitalizes correctly', () => {
      expect(formatProviderName('my-custom-provider')).toBe('My-custom-provider')
    })

    it('GIVEN empty string WHEN formatted THEN returns empty string', () => {
      expect(formatProviderName('')).toBe('')
    })
  })

  describe('case sensitivity', () => {
    it('GIVEN WECHAT in uppercase WHEN formatted THEN returns WeChat Pay', () => {
      expect(formatProviderName('WECHAT')).toBe('WECHAT')
    })

    it('GIVEN WeChat in mixed case WHEN formatted THEN returns WeChat', () => {
      expect(formatProviderName('WeChat')).toBe('WeChat')
    })
  })
})

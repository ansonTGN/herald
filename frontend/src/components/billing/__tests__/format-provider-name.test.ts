import { describe, it, expect } from 'vitest'
import { formatProviderName } from '../format-provider-name'

describe('formatProviderName', () => {
  describe('known provider names', () => {
    it('GIVEN wechat provider WHEN formatted THEN returns WeChat Pay', () => {
      expect(formatProviderName('wechat')).toBe('WeChat Pay')
    })
  })

  describe('edge cases', () => {
    it('GIVEN empty string WHEN formatted THEN returns empty string', () => {
      expect(formatProviderName('')).toBe('')
    })
  })
})
